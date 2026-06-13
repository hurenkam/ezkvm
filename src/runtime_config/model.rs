use std::fmt;

use serde::{Deserialize, Serialize};

use crate::runtime_model::{
    Cpu, IdeAddress, IdeBus, IdeDevice, Memory, PciAddress, PciBus, PciDevice, PcieAddress,
    PcieBus, PcieDevice, SataAddress, SataBus, SataDevice, ScsiAddress, ScsiBus, ScsiDevice,
    UsbAddress, UsbBus, UsbDevice,
};

/// Schema version for ezkvm runtime config specification.
pub const EZKVM_CONFIG_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageResource {
    File { path: String },
    BlockDevice { path: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkResource {
    Tap { name: String },
    Bridge { name: String, bridge: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PciDeviceResource {
    Address { bus: PciBus, address: PciAddress },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PcieDeviceResource {
    Address { bus: PcieBus, address: PcieAddress },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UsbDeviceResource {
    Id { vendor_id: u16, device_id: u16 },
    Address { bus: UsbBus, address: UsbAddress },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    Storage { storage: StorageResource },
    Network { network: NetworkResource },
    PciDevice { pci_device: PciDeviceResource },
    PcieDevice { pcie_device: PcieDeviceResource },
    UsbDevice { usb_device: UsbDeviceResource },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RuntimeConfig {
    pub metadata: Metadata,
    pub virtual_machine: VirtualMachine,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub schema_version: String,
    pub vm_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VirtualMachine {
    pub machine: Machine,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<Cpu>,
    pub memory: Memory,
    pub devices: Vec<Device>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Device {
    Pcie {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<PcieBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<PcieAddress>,
        device: PcieDevice,
    },
    Pci {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<PciBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<PciAddress>,
        device: PciDevice,
    },
    Usb {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<UsbBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<UsbAddress>,
        device: UsbDevice,
    },
    Sata {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<SataBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<SataAddress>,
        device: SataDevice,
    },
    Ide {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<IdeBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port: Option<IdeAddress>,
        device: IdeDevice,
    },
    Scsi {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bus: Option<ScsiBus>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        address: Option<ScsiAddress>,
        device: ScsiDevice,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Machine {
    pub family: String,
    pub chipset: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
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
    - type: sata
      device: {}
resources:
  - type: network
    network:
      type: bridge
      name: "vmbr0"
      bridge: "vmbr0"
"#;

        let config: RuntimeConfig = serde_yaml::from_str(yaml).expect("yaml should deserialize");

        assert!(config.virtual_machine.cpu.is_none());
        assert!(config.virtual_machine.machine.version.is_none());
        match &config.virtual_machine.devices[0] {
            Device::Sata { bus, address, .. } => {
                assert!(bus.is_none());
                assert!(address.is_none());
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
                devices: vec![Device::Sata {
                    bus: None,
                    address: None,
                    device: SataDevice::SataDisk,
                }],
            },
            resources: vec![Resource::Network {
                network: NetworkResource::Bridge {
                    name: "net0".to_string(),
                    bridge: "vmbr0".to_string(),
                },
            }],
        };

        let yaml = serde_yaml::to_string(&config).expect("yaml should serialize");

        assert!(!yaml.contains("cpu: null"));
        assert!(!yaml.contains("version: null"));
        assert!(!yaml.contains("bus: null"));
        assert!(!yaml.contains("address: null"));
    }
}
