use serde::{Deserialize, Serialize};

use crate::config::ezkvm::schema::pci::{PciAddressSchema, PciBusSchema};
use crate::config::ezkvm::schema::pcie::{PcieAddressSchema, PcieBusSchema};
use crate::config::ezkvm::schema::usb::{UsbAddressSchema, UsbBusSchema};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StorageResourceSchema {
    File { file: String },
    BlockDevice { block_device: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NetworkResourceSchema {
    Tap { tap: String },
    Bridge { bridge: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PciDeviceResourceSchema {
    Address {
        bus: PciBusSchema,
        address: PciAddressSchema,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PcieDeviceResourceSchema {
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
        bus: PcieBusSchema,
        address: PcieAddressSchema,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UsbDeviceResourceSchema {
    Id {
        vendor_id: u16,
        device_id: u16,
    },
    HostBusPort {
        hostbus: u16,
        hostport: String,
    },
    Address {
        bus: UsbBusSchema,
        address: UsbAddressSchema,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResourceSchema {
    Storage {
        id: String,
        storage: StorageResourceSchema,
    },
    Network {
        id: String,
        network: NetworkResourceSchema,
    },
    PciDevice {
        id: String,
        pci: PciDeviceResourceSchema,
    },
    PcieDevice {
        id: String,
        pcie: PcieDeviceResourceSchema,
    },
    UsbDevice {
        id: String,
        usb: UsbDeviceResourceSchema,
    },
}
