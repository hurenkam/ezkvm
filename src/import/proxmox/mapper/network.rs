use super::helpers::is_enabled;
use super::super::model::ProxmoxNetEntry;
use super::{MappingWarning, NetworkBackendConfig, NetworkConfig};
use std::collections::BTreeMap;

pub(super) fn map_network(
    network: &ProxmoxNetEntry,
    vmid: Option<u32>,
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

    let has_bridge = network.options.contains_key("bridge");
    let backend_type = if has_bridge {
        "tap".to_string()
    } else {
        "user".to_string()
    };

    let ifname = network.options.get("ifname").cloned().or_else(|| {
        if has_bridge {
            vmid.map(|id| format!("tap{}i{}", id, network.index))
        } else {
            None
        }
    });
    let script = if has_bridge {
        network
            .options
            .get("script")
            .cloned()
            .or_else(|| Some("/usr/libexec/qemu-server/pve-bridge".to_string()))
    } else {
        network.options.get("script").cloned()
    };
    let downscript = if has_bridge {
        network
            .options
            .get("downscript")
            .cloned()
            .or_else(|| Some("/usr/libexec/qemu-server/pve-bridgedown".to_string()))
    } else {
        network.options.get("downscript").cloned()
    };
    let vhost = network
        .options
        .get("vhost")
        .map(|value| is_enabled(Some(value)))
        .or(if has_bridge { Some(true) } else { None });

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
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| {
            if is_windows && model == "virtio-net-pci" {
                Some(1024)
            } else {
                None
            }
        });

    let tx_queue_size = network
        .options
        .get("tx_queue_size")
        .or_else(|| network.options.get("txqueuesz"))
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| {
            if is_windows && model == "virtio-net-pci" {
                Some(256)
            } else {
                None
            }
        });

    let bus = network.options.get("bus").cloned().or_else(|| {
        if is_windows && model == "virtio-net-pci" {
            Some("pci.0".to_string())
        } else {
            None
        }
    });
    let addr = network.options.get("addr").cloned().or_else(|| {
        if is_windows && model == "virtio-net-pci" {
            Some(format!("0x{:x}", 0x12 + network.index as u32))
        } else {
            None
        }
    });

    NetworkConfig {
        id: network.key.clone(),
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
