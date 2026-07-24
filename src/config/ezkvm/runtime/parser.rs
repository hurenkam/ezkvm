use crate::{
    config::ezkvm::{
        ConfigSchema,
        schema::{
            AudioDeviceSchema, BiosSchema, BootSchema, ChipsetSchema, ConfigSchema as SchemaConfig,
            DeviceSchema, DisplaySchema, HostSchema, I440FXChipsetSchema, IdeAddressSchema,
            IdeDeviceSchema, IdeDeviceTypeSchema, MachineSchema, MemoryResourceSchema,
            MemorySchema, MetadataSchema, PciAddressSchema, PciDeviceSchema, PciDeviceTypeSchema,
            PcieAddressSchema, PcieDeviceSchema, PcieDeviceTypeSchema, PcieResourceSchema,
            Q35ChipsetSchema, RawArgsSchema, ResourceSchema, SataAddressSchema, SataDeviceSchema,
            SataDeviceTypeSchema, ScsiAddressSchema, ScsiDeviceSchema, ScsiDeviceTypeSchema,
            SpiceSchema, StorageResourceSchema, SwtpmSchema, TpmSchema, UefiSchema,
            UsbAddressSchema, UsbDeviceSchema, UsbDeviceTypeSchema, VirtualMachineSchema,
            EZKVM_CONFIG_SCHEMA_VERSION,
        },
    },
    runtime::{
        AudioDevice, Cdrom, Chipset, EfiDisk, GenericPciDevice, GenericUsbDevice, Hdd, HostPci,
        Ivshmem, Memory, PciBusDeviceKind, PciDeviceKind, PcieBusDeviceKind, PvScsi, Q35Chipset,
        RawArgs, RootDeviceKind, Runtime, SpiceDisplay, Ssd, StorageDeviceType, TpmState,
        UsbBusDeviceKind, UsbDeviceKind, VirtioNetPcie,
    },
};
use super::error::YamlRuntimeError;
use derive_getters::Getters;
use derive_new::new;

