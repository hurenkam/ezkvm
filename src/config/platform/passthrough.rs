use serde::{Deserialize, Serialize};
use std::collections::HashSet;

fn is_false(value: &bool) -> bool {
    !*value
}

/// Hardware passthrough configuration for PCI devices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostPciConfig {
    /// PCI device address (e.g., "0000:03:00.0")
    pub device: String,

    /// Unique identifier for the device.
    /// When empty, auto-generated as "hostpci{index}" at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
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

impl HostPciConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = format!("hostpci{}", candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }
}

/// USB device passthrough configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceConfig {
    /// Unique identifier for the USB device.
    /// When empty, auto-generated as "usb{index}" at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
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

impl UsbDeviceConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = format!("usb{}", candidate_index);
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }
}

/// XHCI controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhciControllerConfig {
    /// Unique identifier for the controller.
    /// When empty, auto-generated as "xhci" (index 0) or "xhci{index}" at load time.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub id: String,

    /// Number of USB2 ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p2: Option<u8>,

    /// Optional USB devices owned by this XHCI controller in controller-centric YAML.
    /// This field is normalized into `host.usb` and omitted from canonical serialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usb: Vec<UsbDeviceConfig>,

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

impl XhciControllerConfig {
    pub fn assign_default_id(&mut self, index: usize, reserved_ids: &mut HashSet<String>) {
        if self.id.trim().is_empty() {
            let mut candidate_index = index;
            loop {
                let candidate = if candidate_index == 0 {
                    "xhci".to_string()
                } else {
                    format!("xhci{}", candidate_index)
                };
                if reserved_ids.insert(candidate.clone()) {
                    self.id = candidate;
                    break;
                }
                candidate_index += 1;
            }
        }
    }
}
