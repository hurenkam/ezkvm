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
pub enum PciResourceSchema {
    Address {
        bus: PciBusSchema,
        address: PciAddressSchema,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PcieResourceSchema {
    HostAddress {
        address: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        functions: Vec<u8>,
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
pub enum UsbResourceSchema {
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
pub struct MemoryResourceSchema {
    pub path: String,
    pub size: String,
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
        pci: PciResourceSchema,
    },
    PcieDevice {
        id: String,
        pcie: PcieResourceSchema,
    },
    UsbDevice {
        id: String,
        usb: UsbResourceSchema,
    },
    Memory {
        id: String,
        memory: MemoryResourceSchema,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostpci_resource_round_trips_yaml() {
        let schema = ResourceSchema::PcieDevice {
            id: "hostpci0".to_string(),
            pcie: PcieResourceSchema::HostAddress {
                address: "0000:03:00".to_string(),
                functions: vec![0u8, 1u8],
                rombar: None,
                romfile: None,
            },
        };
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        let decoded: ResourceSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        if let ResourceSchema::PcieDevice { pcie: PcieResourceSchema::HostAddress { functions, .. }, .. } = decoded {
            assert_eq!(functions, vec![0u8, 1u8]);
        } else {
            panic!("expected PcieDevice::HostAddress");
        }
    }

    #[test]
    fn memory_resource_round_trips_yaml() {
        let schema = ResourceSchema::Memory {
            id: "shm0".to_string(),
            memory: MemoryResourceSchema {
                path: "/dev/kvmfr0".to_string(),
                size: "128M".to_string(),
            },
        };
        let yaml = crate::serde_yaml::to_string(&schema).unwrap();
        let decoded: ResourceSchema = crate::serde_yaml::from_str(&yaml).unwrap();
        if let ResourceSchema::Memory { id, memory } = decoded {
            assert_eq!(id, "shm0");
            assert_eq!(memory.path, "/dev/kvmfr0");
            assert_eq!(memory.size, "128M");
        } else {
            panic!("expected Memory variant");
        }
    }
}