#[derive(Debug, Getters, new)]
#[allow(dead_code)]
pub struct Parser {
    runtime: Runtime,
}
impl Parser {
    pub fn parse(&self) -> Result<ConfigSchema, YamlRuntimeError> {
        let mut memory: Option<MemorySchema> = None;
        let mut chipset: Option<(ChipsetSchema, Vec<DeviceSchema>, Vec<ResourceSchema>)> = None;
        let mut efidisk_opt: Option<&EfiDisk> = None;
        let mut tpm_opt: Option<TpmSchema> = None;
        let mut audio_opt: Option<AudioDeviceSchema> = None;
        let mut raw_args_opt: Option<RawArgsSchema> = None;
        let mut spice_opt: Option<DisplaySchema> = None;

        for root_device in self.runtime.root_devices() {
            match root_device.device_kind() {
                RootDeviceKind::Memory => {
                    let mem = root_device.as_any().downcast_ref::<Memory>().unwrap();
                    memory = Some(MemorySchema::new(*mem.size(), None, false));
                }
                RootDeviceKind::Chipset => {
                    let chipset_runtime = root_device.as_any().downcast_ref::<Chipset>().unwrap();
                    chipset = Some(self.parse_chipset(chipset_runtime)?);
                }
                RootDeviceKind::EfiDisk => {
                    efidisk_opt = Some(root_device.as_any().downcast_ref::<EfiDisk>().unwrap());
                }
                RootDeviceKind::TpmState => {
                    let tpm = root_device.as_any().downcast_ref::<TpmState>().unwrap();
                    tpm_opt = Some(TpmSchema::Emulated {
                        swtpm: SwtpmSchema::new(tpm.version().clone(), tpm.storage_volume().clone()),
                    });
                }
                RootDeviceKind::AudioDevice => {
                    let audio = root_device.as_any().downcast_ref::<AudioDevice>().unwrap();
                    audio_opt = Some(AudioDeviceSchema::new(audio.device_type().clone(), audio.driver().clone()));
                }
                RootDeviceKind::RawArgs => {
                    let raw_args = root_device.as_any().downcast_ref::<RawArgs>().unwrap();
                    raw_args_opt = Some(RawArgsSchema::new(raw_args.0.clone()));
                }
                RootDeviceKind::SpiceDisplay => {
                    let spice = root_device.as_any().downcast_ref::<SpiceDisplay>().unwrap();
                    spice_opt = Some(DisplaySchema::Spice {
                        spice: SpiceSchema::new(
                            (*spice.port()).unwrap_or(0),
                            spice.addr().clone().unwrap_or_else(|| "0.0.0.0".to_string()),
                            *spice.disable_ticketing(),
                            false,
                            None,
                            None,
                            false,
                            *spice.gl(),
                            spice.rendernode().clone(),
                            *spice.clipboard(),
                        ),
                    });
                }
                // CpuTopology/VgaConfig (added Phase 7 for QEMU -smp/-cpu/-vga emission)
                // have no corresponding YAML schema representation yet — deferred to a
                // later phase that extends the ezkvm YAML schema. No-op here.
                RootDeviceKind::CpuTopology | RootDeviceKind::VgaConfig => {}
            }
        }

        let memory = memory.ok_or(YamlRuntimeError::MissingMemory)?;
        let (chipset, devices, resources) = chipset.ok_or(YamlRuntimeError::MissingChipset)?;

        let boot = match efidisk_opt {
            Some(efidisk) => BootSchema::new(
                None,
                BiosSchema::Uefi {
                    uefi: UefiSchema::new(
                        efidisk.storage_volume().clone(),
                        efidisk.efitype().clone(),
                        *efidisk.pre_enrolled_keys(),
                        efidisk.ms_cert().clone(),
                        Some(efidisk.logical_size().clone()),
                    ),
                },
            ),
            None => BootSchema::default(),
        };

        let metadata = MetadataSchema::new(EZKVM_CONFIG_SCHEMA_VERSION.to_string(), "vm".to_string());
        let host = HostSchema::new(spice_opt, None, resources);
        let machine = MachineSchema::new(chipset, None);
        let virtual_machine = VirtualMachineSchema::new(
            machine,
            None,
            memory,
            boot,
            None,
            None,
            tpm_opt,
            audio_opt,
            raw_args_opt,
            None,
            devices,
        );

        Ok(SchemaConfig::new(metadata, host, virtual_machine))
    }

    fn parse_chipset(
        &self,
        chipset: &Chipset,
    ) -> Result<(ChipsetSchema, Vec<DeviceSchema>, Vec<ResourceSchema>), YamlRuntimeError> {
        match chipset {
            Chipset::I440FX => Ok((
                ChipsetSchema::I440FX {
                    i440fx: I440FXChipsetSchema::new(None),
                },
                vec![],
                vec![],
            )),
            Chipset::Q35(q35) => {
                let (devices, resources) = self.parse_q35_devices(q35)?;
                Ok((
                    ChipsetSchema::Q35 {
                        q35: Q35ChipsetSchema::new(None),
                    },
                    devices,
                    resources,
                ))
            }
        }
    }

