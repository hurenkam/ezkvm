use serde::{Deserialize, Serialize};

fn is_false(value: &bool) -> bool {
    !*value
}

/// Hardware passthrough configuration for PCI devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostPciConfig {
    /// PCI device address (e.g., "0000:03:00.0")
    pub device: String,

    /// Unique identifier for the device
    pub id: String,

    /// PCIe configuration
    #[serde(default, skip_serializing_if = "is_false")]
    pub pcie: bool,

    /// VGA passthrough (for GPU devices)
    #[serde(default, skip_serializing_if = "is_false")]
    pub x_vga: bool,

    /// Optional guest bus placement for the passthrough device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Optional guest slot/function address for the passthrough device
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,

    /// Enable multifunction on the guest slot when grouping related functions
    #[serde(default, skip_serializing_if = "is_false")]
    pub multifunction: bool,

    /// ROM file path (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostbus: Option<String>,

    /// USB host port path for Proxmox-style addressing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostport: Option<String>,

    /// USB controller bus (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// USB controller port (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
}

/// XHCI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhciControllerConfig {
    /// Unique identifier for the controller
    pub id: String,

    /// Number of USB2 ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p2: Option<u8>,

    /// Number of USB3 ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p3: Option<u8>,

    /// Parent bus placement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,

    /// Address on the selected bus
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
}
