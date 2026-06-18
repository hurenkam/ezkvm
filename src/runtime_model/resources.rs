use serde::{Deserialize, Serialize};

use crate::runtime_model::{PciAddress, PciBus, PcieAddress, PcieBus, UsbAddress, UsbBus};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StorageResource {
    File { file: String },
    BlockDevice { block_device: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NetworkResource {
    Tap { tap: String },
    Bridge { bridge: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PciDeviceResource {
    Address { bus: PciBus, address: PciAddress },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PcieDeviceResource {
    Address { bus: PcieBus, address: PcieAddress },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UsbDeviceResource {
    Id { vendor_id: u16, device_id: u16 },
    Address { bus: UsbBus, address: UsbAddress },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Resource {
    Storage {
        id: String,
        storage: StorageResource,
    },
    Network {
        id: String,
        network: NetworkResource,
    },
    PciDevice {
        id: String,
        pci_device: PciDeviceResource,
    },
    PcieDevice {
        id: String,
        pcie_device: PcieDeviceResource,
    },
    UsbDevice {
        id: String,
        usb_device: UsbDeviceResource,
    },
}