    fn parse_q35_devices(
        &self,
        q35: &Q35Chipset,
    ) -> Result<(Vec<DeviceSchema>, Vec<ResourceSchema>), YamlRuntimeError> {
        let mut devices = Vec::new();
        let mut resources: Vec<ResourceSchema> = Vec::new();
        let mut hostpci_idx: u32 = 0;

        let mut pcie_entries = q35.pcie_bus().iter().collect::<Vec<_>>();
        pcie_entries.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        for (pcie_addr, pcie_device) in pcie_entries {
            if pcie_device.device_kind() == PcieBusDeviceKind::PvScsi {
                let pvscsi = pcie_device.as_any().downcast_ref::<PvScsi>().unwrap();
                devices.push(DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        Some(PcieAddressSchema::new(*pcie_addr.device(), *pcie_addr.function())),
                        PcieDeviceTypeSchema::PvScsi,
                    ),
                });

                let mut scsi_entries = pvscsi.scsi_bus().iter().collect::<Vec<_>>();
                scsi_entries.sort_by_key(|(addr, _)| (*addr.target(), *addr.lun()));
                for (scsi_addr, scsi_device) in scsi_entries {
                    let resource_id = format!("scsi-{}-{}", scsi_addr.target(), scsi_addr.lun());
                    let path = storage_device_resource_path(scsi_device.device_kind(), scsi_device.as_any());
                    resources.push(ResourceSchema::Storage {
                        id: resource_id.clone(),
                        storage: storage_resource_schema(&path),
                    });

                    let scsi_type = match scsi_device.device_kind() {
                        StorageDeviceType::Hdd => ScsiDeviceTypeSchema::Hdd { resource: resource_id.clone() },
                        StorageDeviceType::Ssd => ScsiDeviceTypeSchema::Ssd { resource: resource_id.clone() },
                        StorageDeviceType::Odd => ScsiDeviceTypeSchema::Cdrom { resource: resource_id.clone() },
                    };

                    devices.push(DeviceSchema::Scsi {
                        scsi: ScsiDeviceSchema::new(
                            Some(0),
                            Some(ScsiAddressSchema::new(*scsi_addr.target(), *scsi_addr.lun())),
                            scsi_type,
                        ),
                    });
                }
                continue;
            }

