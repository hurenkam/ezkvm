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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ifname: Option<String>,

    /// Host bridge name for bridge-style backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<String>,

    /// Optional helper script path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,

    /// Optional teardown helper script path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downscript: Option<String>,

    /// Optional bridge helper path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helper: Option<String>,

    /// Enable or disable vhost acceleration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vhost: Option<bool>,

    /// Optional queue count for supported backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queues: Option<u16>,

    /// User-mode host forwarding rules in native QEMU `hostfwd=` syntax.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostfwd: Vec<String>,

    /// Listen address or path for socket-like backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen: Option<String>,

    /// Connect address or path for socket-like backends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect: Option<String>,

    /// File descriptor reference for fd-backed netdevs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fd: Option<String>,

    /// Additional backend options preserved for compatibility.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, String>,
}

impl NetworkBackendConfig {
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

    /// Structured network backend configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<NetworkBackendConfig>,

    /// MAC address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,

    /// RX queue size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_queue_size: Option<u32>,

    /// TX queue size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_queue_size: Option<u32>,

    /// Boot index for firmware boot ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_index: Option<u32>,

    /// PCI/PCIe bus placement for the network device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Slot or function address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
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
        match &self.backend {
            Some(backend) => Ok(backend.clone()),
            None => Err(anyhow!("Network '{}' must define 'backend'", self.id)),
        }
    }
}

impl From<NetworkConfig> for QemuArgs {
    fn from(network: NetworkConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-netdev");
        let netdev_spec = network
            .resolved_backend()
            .map(|backend| backend.to_netdev_spec(&network.id))
            .unwrap_or_else(|_| format!("type=user,id={}", network.id));
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
