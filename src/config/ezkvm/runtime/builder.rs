use std::{collections::{BTreeMap, HashSet}, sync::Arc};

use crate::{
    config::ezkvm::{
        ConfigSchema,
        schema::{
            ChipsetSchema, DeviceSchema, IdeDeviceTypeSchema, PciDeviceTypeSchema, PcieDeviceTypeSchema,
            SataDeviceTypeSchema, ScsiDeviceTypeSchema, UsbDeviceTypeSchema,
        },
    }, runtime::{
        Cdrom, Chipset, GenericPciDevice, GenericUsbDevice, Hdd, IdeAddress, Memory, PciAddress, PciDeviceKind, PcieAddress, PcieDevice, PvScsiBuilder, Q35ChipsetBuilder, Runtime, RuntimeBuilder, SataAddress, ScsiAddress, Ssd, UsbAddress, UsbDeviceKind, VirtioNetPcie,
    },
};
use derive_getters::Getters;
use derive_new::new;

#[derive(Clone, Copy)]
enum StorageKind {
    Hdd,
    Ssd,
    Cdrom,
}

#[derive(Default)]
struct PvScsiControllerSpec {
    pcie_address: Option<PcieAddress>,
    scsi_disks: Vec<(ScsiAddress, StorageKind)>,
}

#[derive(Debug, Getters, new)]
#[allow(dead_code)]
pub struct Builder {
    schema: ConfigSchema,
}
impl Builder {
    pub fn build(&self) -> Result<Runtime, String> {
        let vm = self.schema.virtual_machine();
        let memory = Memory::new(*vm.memory().size());
        let chipset = self.build_chipset()?;

        RuntimeBuilder::new()
            .with_memory(memory)
            .with_chipset(chipset)
            .build()
            .map_err(|_| "failed to build runtime".to_string())
    }

    fn build_chipset(&self) -> Result<Chipset, String> {
        let vm = self.schema.virtual_machine();
        match vm.machine().chipset() {
            ChipsetSchema::Q35 { .. } => self.build_q35_chipset(),
            ChipsetSchema::I440FX { .. } => {
                if vm.devices().is_empty() {
                    Ok(Chipset::I440FX)
                } else {
                    Err("i440fx conversion currently supports no attached devices".to_string())
                }
            }
        }
    }

    fn build_q35_chipset(&self) -> Result<Chipset, String> {
        let mut pvsci_by_bus: BTreeMap<u8, PvScsiControllerSpec> = BTreeMap::new();
        let mut pcie_devices: Vec<(PcieAddress, Arc<dyn crate::runtime::PcieDevice>)> = Vec::new();
        let mut sata_disks: Vec<(SataAddress, StorageKind)> = Vec::new();
        let mut pci_devices: Vec<(PciAddress, PciDeviceKind)> = Vec::new();
        let mut ide_devices: Vec<(IdeAddress, StorageKind)> = Vec::new();
        let mut usb_devices: Vec<(UsbAddress, UsbDeviceKind)> = Vec::new();

        for device in self.schema.virtual_machine().devices() {
            match device {
                DeviceSchema::Pcie { pcie } => {
                    match_pcie_device(&mut pvsci_by_bus, &mut pcie_devices, pcie)?;
                }
                DeviceSchema::Scsi { scsi } => {
                    match_scsi_device(&mut pvsci_by_bus, scsi);
                }
                DeviceSchema::Sata { sata } => {
                    match_sata_device(&mut sata_disks, sata)?;
                }
                DeviceSchema::Pci { pci } => {
                    match_pci_device(&mut pci_devices, pci)?;
                }
                DeviceSchema::Usb { usb } => {
                    match_usb_device(&mut usb_devices, usb)?;
                }
                DeviceSchema::Ide { ide } => {
                    match_ide_device(&mut ide_devices, ide);
                }
            }
        }

        sata_disks.sort_by_key(|(addr, _)| (*addr.port(), *addr.device()));
        pcie_devices.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        pci_devices.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        ide_devices.sort_by_key(|(addr, _)| (*addr.channel(), *addr.device()));
        usb_devices.sort_by_key(|(addr, _)| addr.port().clone());

        let mut q35_builder = Q35ChipsetBuilder::new();
        let mut used_pcie_addresses: HashSet<PcieAddress> = HashSet::new();
        q35_builder = insert_pvsci_controllers(pvsci_by_bus, q35_builder, &mut used_pcie_addresses)?;
        q35_builder = insert_pcie_devices(pcie_devices, q35_builder, used_pcie_addresses)?;
        for (pci_address, kind) in pci_devices {
            q35_builder = q35_builder
                .with_pci_device(Some(pci_address), Arc::new(GenericPciDevice::new(kind)));
        }
        q35_builder = insert_sata_devices(sata_disks, q35_builder);
        q35_builder = insert_ide_devices(ide_devices, q35_builder);
        for (usb_address, kind) in usb_devices {
            q35_builder = q35_builder
                .with_usb_device(Some(usb_address), Arc::new(GenericUsbDevice::new(kind)));
        }

        Ok(Chipset::Q35(q35_builder.build()))
    }
}