            if pcie_device.device_kind() == PcieBusDeviceKind::VirtioNet {
                let virtio_net = pcie_device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
                devices.push(DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        Some(PcieAddressSchema::new(*pcie_addr.device(), *pcie_addr.function())),
                        PcieDeviceTypeSchema::VirtioNet {
                            resource: virtio_net.resource().clone(),
                            mac_address: virtio_net.mac_address().clone(),
                            rx_queue_size: *virtio_net.rx_queue_size(),
                            tx_queue_size: *virtio_net.tx_queue_size(),
                            vhost: *virtio_net.vhost(),
                        },
                    ),
                });
                continue;
            }

            if pcie_device.device_kind() == PcieBusDeviceKind::HostPci {
                let host_pci = pcie_device.as_any().downcast_ref::<HostPci>().unwrap();
                let resource_id = format!("hostpci{}", hostpci_idx);
                hostpci_idx += 1;
                resources.push(ResourceSchema::PcieDevice {
                    id: resource_id.clone(),
                    pcie: PcieResourceSchema::HostAddress {
                        address: host_pci.base_bdf().clone(),
                        functions: host_pci.functions().clone(),
                        rombar: *host_pci.rombar(),
                        romfile: host_pci.romfile().clone(),
                    },
                });
                devices.push(DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        Some(PcieAddressSchema::new(*pcie_addr.device(), *pcie_addr.function())),
                        PcieDeviceTypeSchema::HostPci {
                            resource: resource_id,
                            x_vga: *host_pci.x_vga(),
                        },
                    ),
                });
                continue;
            }

            if pcie_device.device_kind() == PcieBusDeviceKind::Ivshmem {
                let ivshmem = pcie_device.as_any().downcast_ref::<Ivshmem>().unwrap();
                resources.push(ResourceSchema::Memory {
                    id: ivshmem.id().clone(),
                    memory: MemoryResourceSchema {
                        path: ivshmem.mem_path().clone(),
                        size: ivshmem.size().clone(),
                    },
                });
                devices.push(DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        Some(PcieAddressSchema::new(*pcie_addr.device(), *pcie_addr.function())),
                        PcieDeviceTypeSchema::IvshmemPlain {
                            resource: ivshmem.id().clone(),
                        },
                    ),
                });
                continue;
            }

            return Err(YamlRuntimeError::UnsupportedPcieDevice {
                device_type: format!("{:?}", pcie_device.device_kind()),
            });
        }

        let mut sata_entries = q35.sata_bus().iter().collect::<Vec<_>>();
        sata_entries.sort_by_key(|(addr, _)| (*addr.port(), *addr.device()));
        for (sata_addr, sata_device) in sata_entries {
            let resource_id = format!("sata-{}-{}", sata_addr.port(), sata_addr.device());
            let path = storage_device_resource_path(sata_device.device_kind(), sata_device.as_any());
            resources.push(ResourceSchema::Storage {
                id: resource_id.clone(),
                storage: storage_resource_schema(&path),
            });

            let sata_type = match sata_device.device_kind() {
                StorageDeviceType::Hdd => SataDeviceTypeSchema::Hdd { resource: resource_id.clone() },
                StorageDeviceType::Ssd => SataDeviceTypeSchema::Ssd { resource: resource_id.clone() },
                StorageDeviceType::Odd => SataDeviceTypeSchema::Cdrom { resource: resource_id.clone() },
            };

            devices.push(DeviceSchema::Sata {
                sata: SataDeviceSchema::new(
                    Some(0),
                    Some(SataAddressSchema::new(*sata_addr.port())),
                    sata_type,
                ),
            });
        }

        let mut pci_entries = q35.pci_bus().iter().collect::<Vec<_>>();
        pci_entries.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        for (pci_addr, pci_device) in pci_entries {
            if pci_device.device_kind() != PciBusDeviceKind::GenericPci {
                return Err(YamlRuntimeError::UnsupportedPcieDevice {
                    device_type: format!("{:?} on legacy PCI bus", pci_device.device_kind()),
                });
            }
            let generic = pci_device.as_any().downcast_ref::<GenericPciDevice>().unwrap();

            let schema_device = match generic.kind() {
                PciDeviceKind::NetworkController => PciDeviceTypeSchema::NetworkController,
                PciDeviceKind::QxlGpu => PciDeviceTypeSchema::QxlGpu,
                PciDeviceKind::Ac97 => PciDeviceTypeSchema::Ac97,
            };

            devices.push(DeviceSchema::Pci {
                pci: PciDeviceSchema::new(
                    Some(0),
                    Some(PciAddressSchema::new(*pci_addr.device(), *pci_addr.function())),
                    schema_device,
                ),
            });
        }

        let mut ide_entries = q35.ide_bus().iter().collect::<Vec<_>>();
        ide_entries.sort_by_key(|(addr, _)| (*addr.channel(), *addr.device()));
        for (ide_addr, ide_device) in ide_entries {
            let resource_id = format!("ide-{}-{}", ide_addr.channel(), ide_addr.device());
            let path = storage_device_resource_path(
                ide_device.storage_options().device_type,
                ide_device.as_any(),
            );
            resources.push(ResourceSchema::Storage {
                id: resource_id.clone(),
                storage: storage_resource_schema(&path),
            });

            let ide_type = match ide_device.storage_options().device_type {
                StorageDeviceType::Hdd => IdeDeviceTypeSchema::Hdd { resource: resource_id.clone() },
                StorageDeviceType::Ssd => IdeDeviceTypeSchema::Ssd { resource: resource_id.clone() },
                StorageDeviceType::Odd => IdeDeviceTypeSchema::Cdrom { resource: resource_id.clone() },
            };

            devices.push(DeviceSchema::Ide {
                ide: IdeDeviceSchema::new(
                    Some(*ide_addr.channel()),
                    Some(IdeAddressSchema::new(*ide_addr.device())),
                    ide_type,
                ),
            });
        }

        let mut usb_entries = q35.usb_bus().iter().collect::<Vec<_>>();
        usb_entries.sort_by_key(|(addr, _)| addr.port().clone());
        for (usb_addr, usb_device) in usb_entries {
            if usb_device.device_kind() != UsbBusDeviceKind::Generic {
                return Err(YamlRuntimeError::UnsupportedPcieDevice {
                    device_type: format!("{:?} on usb bus", usb_device.device_kind()),
                });
            }
            let generic = usb_device.as_any().downcast_ref::<GenericUsbDevice>().unwrap();

            let schema_device = match generic.kind() {
                UsbDeviceKind::NetworkController => UsbDeviceTypeSchema::NetworkController,
                UsbDeviceKind::Tablet => UsbDeviceTypeSchema::Tablet,
                UsbDeviceKind::HostPassthrough { resource } => UsbDeviceTypeSchema::HostPassthrough {
                    resource: resource.clone(),
                },
            };

            devices.push(DeviceSchema::Usb {
                usb: UsbDeviceSchema::new(
                    Some(0),
                    Some(UsbAddressSchema::new(usb_addr.port().clone())),
                    schema_device,
                ),
            });
        }

        Ok((devices, resources))
    }
}

