use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::{
    config_format::ezkvm::{Device, EzkvmConfigSchema, schema::BootModelBuilder},
    runtime_model::{
        BusRegister, BusRegistrationApi, Chipset, I440fxChipset, IdeDeviceBuilder, NetworkResource,
        PcieDeviceApi, PcieDeviceType, PvScsiController, Q35Chipset, Resource, RuntimeModel,
        SataDeviceBuilder, ScsiDeviceBuilder, StorageResource, TpmModelBuilder, UsbDeviceBuilder,
        UsbDeviceResource, VirtioNetController,
    },
};

#[derive(Getters, new)]
#[allow(dead_code)]
pub struct EzkvmHostSchema {
    name: String,
}
#[derive(Default)]
#[allow(dead_code)]
pub struct EzkvmRuntimeModelBuilder {
    name: Option<String>,
    host_config: Option<EzkvmHostSchema>,
    vm_config: Option<EzkvmConfigSchema>,
}
impl EzkvmRuntimeModelBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_vm_config(self, config: EzkvmConfigSchema) -> Self {
        Self {
            vm_config: Some(config),
            ..self
        }
    }
    pub fn with_host_config(self, config: EzkvmHostSchema) -> Self {
        Self {
            host_config: Some(config),
            ..self
        }
    }
    pub fn with_name(self, name: String) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }
    pub fn build(&self) -> Result<RuntimeModel, String> {
        let value = match self.vm_config {
            Some(ref config) => config.clone(),
            None => return Err("missing VM config".to_string()),
        };
        let vm = value.virtual_machine;
        let md = value.metadata;
        let resources = value.resources;

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
            "i440fx" => Chipset::I440FX(I440fxChipset::new(&register)),
            other => return Err(format!("Unsupported chipset: {}", other)),
        };
        let tpm = match vm.tpm {
            Some(ref tpm) => Some(TpmModelBuilder::build(tpm.clone(), &storage_resources)?),
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

#[cfg(test)]
mod tests {
    use super::EzkvmRuntimeModelBuilder;
    use crate::config_format::ezkvm::EzkvmConfigSchema;

    #[test]
    fn q35_supported_subset_renders_valid_command() {
        let runtime_config: EzkvmConfigSchema = serde_json::from_value(serde_json::json!({
            "metadata": {
                "schema_version": "1.0.0",
                "vm_name": "demo"
            },
            "virtual_machine": {
                "machine": {
                    "family": "pc",
                    "chipset": "q35"
                },
                "memory": {
                    "size": 8589934592u64
                },
                "boot": {
                    "uefi": {
                        "resource": "firmware0"
                    }
                },
                "swtpm": {
                    "version": 2.0,
                    "resource": "tpmstate0"
                },
                "devices": [
                    {
                        "pcie": {
                            "type": "pv_scsi"
                        }
                    },
                    {
                        "scsi": {
                            "type": "hdd",
                            "resource": "disk0"
                        }
                    },
                    {
                        "pcie": {
                            "type": "virtio_net",
                            "resource": "net0"
                        }
                    },
                    {
                        "ide": {
                            "type": "cdrom",
                            "resource": "iso0"
                        }
                    }
                ]
            },
            "resources": [
                {
                    "id": "firmware0",
                    "storage": {
                        "file": "/var/lib/ezkvm/efivars.fd"
                    }
                },
                {
                    "id": "tpmstate0",
                    "storage": {
                        "file": "/var/lib/ezkvm/tpmstate"
                    }
                },
                {
                    "id": "disk0",
                    "storage": {
                        "block_device": "/dev/vm/disk0"
                    }
                },
                {
                    "id": "iso0",
                    "storage": {
                        "file": "/iso/debian.iso"
                    }
                },
                {
                    "id": "net0",
                    "network": {
                        "bridge": "vmbr0"
                    }
                }
            ]
        }))
        .expect("json should parse");

        let model = EzkvmRuntimeModelBuilder::new()
            .with_vm_config(runtime_config)
            .build()
            .expect("runtime model should build");
        let command = model.qemu_command();

        assert_eq!(command[0], "qemu-system-x86_64");
        assert!(command.contains(&"type=q35".to_string()));
        assert!(command.contains(&"menu=on,strict=on,reboot-timeout=1000".to_string()));
        assert!(command.contains(&"if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/var/lib/ezkvm/efivars.fd,size=540672".to_string()));
        assert!(command.contains(&"socket,id=tpmchar,path=/var/run/ezkvm/demo.swtpm".to_string()));
        assert!(command.contains(&"pvscsi,id=scsihw0,bus=pcie.0,addr=0x0.0".to_string()));
        assert!(command.contains(&"id=drive-scsi0,file=/dev/vm/disk0,if=none,format=raw,discard=unmap,detect-zeroes=unmap".to_string()));
        assert!(
            command.contains(
                &"scsi-hd,bus=scsihw0.0,channel=0,scsi-id=0,lun=0,drive=drive-scsi0,id=scsi0"
                    .to_string()
            )
        );
        assert!(command.contains(&"bridge,id=net0f1,br=vmbr0".to_string()));
        assert!(
            command.contains(
                &"virtio-net-pci,id=net0f1,netdev=net0f1,bus=pcie.0,addr=0x0.1".to_string()
            )
        );
        assert!(
            command.contains(
                &"if=none,id=drive-ide0,file=/iso/debian.iso,format=raw,media=cdrom,readonly=on"
                    .to_string()
            )
        );
        assert!(command.contains(&"ide-cd,bus=ide.1,unit=0,drive=drive-ide0,id=ide0".to_string()));
    }

    #[test]
    fn command_display_shell_escapes_paths_with_spaces() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "demo"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 1073741824
    devices:
        - ide:
                type: cdrom
                resource: "iso0"
resources:
    - id: "iso0"
      storage:
        file: "/iso/Debian 12.iso"
"#;

        let runtime_config: EzkvmConfigSchema =
            serde_yaml::from_str(yaml).expect("yaml should parse");
        let model = EzkvmRuntimeModelBuilder::new()
            .with_vm_config(runtime_config)
            .build()
            .expect("runtime model should build");
        let display = model.qemu_command_display();

        assert!(display.contains(
            "'if=none,id=drive-ide0,file=/iso/Debian 12.iso,format=raw,media=cdrom,readonly=on'"
        ));
    }
}