fn insert_pvsci_controllers(pvsci_by_bus: BTreeMap<u8, PvScsiControllerSpec>, mut q35_builder: Q35ChipsetBuilder, used_pcie_addresses: &mut HashSet<PcieAddress>) -> Result<Q35ChipsetBuilder, String> {
    for (bus, mut spec) in pvsci_by_bus {
        spec.scsi_disks
            .sort_by_key(|(addr, _)| (*addr.target(), *addr.lun()));

        let mut pvsci_builder = PvScsiBuilder::new();
        for (scsi_address, storage_kind) in spec.scsi_disks {
            let storage: Arc<dyn crate::runtime::ScsiDevice> = match storage_kind {
                StorageKind::Hdd => Arc::new(Hdd::new(String::new())),
                StorageKind::Ssd => Arc::new(Ssd::new(String::new())),
                StorageKind::Cdrom => Arc::new(Cdrom::new(String::new())),
            };
            pvsci_builder = pvsci_builder.with_scsi_device(Some(scsi_address), storage);
        }
        let pcie_address = spec
            .pcie_address
            .unwrap_or_else(|| PcieAddress::new(bus, 0));
        if !used_pcie_addresses.insert(pcie_address) {
            return Err(format!(
                "duplicate pcie address during runtime conversion: {}:{}",
                pcie_address.device(),
                pcie_address.function()
            ));
        }

        q35_builder = q35_builder
            .with_pcie_device(Some(pcie_address), Arc::new(pvsci_builder.build()));
    }
    Ok(q35_builder)
}

fn insert_ide_devices(ide_devices: Vec<(IdeAddress, StorageKind)>, mut q35_builder: Q35ChipsetBuilder) -> Q35ChipsetBuilder {
    for (ide_address, storage_kind) in ide_devices {
        match storage_kind {
            StorageKind::Hdd => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Hdd::new(String::new())));
            }
            StorageKind::Ssd => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Ssd::new(String::new())));
            }
            StorageKind::Cdrom => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Cdrom::new(String::new())));
            }
        }
    }
    q35_builder
}

fn insert_sata_devices(sata_disks: Vec<(SataAddress, StorageKind)>, mut q35_builder: Q35ChipsetBuilder) -> Q35ChipsetBuilder {
    for (sata_address, storage_kind) in sata_disks {
        q35_builder = match storage_kind {
            StorageKind::Hdd => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Hdd::new(String::new())))
            }
            StorageKind::Ssd => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Ssd::new(String::new())))
            }
            StorageKind::Cdrom => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Cdrom::new(String::new())))
            }
        };
    }
    q35_builder
}