/// Choose the storage resource variant based on path shape: absolute device-node paths
/// (`/dev/...`) are `BlockDevice`; anything else is a `File`.
fn storage_resource_schema(path: &str) -> StorageResourceSchema {
    if path.starts_with("/dev/") {
        StorageResourceSchema::BlockDevice { block_device: path.to_string() }
    } else {
        StorageResourceSchema::File { file: path.to_string() }
    }
}

/// Downcast a storage bus device trait object to its concrete type (Hdd/Ssd/Cdrom) and
/// read the resource path. Never synthesizes a path — an unexpected mismatch between
/// `device_kind()` and the concrete type yields an empty string, which is a bug elsewhere
/// (device_kind() is guaranteed to match the concrete type per each StorageDevice impl).
fn storage_device_resource_path(kind: StorageDeviceType, device: &dyn std::any::Any) -> String {
    match kind {
        StorageDeviceType::Hdd => device.downcast_ref::<Hdd>().map(|d| d.resource.clone()).unwrap_or_default(),
        StorageDeviceType::Ssd => device.downcast_ref::<Ssd>().map(|d| d.resource.clone()).unwrap_or_default(),
        StorageDeviceType::Odd => device.downcast_ref::<Cdrom>().map(|d| d.resource.clone()).unwrap_or_default(),
    }
}

impl TryFrom<Runtime> for ConfigSchema {
    type Error = YamlRuntimeError;

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let parser = Parser::new(value);
        parser.parse()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::runtime::{
        AudioDevice, Cdrom, Chipset, EfiDisk, GenericPciDevice, GenericUsbDevice, Hdd, HostPci,
        IdeAddress, Ivshmem, Memory, PciAddress, PciDeviceKind, PcieAddress, PcieBusDeviceKind,
        PvScsiBuilder, Q35ChipsetBuilder, RawArgs, RootDeviceKind, Runtime, RuntimeBuilder,
        SataAddress, ScsiAddress, SpiceDisplay, Ssd, TpmState, UsbAddress, UsbDeviceKind,
        VirtioNetPcie,
    };

