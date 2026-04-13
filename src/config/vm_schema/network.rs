use crate::qemu::types::QemuArgs;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NetworkBackendConfig {
    /// QEMU netdev backend type such as `user`, `tap`, `bridge`, or `socket`.
    #[serde(rename = "type")]
    pub backend_type: String,

    /// Host-side interface name for tap backends.
    #[serde(default)]
    pub ifname: Option<String>,

    /// Host bridge name for bridge-style backends.
    #[serde(default)]
    pub bridge: Option<String>,

    /// Optional helper script path.
    #[serde(default)]
    pub script: Option<String>,

    /// Optional teardown helper script path.
    #[serde(default)]
    pub downscript: Option<String>,

    /// Optional bridge helper path.
    #[serde(default)]
    pub helper: Option<String>,

    /// Enable or disable vhost acceleration.
    #[serde(default)]
    pub vhost: Option<bool>,

    /// Optional queue count for supported backends.
    #[serde(default)]
    pub queues: Option<u16>,

    /// User-mode host forwarding rules in native QEMU `hostfwd=` syntax.
    #[serde(default)]
    pub hostfwd: Vec<String>,

    /// Listen address or path for socket-like backends.
    #[serde(default)]
    pub listen: Option<String>,

    /// Connect address or path for socket-like backends.
    #[serde(default)]
    pub connect: Option<String>,

    /// File descriptor reference for fd-backed netdevs.
    #[serde(default)]
    pub fd: Option<String>,

    /// Additional backend options preserved for compatibility.
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

impl NetworkBackendConfig {
    pub fn from_legacy_mode(mode: &str) -> Result<Self> {
        let mut parts = mode.split(',');
        let Some(backend_type) = parts.next().map(str::trim).filter(|part| !part.is_empty()) else {
            return Err(anyhow!("Network mode cannot be empty"));
        };

        let mut backend = NetworkBackendConfig {
            backend_type: backend_type.to_string(),
            ..Default::default()
        };

        for segment in parts {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                backend.extra.insert(trimmed.to_string(), String::new());
                continue;
            };

            match key {
                "ifname" => backend.ifname = Some(value.to_string()),
                "bridge" | "br" => backend.bridge = Some(value.to_string()),
                "script" => backend.script = Some(value.to_string()),
                "downscript" => backend.downscript = Some(value.to_string()),
                "helper" => backend.helper = Some(value.to_string()),
                "listen" => backend.listen = Some(value.to_string()),
                "connect" => backend.connect = Some(value.to_string()),
                "fd" => backend.fd = Some(value.to_string()),
                "queues" => {
                    backend.queues = Some(value.parse().map_err(|_| {
                        anyhow!("Invalid network queues value '{}': expected integer", value)
                    })?)
                }
                "vhost" => {
                    backend.vhost = Some(parse_on_off_bool("vhost", value)?);
                }
                "hostfwd" => backend.hostfwd.push(value.to_string()),
                _ => {
                    backend.extra.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(backend)
    }

    pub fn to_netdev_spec(&self, id: &str) -> String {
        let mut parts = vec![format!("type={}", self.backend_type), format!("id={}", id)];

        if let Some(ifname) = &self.ifname {
            parts.push(format!("ifname={}", ifname));
        }
        if let Some(bridge) = &self.bridge {
            let key = if self.backend_type == "bridge" {
                "br"
            } else {
                "bridge"
            };
            parts.push(format!("{}={}", key, bridge));
        }
        if let Some(script) = &self.script {
            parts.push(format!("script={}", script));
        }
        if let Some(downscript) = &self.downscript {
            parts.push(format!("downscript={}", downscript));
        }
        if let Some(helper) = &self.helper {
            parts.push(format!("helper={}", helper));
        }
        if let Some(vhost) = self.vhost {
            parts.push(format!("vhost={}", if vhost { "on" } else { "off" }));
        }
        if let Some(queues) = self.queues {
            parts.push(format!("queues={}", queues));
        }
        for hostfwd in &self.hostfwd {
            parts.push(format!("hostfwd={}", hostfwd));
        }
        if let Some(listen) = &self.listen {
            parts.push(format!("listen={}", listen));
        }
        if let Some(connect) = &self.connect {
            parts.push(format!("connect={}", connect));
        }
        if let Some(fd) = &self.fd {
            parts.push(format!("fd={}", fd));
        }
        for (key, value) in &self.extra {
            if value.is_empty() {
                parts.push(key.clone());
            } else {
                parts.push(format!("{}={}", key, value));
            }
        }

        parts.join(",")
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Unique identifier for the network device
    #[serde(default)]
    pub id: String,

    /// Network model (virtio-net, e1000, etc.)
    pub model: String,

    /// Legacy network backend string.
    /// Prefer `backend` for new configurations.
    #[serde(default)]
    pub mode: String,

    /// Structured network backend configuration.
    #[serde(default)]
    pub backend: Option<NetworkBackendConfig>,

    /// MAC address
    pub mac: Option<String>,

    /// RX queue size
    pub rx_queue_size: Option<u32>,

    /// TX queue size
    pub tx_queue_size: Option<u32>,

    /// Boot index for firmware boot ordering
    pub boot_index: Option<u32>,

    /// PCI/PCIe bus placement for the network device
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    pub addr: Option<String>,
}

impl NetworkConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = format!("net{}", candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }

    pub fn resolved_backend(&self) -> Result<NetworkBackendConfig> {
        let has_mode = !self.mode.trim().is_empty();

        match (&self.backend, has_mode) {
            (Some(_), true) => Err(anyhow!(
                "Network '{}' cannot specify both 'backend' and legacy 'mode'",
                self.id
            )),
            (Some(backend), false) => Ok(backend.clone()),
            (None, true) => NetworkBackendConfig::from_legacy_mode(&self.mode),
            (None, false) => Err(anyhow!(
                "Network '{}' must define either 'backend' or legacy 'mode'",
                self.id
            )),
        }
    }
}

impl From<NetworkConfig> for QemuArgs {
    fn from(network: NetworkConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-netdev");
        let netdev_spec = match network.backend.as_ref() {
            Some(backend) => backend.to_netdev_spec(&network.id),
            None => match network.mode.as_str() {
                "user" => format!("type=user,id={}", network.id),
                _ => format!("type={},id={}", network.mode, network.id),
            },
        };
        args.push(netdev_spec);

        args.push_str("-device");
        let mut device_spec = format!("{},netdev={}", network.model, network.id);

        if let Some(mac) = network.mac {
            device_spec.push_str(&format!(",mac={}", mac));
        }

        if let Some(bus) = network.bus {
            device_spec.push_str(&format!(",bus={}", bus));
        }

        if let Some(addr) = network.addr {
            device_spec.push_str(&format!(",addr={}", addr));
        }

        if let Some(rx_queue_size) = network.rx_queue_size {
            device_spec.push_str(&format!(",rx_queue_size={}", rx_queue_size));
        }

        if let Some(tx_queue_size) = network.tx_queue_size {
            device_spec.push_str(&format!(",tx_queue_size={}", tx_queue_size));
        }

        if let Some(boot_index) = network.boot_index {
            device_spec.push_str(&format!(",bootindex={}", boot_index));
        }

        args.push(device_spec);
        args
    }
}

fn parse_on_off_bool(field_name: &str, value: &str) -> Result<bool> {
    match value {
        "on" | "true" => Ok(true),
        "off" | "false" => Ok(false),
        _ => Err(anyhow!(
            "Invalid value '{}' for {}: expected on/off or true/false",
            value,
            field_name
        )),
    }
}
