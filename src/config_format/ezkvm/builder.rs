use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::{
    config_format::ezkvm::{
        Device, EzkvmConfigSchema,
        schema::{AudioSchema, BootModelBuilder},
    },
    runtime_model::{
        Audio, AudioBackend, AudioController, AudioModelBuilder, BusRegister, BusRegistrationApi,
        Chipset, DisplayModelBuilder, GuestAgentModelBuilder, I440fxChipset, IdeDeviceBuilder,
        NetworkResource, PcieDeviceApi, PcieDeviceResource, PcieDeviceType, PvScsiController,
        Q35Chipset, Resource, RuntimeModel, SataDeviceBuilder, ScsiDeviceBuilder, StorageResource,
        TpmModelBuilder, UsbDeviceBuilder, UsbDeviceResource, VirtioNetController,
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
        let resources = value.host.resources;

        let mut storage_resources: HashMap<String, StorageResource> = HashMap::new();
        let mut network_resources: HashMap<String, NetworkResource> = HashMap::new();
        let mut pcie_resources: HashMap<String, PcieDeviceResource> = HashMap::new();
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
                Resource::PcieDevice { id, pcie } => {
                    pcie_resources.insert(id, pcie);
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
        let smbios_uuid = vm.smbios_uuid.clone();
        let vmgenid = vm.vmgenid.clone();

        let display = value
            .host
            .display
            .and_then(|d| serde_json::from_value(serde_json::to_value(d).ok()?).ok())
            .or(vm.display)
            .map(DisplayModelBuilder::build);
        let audio = value
            .host
            .audio
            .map(audio_schema_to_audio)
            .or(vm.audio)
            .map(AudioModelBuilder::build);
        let guest_agent = vm.guest_agent.map(GuestAgentModelBuilder::build);

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
                        PcieDeviceType::Passthrough {
                            resource,
                            host,
                            id,
                            multifunction,
                            rombar,
                            romfile,
                        } => {
                            let (
                                resolved_host,
                                resolved_multifunction,
                                resolved_rombar,
                                resolved_romfile,
                            ) = resolve_pcie_passthrough(
                                resource.as_ref(),
                                host.as_ref(),
                                *multifunction,
                                *rombar,
                                romfile.as_ref(),
                                &pcie_resources,
                            )?;

                            let resolved = PcieDeviceType::Passthrough {
                                resource: resource.clone(),
                                host: Some(resolved_host),
                                id: id.clone(),
                                multifunction: resolved_multifunction,
                                rombar: resolved_rombar,
                                romfile: resolved_romfile,
                            };
                            (&resolved).into()
                        }
                        _ => pcie.device().into(),
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
            md.vm_name,
            cpu,
            vm.memory,
            chipset,
            boot,
            smbios_uuid,
            vmgenid,
            tpm,
            display,
            audio,
            guest_agent,
            register,
        ))
    }
}

fn audio_schema_to_audio(audio_schema: AudioSchema) -> Audio {
    let backend = match audio_schema {
        AudioSchema::Alsa { .. } => AudioBackend::Alsa,
        AudioSchema::PulseAudio { .. } => AudioBackend::PulseAudio,
        AudioSchema::PipeWire { .. } => AudioBackend::PipeWire,
    };

    Audio {
        backend,
        controller: AudioController::Ich9IntelHda,
    }
}

type ResolvedPciePassthrough = (String, Option<bool>, Option<bool>, Option<String>);

fn resolve_pcie_passthrough(
    resource_id: Option<&String>,
    host: Option<&String>,
    multifunction: Option<bool>,
    rombar: Option<bool>,
    romfile: Option<&String>,
    pcie_resources: &HashMap<String, PcieDeviceResource>,
) -> Result<ResolvedPciePassthrough, String> {
    if let Some(resource_id) = resource_id {
        let resource = pcie_resources.get(resource_id).ok_or_else(|| {
            format!(
                "missing pcie resource '{}' referenced by PCIe passthrough device",
                resource_id
            )
        })?;

        return match resource {
            PcieDeviceResource::HostAddress {
                address,
                multifunction,
                rombar,
                romfile,
            } => Ok((address.clone(), *multifunction, *rombar, romfile.clone())),
            PcieDeviceResource::Address { bus, address } => {
                let resolved = format!(
                    "0000:{:02x}:{:02x}.{}",
                    bus,
                    address.device(),
                    address.function()
                );
                Ok((resolved, multifunction, rombar, romfile.cloned()))
            }
        };
    }

    let resolved_host = host
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "PCIe passthrough device requires either 'resource' or legacy 'host'".to_string()
        })?
        .to_string();

    Ok((resolved_host, multifunction, rombar, romfile.cloned()))
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
            "host": {
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
            }
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
host:
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