    #[test]
    fn round_trip_q35_pvscsi_scsi_ssd() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(1024))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(
                        Some(PcieAddress::new(0, 0)),
                        Arc::new(
                            PvScsiBuilder::new()
                                .with_scsi_device(Some(ScsiAddress::new(0, 0)), Arc::new(Ssd::new(String::new())))
                                .build(),
                        ),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        assert_runtime_has_memory_and_q35_counts(&round_tripped, 1024, 1, 1, 0);
    }

    #[test]
    fn round_trip_q35_sata_ssd() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(2048))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_sata_device(Some(SataAddress::new(1, 0)), Arc::new(Ssd::new(String::new())))
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        assert_runtime_has_memory_and_q35_counts(&round_tripped, 2048, 0, 0, 1);
    }

    #[test]
    fn round_trip_i440fx_memory() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let mut found_memory = false;
        let mut found_i440fx = false;
        for root in round_tripped.root_devices() {
            if root.device_kind() == RootDeviceKind::Memory {
                let memory = root.as_any().downcast_ref::<Memory>().unwrap();
                found_memory = true;
                assert_eq!(*memory.size(), 4096);
            }
            if root.device_kind() == RootDeviceKind::Chipset {
                let chipset = root.as_any().downcast_ref::<Chipset>().unwrap();
                if matches!(chipset, Chipset::I440FX) {
                    found_i440fx = true;
                }
            }
        }
        assert!(found_memory);
        assert!(found_i440fx);
    }

    #[test]
    fn round_trip_q35_extended_devices() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(
                        Some(PcieAddress::new(1, 0)),
                        Arc::new(
                            PvScsiBuilder::new()
                                .with_scsi_device(Some(ScsiAddress::new(2, 0)), Arc::new(Hdd::new(String::new())))
                                .build(),
                        ),
                    )
                    .with_pcie_device(
                        Some(PcieAddress::new(2, 0)),
                        Arc::new(VirtioNetPcie::new(
                            Some("net0".to_string()),
                            Some("de:ad:be:ef:00:01".to_string()),
                            Some(256),
                            Some(256),
                            Some(true),
                        )),
                    )
                    .with_pci_device(
                        Some(PciAddress::new(3, 0)),
                        Arc::new(GenericPciDevice::new(PciDeviceKind::QxlGpu)),
                    )
                    .with_usb_device(
                        Some(UsbAddress::new("2.1".to_string())),
                        Arc::new(GenericUsbDevice::new(UsbDeviceKind::Tablet)),
                    )
                    .with_ide_device(Some(IdeAddress::new(0, 1)), Arc::new(Cdrom::new(String::new())))
                    .with_sata_device(Some(SataAddress::new(4, 0)), Arc::new(Ssd::new(String::new())))
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let mut found_q35 = false;
        for root in round_tripped.root_devices() {
            if root.device_kind() == RootDeviceKind::Chipset {
                if let Chipset::Q35(q35) = root.as_any().downcast_ref::<Chipset>().unwrap() {
                    found_q35 = true;
                    assert_eq!(q35.pcie_bus().len(), 2);
                    assert_eq!(q35.pci_bus().len(), 1);
                    assert_eq!(q35.usb_bus().len(), 1);
                    assert_eq!(q35.ide_bus().len(), 1);
                    assert_eq!(q35.sata_bus().len(), 1);
                }
            }
        }
        assert!(found_q35);
    }

    fn assert_runtime_has_memory_and_q35_counts(
        runtime: &Runtime,
        expected_memory: usize,
        expected_pcie: usize,
        expected_scsi: usize,
        expected_sata: usize,
    ) {
        let mut memory_size: Option<usize> = None;
        let mut q35_counts: Option<(usize, usize, usize)> = None;

        for root in runtime.root_devices() {
            if root.device_kind() == RootDeviceKind::Memory {
                let memory = root.as_any().downcast_ref::<Memory>().unwrap();
                memory_size = Some(*memory.size());
            }
            if root.device_kind() == RootDeviceKind::Chipset {
                if let Chipset::Q35(q35) = root.as_any().downcast_ref::<Chipset>().unwrap() {
                    let pcie_count = q35.pcie_bus().len();
                    let mut scsi_count = 0usize;
                    for pcie_dev in q35.pcie_bus().values() {
                        if pcie_dev.device_kind() == PcieBusDeviceKind::PvScsi {
                            let pvscsi = pcie_dev.as_any().downcast_ref::<crate::runtime::PvScsi>().unwrap();
                            scsi_count += pvscsi.scsi_bus().len();
                        }
                    }
                    let sata_count = q35.sata_bus().len();
                    q35_counts = Some((pcie_count, scsi_count, sata_count));
                }
            }
        }

        assert_eq!(memory_size, Some(expected_memory));
        assert_eq!(q35_counts, Some((expected_pcie, expected_scsi, expected_sata)));
    }

    // ── TASK 1: tracer — EfiDisk end-to-end round-trip ─────────────────────────────

    #[test]
    fn round_trip_efidisk_fields_preserved() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .with_efidisk(EfiDisk::new(
                "/dev/vm1/vm-108-efidisk".to_string(),
                Some("4m".to_string()),
                true,
                None,
                "4M".to_string(),
                None,
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let efidisk = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::EfiDisk)
            .expect("EfiDisk not found")
            .as_any()
            .downcast_ref::<EfiDisk>()
            .expect("downcast to EfiDisk")
            .clone();

        assert_eq!(efidisk.storage_volume(), "/dev/vm1/vm-108-efidisk");
        assert_eq!(efidisk.efitype().as_deref(), Some("4m"));
        assert!(*efidisk.pre_enrolled_keys());
        assert_eq!(efidisk.ms_cert(), &None);
        assert_eq!(efidisk.logical_size(), "4M");
        assert_eq!(efidisk.block_device_size_bytes(), &None);
    }

    // ── TASK 2: remaining root-device round-trips ──────────────────────────────────

    #[test]
    fn round_trip_tpmstate_version_and_volume() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .with_tpmstate(TpmState::new(
                "/dev/vm1/vm-108-tpmstate".to_string(),
                "v2.0".to_string(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let tpm = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::TpmState)
            .expect("TpmState not found")
            .as_any()
            .downcast_ref::<TpmState>()
            .expect("downcast to TpmState");

        assert_eq!(tpm.storage_volume(), "/dev/vm1/vm-108-tpmstate");
        assert_eq!(tpm.version(), "v2.0");
    }

    #[test]
    fn round_trip_audio_device_fields() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .with_audio_device(AudioDevice::new(
                "ich9-intel-hda".to_string(),
                "spice".to_string(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let audio = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::AudioDevice)
            .expect("AudioDevice not found")
            .as_any()
            .downcast_ref::<AudioDevice>()
            .expect("downcast to AudioDevice");

        assert_eq!(audio.device_type(), "ich9-intel-hda");
        assert_eq!(audio.driver(), "spice");
    }

    #[test]
    fn round_trip_rawargs_verbatim() {
        let original = "-device vfio-pci,host=03:00.0 -object something,id=x";
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .with_raw_args(RawArgs(original.to_string()))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let raw_args = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::RawArgs)
            .expect("RawArgs not found")
            .as_any()
            .downcast_ref::<RawArgs>()
            .expect("downcast to RawArgs");

        assert_eq!(raw_args.0, original);
    }

    #[test]
    fn round_trip_spice_display() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::I440FX)
            .with_spice_display(SpiceDisplay::new(
                Some(5903),
                Some("0.0.0.0".to_string()),
                true,
                true,
                Some("/dev/dri/renderD128".to_string()),
                false,
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let spice = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::SpiceDisplay)
            .expect("SpiceDisplay not found")
            .as_any()
            .downcast_ref::<SpiceDisplay>()
            .expect("downcast to SpiceDisplay");

        assert_eq!(spice.port(), &Some(5903));
        assert_eq!(spice.addr().as_deref(), Some("0.0.0.0"));
        assert!(*spice.disable_ticketing());
        assert!(*spice.gl());
        assert_eq!(spice.rendernode().as_deref(), Some("/dev/dri/renderD128"));
        assert!(!*spice.clipboard());
    }

    // ── TASK 3: PCIe devices + storage resource path preservation ──────────────────

    #[test]
    fn round_trip_hostpci_base_bdf_and_functions() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(
                        Some(PcieAddress::new(0, 0)),
                        Arc::new(HostPci::new(
                            "0000:03:00".to_string(),
                            vec![0, 1],
                            true,
                            true,
                            Some(true),
                            None,
                        )),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let chipset = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset");
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35 chipset"),
        };
        let host_pci = q35
            .pcie_bus()
            .values()
            .find_map(|d| d.as_any().downcast_ref::<HostPci>())
            .expect("HostPci not found");

        assert_eq!(host_pci.base_bdf(), "0000:03:00");
        assert_eq!(host_pci.functions(), &vec![0u8, 1u8]);
        assert!(*host_pci.x_vga());
        assert_eq!(host_pci.rombar(), &Some(true));
        assert_eq!(host_pci.romfile(), &None);
    }

    #[test]
    fn round_trip_ivshmem_id_path_size() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(
                        Some(PcieAddress::new(0, 0)),
                        Arc::new(Ivshmem::new(
                            "shm0".to_string(),
                            "/dev/kvmfr0".to_string(),
                            "128M".to_string(),
                        )),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let chipset = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset");
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35 chipset"),
        };
        let ivshmem = q35
            .pcie_bus()
            .values()
            .find_map(|d| d.as_any().downcast_ref::<Ivshmem>())
            .expect("Ivshmem not found");

        assert_eq!(ivshmem.id(), "shm0");
        assert_eq!(ivshmem.mem_path(), "/dev/kvmfr0");
        assert_eq!(ivshmem.size(), "128M");
    }

    #[test]
    fn round_trip_scsi_storage_resource_path() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_pcie_device(
                        Some(PcieAddress::new(16, 0)),
                        Arc::new(
                            PvScsiBuilder::new()
                                .with_scsi_device(
                                    Some(ScsiAddress::new(0, 0)),
                                    Arc::new(Ssd::new("/dev/vm1/vm-108-boot".to_string())),
                                )
                                .build(),
                        ),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let chipset = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset");
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35 chipset"),
        };
        let pvscsi = q35
            .pcie_bus()
            .values()
            .find_map(|d| d.as_any().downcast_ref::<crate::runtime::PvScsi>())
            .expect("PvScsi not found");
        let ssd = pvscsi
            .scsi_bus()
            .get(&ScsiAddress::new(0, 0))
            .expect("scsi0 not found")
            .as_any()
            .downcast_ref::<Ssd>()
            .expect("downcast to Ssd");

        assert_eq!(ssd.resource, "/dev/vm1/vm-108-boot");
        assert!(!ssd.resource.is_empty());
    }

    #[test]
    fn round_trip_sata_hdd_storage_resource_path() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_sata_device(
                        Some(SataAddress::new(1, 0)),
                        Arc::new(Hdd::new("/dev/vm1/vm-108-data".to_string())),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let chipset = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset");
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35 chipset"),
        };
        let hdd = q35
            .sata_bus()
            .get(&SataAddress::new(1, 0))
            .expect("sata1 not found")
            .as_any()
            .downcast_ref::<Hdd>()
            .expect("downcast to Hdd");

        assert_eq!(hdd.resource, "/dev/vm1/vm-108-data");
        assert!(!hdd.resource.is_empty());
    }

    #[test]
    fn round_trip_ide_cdrom_storage_resource_path() {
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(4096))
            .with_chipset(Chipset::Q35(
                Q35ChipsetBuilder::new()
                    .with_ide_device(
                        Some(IdeAddress::new(0, 1)),
                        Arc::new(Cdrom::new("/mnt/iso/debian-12.iso".to_string())),
                    )
                    .build(),
            ))
            .build()
            .expect("runtime build failed");

        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        let chipset = round_tripped
            .root_devices()
            .iter()
            .find(|d| d.device_kind() == RootDeviceKind::Chipset)
            .expect("Chipset not found")
            .as_any()
            .downcast_ref::<Chipset>()
            .expect("downcast to Chipset");
        let q35 = match chipset {
            Chipset::Q35(q) => q,
            _ => panic!("expected Q35 chipset"),
        };
        let cdrom = q35
            .ide_bus()
            .get(&IdeAddress::new(0, 1))
            .expect("ide0:1 not found")
            .as_any()
            .downcast_ref::<Cdrom>()
            .expect("downcast to Cdrom");

        assert_eq!(cdrom.resource, "/mnt/iso/debian-12.iso");
        assert!(!cdrom.resource.is_empty());
    }
}
