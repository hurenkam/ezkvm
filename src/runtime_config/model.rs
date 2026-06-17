use derive_getters::Getters;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, sync::Arc};

use crate::runtime_model::{
    BootModelBuilder, BusRegister, BusRegistrationApi, Chipset, Cpu, I440fxChipset, IdeDevice,
    IdeDeviceBuilder, Memory, PciAddress, PciBus, PciDevice, PcieAddress, PcieBus, PcieDevice,
    PcieDeviceApi, PcieDeviceType, PvScsiController, Q35Chipset, RuntimeModel, SataDevice,
    SataDeviceBuilder, ScsiDevice, ScsiDeviceBuilder, TpmModelBuilder, UsbAddress, UsbBus,
    UsbDevice, UsbDeviceBuilder, VirtioNetController,
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
impl TryFrom<&RuntimeModel> for RuntimeConfig {
    type Error = String;

    fn try_from(_value: &RuntimeModel) -> Result<Self, Self::Error> {
        Err("exporting runtime model to config is not implemented yet".to_string())
    }
}
impl TryFrom<RuntimeConfig> for RuntimeModel {
    type Error = String;

    fn try_from(value: RuntimeConfig) -> Result<Self, Self::Error> {
        let RuntimeConfig {
            metadata: md,
            virtual_machine: vm,
            resources,
        } = value;

        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();
        let mut network_resources: HashMap<String, NetworkResource> = HashMap::new();
        let mut usb_resources: HashMap<String, UsbDeviceResource> = HashMap::new();
        for resource in resources {
            match resource {
                Resource::Storage { id, storage } => {
                    storage_resources.insert(id, storage);
                }
                Resource::Network { id, network } => {
                    network_resources.insert(id, network);
                }
                Resource::UsbDevice { id, usb_device } => {
                    usb_resources.insert(id, usb_device);
                }
                Resource::PciDevice {
                    id: _,
                    pci_device: _,
                } => {
                    // Handle PCI device resources if needed
                }
                Resource::PcieDevice {
                    id: _,
                    pcie_device: _,
                } => {
                    // Handle PCIe device resources if needed
                }
            }
        }

        let cpu = vm.cpu.unwrap_or_default();
        let mut register = BusRegister::new();
        let chipset = match vm.machine.chipset.as_str() {
            "q35" => Chipset::Q35(Q35Chipset::new(&mut register)),
            "i440fx" => Chipset::I440FX(I440fxChipset::new(&mut register)),
            other => return Err(format!("Unsupported chipset: {}", other)),
        };
        let tpm = match vm.tpm {
            Some(ref tpm) => Some(TpmModelBuilder::build(tpm, &storage_resources)?),
            None => None,
        };
        let boot = BootModelBuilder::build(&vm.boot, &storage_resources)?;

        // TODO:
        //   - spice/vnc/gpu
        //   - serial ports
        //   - audio
        //   - qmp/guest agent

        for device in vm.devices {
            match device {
                Device::Pcie { pcie } => {
                    let pcie_api: Arc<dyn PcieDeviceApi> = match pcie.device() {
                        PcieDeviceType::PvScsi => {
                            let controller = Arc::new(PvScsiController::default());
                            register.register_scsi_bus(controller.clone())?;
                            controller
                        }
                        PcieDeviceType::VirtioNet { resource } => {
                            let resolved = match resource {
                                Some(id) => Some(
                                    network_resources
                                        .get(id)
                                        .cloned()
                                        .ok_or_else(|| {
                                            format!(
                                                "missing network resource '{}' referenced by PCIe virtio_net device",
                                                id
                                            )
                                        })?,
                                ),
                                None => None,
                            };
                            Arc::new(VirtioNetController::new(resolved))
                        }
                    };
                    match register.pcie_busses().get(&pcie.bus().unwrap_or_default()) {
                        Some(controller) => {
                            controller.register_pcie_device(pcie_api, pcie.address().clone())?
                        }
                        None => {
                            return Err(format!(
                                "PCIe bus with id {} does not exist",
                                pcie.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
                Device::Pci { pci } => {
                    match register.pci_busses().get(&pci.bus().unwrap_or_default()) {
                        Some(controller) => controller
                            .register_pci_device(pci.device().into(), pci.address().clone())?,
                        None => {
                            return Err(format!(
                                "PCI bus with id {} does not exist",
                                pci.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
                Device::Usb { usb } => {
                    match register.usb_busses().get(&usb.bus().unwrap_or_default()) {
                        Some(controller) => controller.register_usb_device(
                            UsbDeviceBuilder::build(usb.device(), &usb_resources),
                            usb.address().clone(),
                        )?,
                        None => {
                            return Err(format!(
                                "USB bus with id {} does not exist",
                                usb.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
                Device::Ide { ide } => {
                    match register.ide_busses().get(&ide.bus().unwrap_or_default()) {
                        Some(controller) => controller.register_ide_device(
                            IdeDeviceBuilder::build(ide.device(), &storage_resources)?,
                            ide.address().clone(),
                        )?,
                        None => {
                            return Err(format!(
                                "IDE bus with id {} does not exist",
                                ide.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
                Device::Sata { sata } => {
                    match register.sata_busses().get(&sata.bus().unwrap_or_default()) {
                        Some(controller) => controller.register_sata_device(
                            SataDeviceBuilder::build(sata.device(), &storage_resources)?,
                            sata.address().clone(),
                        )?,
                        None => {
                            return Err(format!(
                                "SATA bus with id {} does not exist",
                                sata.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
                Device::Scsi { scsi } => {
                    match register.scsi_busses().get(&scsi.bus().unwrap_or_default()) {
                        Some(controller) => controller.register_scsi_device(
                            ScsiDeviceBuilder::build(scsi.device(), &storage_resources)?,
                            scsi.address().clone(),
                        )?,
                        None => {
                            return Err(format!(
                                "SCSI bus with id {} does not exist",
                                scsi.bus().unwrap_or_default()
                            ));
                        }
                    }
                }
            }
        }

        Ok(RuntimeModel::new(
            md.vm_name, cpu, vm.memory, chipset, boot, tpm, register,
        ))
    }
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
