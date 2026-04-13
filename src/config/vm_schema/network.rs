use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Unique identifier for the network device
    pub id: String,

    /// Network model (virtio-net, e1000, etc.)
    pub model: String,

    /// Network mode (user, bridge, socket)
    pub mode: String,

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

impl From<NetworkConfig> for QemuArgs {
    fn from(network: NetworkConfig) -> Self {
        let mut args = QemuArgs::new();

        args.push_str("-netdev");
        let netdev_spec = match network.mode.as_str() {
            "user" => format!("type=user,id={}", network.id),
            _ => format!("type={},id={}", network.mode, network.id),
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
