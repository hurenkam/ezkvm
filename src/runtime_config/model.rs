use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::runtime_model::{
    Cpu, IdeDevice, Memory, PciAddress, PciBus, PciDevice, PcieAddress, PcieBus, PcieDevice,
    SataDevice, ScsiDevice, UsbAddress, UsbBus, UsbDevice,
};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub metadata: Metadata,
    pub virtual_machine: VirtualMachine,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub machine: Machine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Cpu>,
    pub memory: Memory,
    #[serde(default)]
    pub boot: Boot,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<Tpm>,
    pub devices: Vec<Device>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Device {
    Pcie { pcie: PcieDevice },
    Pci { pci: PciDevice },
    Usb { usb: UsbDevice },
    Sata { sata: SataDevice },
    Ide { ide: IdeDevice },
    Scsi { scsi: ScsiDevice },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, Getters)]
pub struct Boot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    secure: Option<bool>,
    #[serde(flatten)]
    bios: Bios,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bios {
    SeaBios { seabios: SeaBios },
    Uefi { uefi: Uefi },
}
impl Default for Bios {
    fn default() -> Self {
        Bios::SeaBios {
            seabios: SeaBios::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SeaBios {
    firmware: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Uefi {
    resource: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Tpm {
    Emulated { swtpm: Swtpm },
    Passthrough { hwtpm: Hwtpm },
}
impl Default for Tpm {
    fn default() -> Self {
        Tpm::Emulated {
            swtpm: Swtpm::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Swtpm {
    version: f32,
    resource: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters)]
pub struct Hwtpm {
    resource: String,
}

impl fmt::Display for RuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = serde_yaml::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_model::SataDeviceType;

    #[test]
    fn omitted_optional_fields_deserialize_as_none() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "demo-vm"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 8589934592
  devices:
    - sata:
        type: ssd
        resource: "disk0"
resources:
  - storage:
      block_device: "/dev/vm/disk0"
    id: "disk0"
  - network:
      bridge: "vmbr0"
    id: "net0"
"#;

        let config: RuntimeConfig = serde_yaml::from_str(yaml).expect("yaml should deserialize");

        assert!(config.virtual_machine.cpu.is_none());
        assert!(config.virtual_machine.machine.version.is_none());
        match &config.virtual_machine.devices[0] {
            Device::Sata { sata } => {
                assert!(sata.address().is_none());
            }
            other => panic!("expected sata device, got {other:?}"),
        }
    }

    #[test]
    fn optional_none_fields_serialize_without_nulls() {
        let config = RuntimeConfig {
            metadata: Metadata {
                schema_version: EZKVM_CONFIG_SCHEMA_VERSION.to_string(),
                vm_name: "demo-vm".to_string(),
            },
            virtual_machine: VirtualMachine {
                machine: Machine {
                    family: "pc".to_string(),
                    chipset: "q35".to_string(),
                    version: None,
                },
                cpu: None,
                memory: Memory::gigabytes(8),
                boot: Boot::default(),
                tpm: None,
                devices: vec![Device::Sata {
                    sata: SataDevice::new(
                        None,
                        None,
                        SataDeviceType::Ssd {
                            resource: "disk0".to_string(),
                        },
                    ),
                }],
            },
            resources: vec![
                Resource::Storage {
                    id: "disk0".to_string(),
                    storage: StorageResource::File {
                        file: "/path/to/disk.img".to_string(),
                    },
                },
                Resource::Network {
                    id: "net0".to_string(),
                    network: NetworkResource::Bridge {
                        bridge: "vmbr0".to_string(),
                    },
                },
            ],
        };

        let yaml = serde_yaml::to_string(&config).expect("yaml should serialize");

        assert!(!yaml.contains("cpu: null"));
        assert!(!yaml.contains("version: null"));
        assert!(!yaml.contains("sata: null"));
        assert!(!yaml.contains("address: null"));
    }
}
