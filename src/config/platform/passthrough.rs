use serde::{Deserialize, Serialize};

/// Hardware passthrough configuration for PCI devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostPciConfig {
    /// PCI device address (e.g., "0000:03:00.0")
    pub device: String,

    /// Unique identifier for the device
    pub id: String,

    /// PCIe configuration
    #[serde(default)]
    pub pcie: bool,

    /// VGA passthrough (for GPU devices)
    #[serde(default)]
    pub x_vga: bool,

    /// Optional guest bus placement for the passthrough device
    pub bus: Option<String>,

    /// Optional guest slot/function address for the passthrough device
    pub addr: Option<String>,

    /// Enable multifunction on the guest slot when grouping related functions
    #[serde(default)]
    pub multifunction: bool,

    /// ROM file path (optional)
    pub romfile: Option<String>,
}

/// USB device passthrough configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceConfig {
    /// Unique identifier for the USB device
    pub id: String,

    /// USB device specification
    #[serde(default)]
    pub host: String,

    /// USB host bus number for Proxmox-style addressing
    pub hostbus: Option<String>,

    /// USB host port path for Proxmox-style addressing
    pub hostport: Option<String>,

    /// USB controller bus (optional)
    pub bus: Option<String>,

    /// USB controller port (optional)
    pub port: Option<String>,
}

/// XHCI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhciControllerConfig {
    /// Unique identifier for the controller
    pub id: String,

    /// Number of USB2 ports
    pub p2: Option<u8>,

    /// Number of USB3 ports
    pub p3: Option<u8>,

    /// Parent bus placement
    pub bus: Option<String>,

    /// Address on the selected bus
    pub addr: Option<String>,
}
