use derive_new::new;
use serde::{Deserialize, Serialize};

//use crate::runtime_model::{PciAddress, PciBus, PcieAddress, PcieBus, UsbAddress, UsbBus};

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

type PciBus = u8;
#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PciAddress {
    pub device: u8,
    pub function: u8,
}
impl std::fmt::Display for PciAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PciDeviceResource {
    Address { bus: PciBus, address: PciAddress },
}

type PcieBus = u8;
#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PcieAddress {
    pub device: u8,
    pub function: u8,
}
impl std::fmt::Display for PcieAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PcieDeviceResource {
    HostAddress {
        address: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        multifunction: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rombar: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        romfile: Option<String>,
    },
    Address {
        bus: PcieBus,
        address: PcieAddress,
    },
}

//#[derive(Debug, Clone, Deserialize, Serialize)]
//#[serde(untagged)]
//pub enum UsbDeviceResource {
//    Id { vendor_id: u16, device_id: u16 },
//    HostBusPort { hostbus: u16, hostport: String },
//    Address { bus: UsbBus, address: UsbAddress },
//}

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
        #[serde(rename = "pcie", alias = "pcie_device")]
        pcie: PcieDeviceResource,
    },
    //UsbDevice {
    //    id: String,
    //    usb_device: UsbDeviceResource,
    //},
}
