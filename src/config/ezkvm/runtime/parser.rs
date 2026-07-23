use crate::{
    config::ezkvm::{
        ConfigSchema,
        schema::{
            BootSchema, ChipsetSchema, ConfigSchema as SchemaConfig, DeviceSchema, HostSchema,
            I440FXChipsetSchema, IdeAddressSchema, IdeDeviceSchema, IdeDeviceTypeSchema,
            MachineSchema, MemorySchema, MetadataSchema, PciAddressSchema, PciDeviceSchema,
            PciDeviceTypeSchema, PcieAddressSchema, PcieDeviceSchema, PcieDeviceTypeSchema,
            Q35ChipsetSchema, SataAddressSchema, SataDeviceSchema, SataDeviceTypeSchema,
            ScsiAddressSchema, ScsiDeviceSchema, ScsiDeviceTypeSchema, UsbAddressSchema,
            UsbDeviceSchema, UsbDeviceTypeSchema, VirtualMachineSchema,
            EZKVM_CONFIG_SCHEMA_VERSION,
        },
    },
    runtime::{
        Chipset, GenericPciDevice, GenericUsbDevice, Memory, PciBusDeviceKind, PciDeviceKind,
        PcieBusDeviceKind, PvScsi, Q35Chipset, RootDeviceKind, Runtime, StorageDeviceType,
        UsbBusDeviceKind, UsbDeviceKind, VirtioNetPcie,
    },
};
use derive_getters::Getters;
use derive_new::new;

#[derive(Debug, Getters, new)]
#[allow(dead_code)]
pub struct Parser {
    runtime: Runtime,
}
impl Parser {
    pub fn parse(&self) -> Result<ConfigSchema, String> {
        let mut memory: Option<MemorySchema> = None;
        let mut chipset: Option<(ChipsetSchema, Vec<DeviceSchema>)> = None;

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
                _ => {} // device kinds not yet handled by ezkvm schema conversion
            }
        }

        let memory = memory.ok_or_else(|| "runtime missing memory root device".to_string())?;
        let (chipset, devices) =
            chipset.ok_or_else(|| "runtime missing chipset root device".to_string())?;

        let metadata = MetadataSchema::new(EZKVM_CONFIG_SCHEMA_VERSION.to_string(), "vm".to_string());
        let host = HostSchema::new(None, None, vec![]);
        let machine = MachineSchema::new(chipset, None);
        let virtual_machine = VirtualMachineSchema::new(
            machine,
            None,
            memory,
            BootSchema::default(),
            None,
            None,
            None,
            None,
            None,
            None,
            devices,
        );

        Ok(SchemaConfig::new(metadata, host, virtual_machine))
    }

    fn parse_chipset(&self, chipset: &Chipset) -> Result<(ChipsetSchema, Vec<DeviceSchema>), String> {
        match chipset {
            Chipset::I440FX => Ok((
                ChipsetSchema::I440FX {
                    i440fx: I440FXChipsetSchema::new(None),
                },
                vec![],
            )),
            Chipset::Q35(q35) => {
                let devices = self.parse_q35_devices(q35)?;
                Ok((
                    ChipsetSchema::Q35 {
                        q35: Q35ChipsetSchema::new(None),
                    },
                    devices,
                ))
            }
        }
    }

    fn parse_q35_devices(&self, q35: &Q35Chipset) -> Result<Vec<DeviceSchema>, String> {
        let mut devices = Vec::new();

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
                    let scsi_type = match scsi_device.device_kind() {
                        StorageDeviceType::Hdd => ScsiDeviceTypeSchema::Hdd {
                            resource: format!("scsi-{}-{}", scsi_addr.target(), scsi_addr.lun()),
                        },
                        StorageDeviceType::Ssd => ScsiDeviceTypeSchema::Ssd {
                            resource: format!("scsi-{}-{}", scsi_addr.target(), scsi_addr.lun()),
                        },
                        StorageDeviceType::Odd => ScsiDeviceTypeSchema::Cdrom {
                            resource: format!("scsi-{}-{}", scsi_addr.target(), scsi_addr.lun()),
                        },
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

            return Err("unsupported q35 pcie device in runtime parser".to_string());
        }

        let mut sata_entries = q35.sata_bus().iter().collect::<Vec<_>>();
        sata_entries.sort_by_key(|(addr, _)| (*addr.port(), *addr.device()));
        for (sata_addr, sata_device) in sata_entries {
            let sata_type = match sata_device.device_kind() {
                StorageDeviceType::Hdd => SataDeviceTypeSchema::Hdd {
                    resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()),
                },
                StorageDeviceType::Ssd => SataDeviceTypeSchema::Ssd {
                    resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()),
                },
                StorageDeviceType::Odd => SataDeviceTypeSchema::Cdrom {
                    resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()),
                },
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
                return Err("unsupported q35 pci device in runtime parser".to_string());
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
            let ide_type = match ide_device.storage_options().device_type {
                StorageDeviceType::Hdd => IdeDeviceTypeSchema::Hdd {
                    resource: format!("ide-{}-{}", ide_addr.channel(), ide_addr.device()),
                },
                StorageDeviceType::Ssd => IdeDeviceTypeSchema::Ssd {
                    resource: format!("ide-{}-{}", ide_addr.channel(), ide_addr.device()),
                },
                StorageDeviceType::Odd => IdeDeviceTypeSchema::Cdrom {
                    resource: format!("ide-{}-{}", ide_addr.channel(), ide_addr.device()),
                },
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
                return Err("unsupported q35 usb device in runtime parser".to_string());
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

        Ok(devices)
    }
}

impl TryFrom<Runtime> for ConfigSchema {
    type Error = String;

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let parser = Parser::new(value);
        parser.parse()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::runtime::{
        Cdrom, Chipset, GenericPciDevice, GenericUsbDevice, Hdd, IdeAddress, Memory, PciAddress,
        PciBusDeviceKind, PciDeviceKind, PcieAddress, PcieBusDeviceKind, PvScsiBuilder,
        Q35ChipsetBuilder, RootDeviceKind, Runtime, RuntimeBuilder, SataAddress, ScsiAddress,
        Ssd, UsbAddress, UsbDeviceKind, VirtioNetPcie,
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
}