fn insert_pcie_devices(pcie_devices: Vec<(PcieAddress, Arc<dyn PcieDevice + 'static>)>, mut q35_builder: Q35ChipsetBuilder, mut used_pcie_addresses: HashSet<PcieAddress>) -> Result<Q35ChipsetBuilder, String> {
    for (pcie_address, pcie_device) in pcie_devices {
        if !used_pcie_addresses.insert(pcie_address) {
            return Err(format!(
                "duplicate pcie address during runtime conversion: {}:{}",
                pcie_address.device(),
                pcie_address.function()
            ));
        }
        q35_builder = q35_builder.with_pcie_device(Some(pcie_address), pcie_device);
    }
    Ok(q35_builder)
}

fn match_ide_device(ide_devices: &mut Vec<(IdeAddress, StorageKind)>, ide: &crate::config::ezkvm::schema::IdeDeviceSchema) {
    let channel = ide.bus().unwrap_or(0);
    let address = ide.address().as_ref().map(|a| a.address).unwrap_or(0);

    let storage_kind = match ide.device() {
        IdeDeviceTypeSchema::Hdd { .. } => StorageKind::Hdd,
        IdeDeviceTypeSchema::Ssd { .. } => StorageKind::Ssd,
        IdeDeviceTypeSchema::Cdrom { .. } => StorageKind::Cdrom,
    };

    ide_devices.push((IdeAddress::new(channel, address), storage_kind));
}

fn match_usb_device(usb_devices: &mut Vec<(UsbAddress, UsbDeviceKind)>, usb: &crate::config::ezkvm::schema::UsbDeviceSchema) -> Result<(), String> {
    let bus = usb.bus().unwrap_or(0);
    if bus != 0 {
        return Err(format!("unsupported usb bus {} for runtime conversion", bus));
    }
    let address = usb
        .address()
        .as_ref()
        .map(|a| UsbAddress::new(a.port.clone()))
        .unwrap_or_else(|| UsbAddress::new("1".to_string()));
    let kind = match usb.device() {
        UsbDeviceTypeSchema::NetworkController => UsbDeviceKind::NetworkController,
        UsbDeviceTypeSchema::Tablet => UsbDeviceKind::Tablet,
        UsbDeviceTypeSchema::HostPassthrough { resource } => {
            UsbDeviceKind::HostPassthrough {
                resource: resource.clone(),
            }
        }
    };
    usb_devices.push((address, kind));

    Ok(())
}

fn match_pci_device(pci_devices: &mut Vec<(PciAddress, PciDeviceKind)>, pci: &crate::config::ezkvm::schema::PciDeviceSchema) -> Result<(), String> {
    let bus = pci.bus().unwrap_or(0);
    if bus != 0 {
        return Err(format!("unsupported pci bus {} for runtime conversion", bus));
    }
    let address = pci
        .address()
        .as_ref()
        .map(|a| PciAddress::new(a.device, a.function))
        .unwrap_or_else(|| PciAddress::new(0, 0));
    let kind = match pci.device() {
        PciDeviceTypeSchema::NetworkController => PciDeviceKind::NetworkController,
        PciDeviceTypeSchema::QxlGpu => PciDeviceKind::QxlGpu,
        PciDeviceTypeSchema::Ac97 => PciDeviceKind::Ac97,
        PciDeviceTypeSchema::HostPci { .. } => {
            return Err("HostPci passthrough on PCI bus is not yet supported in runtime conversion".to_string());
        }
    };
    pci_devices.push((address, kind));


    Ok(())
}

fn match_sata_device(sata_disks: &mut Vec<(SataAddress, StorageKind)>, sata: &crate::config::ezkvm::schema::SataDeviceSchema) -> Result<(), String> {
    let storage_kind = match sata.device() {
        SataDeviceTypeSchema::Hdd { .. } => StorageKind::Hdd,
        SataDeviceTypeSchema::Ssd { .. } => StorageKind::Ssd,
        SataDeviceTypeSchema::Cdrom { .. } => StorageKind::Cdrom,
    };
    let bus = sata.bus().unwrap_or(0);
    if bus != 0 {
        return Err(format!(
            "unsupported sata bus {} for runtime conversion",
            bus
        ));
    }
    let address = sata
        .address()
        .as_ref()
        .map(|a| SataAddress::new(a.address, 0))
        .unwrap_or_else(|| SataAddress::new(0, 0));
    sata_disks.push((address, storage_kind));

    Ok(())
}

fn match_scsi_device(pvsci_by_bus: &mut BTreeMap<u8, PvScsiControllerSpec>, scsi: &crate::config::ezkvm::schema::ScsiDeviceSchema) {
    let storage_kind = match scsi.device() {
        ScsiDeviceTypeSchema::Hdd { .. } => StorageKind::Hdd,
        ScsiDeviceTypeSchema::Ssd { .. } => StorageKind::Ssd,
        ScsiDeviceTypeSchema::Cdrom { .. } => StorageKind::Cdrom,
    };

    let bus = scsi.bus().unwrap_or(0);
    let address = scsi
        .address()
        .as_ref()
        .map(|a| ScsiAddress::new(a.target, a.lun))
        .unwrap_or_else(|| ScsiAddress::new(0, 0));
    pvsci_by_bus
        .entry(bus)
        .or_default()
        .scsi_disks
        .push((address, storage_kind));
}

fn match_pcie_device(pvsci_by_bus: &mut BTreeMap<u8, PvScsiControllerSpec>, pcie_devices: &mut Vec<(PcieAddress, Arc<dyn PcieDevice + 'static>)>, pcie: &crate::config::ezkvm::schema::PcieDeviceSchema) -> Result<(), String> {
    let bus = pcie.bus().unwrap_or(0);
    let pcie_address = pcie
        .address()
        .as_ref()
        .map(|a| PcieAddress::new(a.device, a.function));

    match pcie.device() {
        PcieDeviceTypeSchema::PvScsi => {
            let slot = pvsci_by_bus.entry(bus).or_default();
            if pcie_address.is_some() {
                slot.pcie_address = pcie_address;
            }
        }
        PcieDeviceTypeSchema::VirtioNet {
            resource,
            mac_address,
            rx_queue_size,
            tx_queue_size,
            vhost,
        } => {
            if bus != 0 {
                return Err(format!(
                    "unsupported pcie bus {} for virtio_net runtime conversion",
                    bus
                ));
            }

            let address = pcie_address.unwrap_or_else(|| PcieAddress::new(0, 0));
            let nic = Arc::new(VirtioNetPcie::new(
                resource.clone(),
                mac_address.clone(),
                *rx_queue_size,
                *tx_queue_size,
                *vhost,
            ));
            pcie_devices.push((address, nic));
        }
        _ => {
            return Err(format!(
                "unsupported pcie device for runtime conversion: {:?}",
                pcie.device()
            ));
        }
    }
    Ok(())
}

impl TryFrom<ConfigSchema> for Runtime {
    type Error = String;

    fn try_from(value: ConfigSchema) -> Result<Self, Self::Error> {
        let builder = Builder::new(value);
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::ezkvm::schema::{
        BootSchema, ChipsetSchema, ConfigSchema, DeviceSchema, HostSchema, I440FXChipsetSchema,
        IdeAddressSchema, IdeDeviceSchema, IdeDeviceTypeSchema, MachineSchema, MemorySchema,
        MetadataSchema, PciAddressSchema, PciDeviceSchema, PciDeviceTypeSchema, PcieAddressSchema,
        PcieDeviceSchema, PcieDeviceTypeSchema, Q35ChipsetSchema, SataAddressSchema,
        SataDeviceSchema, SataDeviceTypeSchema, UsbAddressSchema, UsbDeviceSchema,
        UsbDeviceTypeSchema, VirtualMachineSchema,
    };

    #[test]
    fn i440fx_with_devices_is_rejected() {
        let schema = ConfigSchema::new(
            MetadataSchema::new("1.0.0".to_string(), "vm".to_string()),
            HostSchema::new(None, None, vec![]),
            VirtualMachineSchema::new(
                MachineSchema::new(
                    ChipsetSchema::I440FX {
                        i440fx: I440FXChipsetSchema::new(None),
                    },
                    None,
                ),
                None,
                MemorySchema::new(1024, None, false),
                BootSchema::default(),
                None,
                None,
                None,
                None,
                None,
                None,
                vec![crate::config::ezkvm::schema::DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(Some(0), None, PcieDeviceTypeSchema::PvScsi),
                }],
            ),
        );

        let result = crate::runtime::Runtime::try_from(schema);
        assert!(result.is_err());
    }

    #[test]
    fn sata_hdd_is_supported() {
        let schema = ConfigSchema::new(
            MetadataSchema::new("1.0.0".to_string(), "vm".to_string()),
            HostSchema::new(None, None, vec![]),
            VirtualMachineSchema::new(
                MachineSchema::new(
                    ChipsetSchema::Q35 {
                        q35: crate::config::ezkvm::schema::Q35ChipsetSchema::new(None),
                    },
                    None,
                ),
                None,
                MemorySchema::new(1024, None, false),
                BootSchema::default(),
                None,
                None,
                None,
                None,
                None,
                None,
                vec![crate::config::ezkvm::schema::DeviceSchema::Sata {
                    sata: SataDeviceSchema::new(
                        Some(0),
                        Some(SataAddressSchema::new(1)),
                        SataDeviceTypeSchema::Hdd {
                            resource: "storage0".to_string(),
                        },
                    ),
                }],
            ),
        );

        let result = crate::runtime::Runtime::try_from(schema);
        assert!(result.is_ok());
    }

    #[test]
    fn q35_extended_device_types_are_supported() {
        let schema = ConfigSchema::new(
            MetadataSchema::new("1.0.0".to_string(), "vm".to_string()),
            HostSchema::new(None, None, vec![]),
            VirtualMachineSchema::new(
                MachineSchema::new(
                    ChipsetSchema::Q35 {
                        q35: Q35ChipsetSchema::new(None),
                    },
                    None,
                ),
                None,
                MemorySchema::new(1024, None, false),
                BootSchema::default(),
                None,
                None,
                None,
                None,
                None,
                None,
                vec![
                    DeviceSchema::Pcie {
                        pcie: PcieDeviceSchema::new(
                            Some(0),
                            Some(PcieAddressSchema::new(1, 0)),
                            PcieDeviceTypeSchema::VirtioNet {
                                resource: Some("net0".to_string()),
                                mac_address: Some("de:ad:be:ef:00:01".to_string()),
                                rx_queue_size: Some(256),
                                tx_queue_size: Some(256),
                                vhost: Some(true),
                            },
                        ),
                    },
                    DeviceSchema::Pci {
                        pci: PciDeviceSchema::new(
                            Some(0),
                            Some(PciAddressSchema::new(2, 0)),
                            PciDeviceTypeSchema::QxlGpu,
                        ),
                    },
                    DeviceSchema::Usb {
                        usb: UsbDeviceSchema::new(
                            Some(0),
                            Some(UsbAddressSchema::new("2.1".to_string())),
                            UsbDeviceTypeSchema::Tablet,
                        ),
                    },
                    DeviceSchema::Ide {
                        ide: IdeDeviceSchema::new(
                            Some(0),
                            Some(IdeAddressSchema::new(1)),
                            IdeDeviceTypeSchema::Cdrom {
                                resource: "iso0".to_string(),
                            },
                        ),
                    },
                    DeviceSchema::Sata {
                        sata: SataDeviceSchema::new(
                            Some(0),
                            Some(SataAddressSchema::new(3)),
                            SataDeviceTypeSchema::Hdd {
                                resource: "storage0".to_string(),
                            },
                        ),
                    },
                ],
            ),
        );

        let result = crate::runtime::Runtime::try_from(schema);
        assert!(result.is_ok());
    }
}
