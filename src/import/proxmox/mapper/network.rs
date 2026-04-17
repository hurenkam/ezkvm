use super::super::model::ProxmoxNetEntry;
use super::helpers::is_enabled;
use super::{MappingWarning, NetworkBackendConfig, NetworkConfig};
use std::collections::BTreeMap;

pub(super) fn map_network(
    network: &ProxmoxNetEntry,
    _vmid: Option<u32>,
    is_windows: bool,
    warnings: &mut Vec<MappingWarning>,
) -> NetworkConfig {
    let model = match network.model.as_str() {
        "virtio" => {
            if is_windows {
                "virtio-net-pci".to_string()
            } else {
                "virtio-net".to_string()
            }
        }
        "virtio-net" => {
            if is_windows {
                "virtio-net-pci".to_string()
            } else {
                "virtio-net".to_string()
            }
        }
        "virtio-net-pci" | "e1000" | "e1000e" | "rtl8139" => network.model.clone(),
        other => {
            warnings.push(MappingWarning {
                source_field: network.key.clone(),
                message: format!(
                    "unsupported network model '{}' mapped to 'virtio-net'",
                    other
                ),
            });
            "virtio-net".to_string()
        }
    };

    // Proxmox uses a tap backend with a bridge script when a bridge is configured.
    // The QEMU `bridge` backend helper is a different mechanism; map to `tap` instead
    // so that the proxmox-base profile can supply the Proxmox bridge scripts.
    let has_bridge = network.options.contains_key("bridge");
    let backend_type = if has_bridge {
        "tap".to_string()
    } else {
        "user".to_string()
    };

    let ifname = network.options.get("ifname").cloned();
    let script = network.options.get("script").cloned();
    let downscript = network.options.get("downscript").cloned();
    let vhost = network
        .options
        .get("vhost")
        .map(|value| is_enabled(Some(value)));

    let backend = NetworkBackendConfig {
        backend_type,
        ifname,
        bridge: network.options.get("bridge").cloned(),
        script,
        downscript,
        helper: None,
        vhost,
        queues: network
            .options
            .get("queues")
            .and_then(|v| v.parse::<u16>().ok()),
        hostfwd: Vec::new(),
        listen: None,
        connect: None,
        fd: None,
        extra: BTreeMap::new(),
    };

    let rx_queue_size = network
        .options
        .get("rx_queue_size")
        .or_else(|| network.options.get("rxqueuesz"))
        .and_then(|v| v.parse::<u32>().ok());

    let tx_queue_size = network
        .options
        .get("tx_queue_size")
        .or_else(|| network.options.get("txqueuesz"))
        .and_then(|v| v.parse::<u32>().ok());

    let bus = network.options.get("bus").cloned();
    let addr = network.options.get("addr").cloned();

    NetworkConfig {
        id: String::new(),
        model,
        backend: Some(backend),
        mac: network.mac.clone(),
        rx_queue_size,
        tx_queue_size,
        boot_index: None,
        bus,
        addr,
    }
}
