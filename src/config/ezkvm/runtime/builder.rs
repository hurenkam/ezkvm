use std::{collections::{BTreeMap, HashSet}, sync::Arc};

use crate::{
    config::ezkvm::{
        ConfigSchema,
        schema::{
            BiosSchema, ChipsetSchema, DeviceSchema, DisplaySchema, IdeDeviceTypeSchema,
            MemoryResourceSchema, PciDeviceTypeSchema, PcieDeviceTypeSchema, PcieResourceSchema,
            PcieStorageDeviceTypeSchema, ResourceSchema, SataDeviceTypeSchema,
            ScsiControllerTypeSchema, ScsiDeviceTypeSchema, StorageResourceSchema, TpmSchema,
            UsbDeviceTypeSchema, UsbHostIdentitySchema,
        },
    }, runtime::{
        AudioDevice, Cdrom, Chipset, EfiDisk, GenericPciDevice, GenericScsiControllerBuilder,
        GenericUsbDevice, Hdd, HostPci, IdeAddress, Ivshmem, Memory, PciAddress, PciDeviceKind,
        PcieAddress, PcieDevice, Q35ChipsetBuilder, RawArgs, Runtime, RuntimeBuilder,
        SataAddress, ScsiAddress, ScsiControllerType, SpiceDisplay, Ssd, StorageDeviceType,
        TpmState, UsbAddress, UsbDeviceKind, UsbHostIdentity, VirtioNetPcie,
        VirtioScsiSingleDisk,
    },
};
use super::error::YamlRuntimeError;
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
    controller_type: Option<ScsiControllerType>,
    scsi_disks: Vec<(ScsiAddress, StorageKind, String)>,
}

#[derive(Debug, Getters, new)]
#[allow(dead_code)]
pub struct Builder {
    schema: ConfigSchema,
}
impl Builder {
    pub fn build(&self) -> Result<Runtime, YamlRuntimeError> {
        let vm = self.schema.virtual_machine();
        let memory = Memory::new(*vm.memory().size());
        let chipset = self.build_chipset()?;

        let mut builder = RuntimeBuilder::new().with_memory(memory).with_chipset(chipset);

        if let BiosSchema::Uefi { uefi } = vm.boot().bios() {
            builder = builder.with_efidisk(EfiDisk::new(
                uefi.resource().clone(),
                uefi.efitype().clone(),
                *uefi.pre_enrolled_keys(),
                uefi.ms_cert().clone(),
                uefi.logical_size().clone().unwrap_or_default(),
                None, // block_device_size_bytes: Phase 7/9 scope only
            ));
        }

        if let Some(tpm) = vm.tpm() {
            match tpm {
                TpmSchema::Emulated { swtpm } => {
                    builder = builder.with_tpmstate(TpmState::new(
                        swtpm.resource().clone(),
                        swtpm.version().clone(),
                    ));
                }
                TpmSchema::Passthrough { .. } => {
                    return Err(YamlRuntimeError::UnsupportedTpmType {
                        tpm_type: "passthrough".to_string(),
                    });
                }
            }
        }

        if let Some(audio) = vm.audio_device() {
            builder = builder.with_audio_device(AudioDevice::new(
                audio.device_type().clone(),
                audio.driver().clone(),
            ));
        }

        if let Some(raw_args) = vm.raw_args() {
            builder = builder.with_raw_args(RawArgs(raw_args.value().to_string()));
        }

        if let Some(DisplaySchema::Spice { spice }) = self.schema.host().display() {
            builder = builder.with_spice_display(SpiceDisplay::new(
                Some(*spice.port()),
                Some(spice.listen().clone()),
                *spice.disable_ticketing(),
                *spice.gl(),
                spice.rendernode().clone(),
                *spice.clipboard(),
            ));
        }

        Ok(builder
            .build()
            .unwrap_or_else(|_| unreachable!("RuntimeBuilder::build() never fails")))
    }

    fn build_chipset(&self) -> Result<Chipset, YamlRuntimeError> {
        let vm = self.schema.virtual_machine();
        match vm.machine().chipset() {
            ChipsetSchema::Q35 { .. } => self.build_q35_chipset(),
            ChipsetSchema::I440FX { .. } => {
                if vm.devices().is_empty() {
                    Ok(Chipset::I440FX)
                } else {
                    Err(YamlRuntimeError::I440fxWithDevices)
                }
            }
        }
    }

    fn build_q35_chipset(&self) -> Result<Chipset, YamlRuntimeError> {
        let resources = self.schema.host().resources();
        let mut pvsci_by_bus: BTreeMap<u8, PvScsiControllerSpec> = BTreeMap::new();
        let mut pcie_devices: Vec<(PcieAddress, Arc<dyn crate::runtime::PcieDevice>)> = Vec::new();
        let mut sata_disks: Vec<(SataAddress, StorageKind, String)> = Vec::new();
        let mut pci_devices: Vec<(PciAddress, PciDeviceKind)> = Vec::new();
        let mut ide_devices: Vec<(IdeAddress, StorageKind, String)> = Vec::new();
        let mut usb_devices: Vec<(UsbAddress, UsbDeviceKind)> = Vec::new();

        for device in self.schema.virtual_machine().devices() {
            match device {
                DeviceSchema::Pcie { pcie } => {
                    match_pcie_device(&mut pvsci_by_bus, &mut pcie_devices, pcie, resources)?;
                }
                DeviceSchema::Scsi { scsi } => {
                    match_scsi_device(&mut pvsci_by_bus, scsi, resources)?;
                }
                DeviceSchema::Sata { sata } => {
                    match_sata_device(&mut sata_disks, sata, resources)?;
                }
                DeviceSchema::Pci { pci } => {
                    match_pci_device(&mut pci_devices, pci)?;
                }
                DeviceSchema::Usb { usb } => {
                    match_usb_device(&mut usb_devices, usb)?;
                }
                DeviceSchema::Ide { ide } => {
                    match_ide_device(&mut ide_devices, ide, resources)?;
                }
            }
        }

        sata_disks.sort_by_key(|(addr, _, _)| (*addr.port(), *addr.device()));
        pcie_devices.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        pci_devices.sort_by_key(|(addr, _)| (*addr.device(), *addr.function()));
        ide_devices.sort_by_key(|(addr, _, _)| (*addr.channel(), *addr.device()));
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

fn find_storage_resource(id: &str, resources: &[ResourceSchema]) -> Result<String, YamlRuntimeError> {
    for resource in resources {
        if let ResourceSchema::Storage { id: rid, storage } = resource {
            if rid == id {
                return Ok(match storage {
                    StorageResourceSchema::File { file } => file.clone(),
                    StorageResourceSchema::BlockDevice { block_device } => block_device.clone(),
                });
            }
        }
    }
    Err(YamlRuntimeError::ResourceNotFound { id: id.to_string() })
}

fn find_pcie_resource<'a>(
    id: &str,
    resources: &'a [ResourceSchema],
) -> Result<&'a PcieResourceSchema, YamlRuntimeError> {
    for resource in resources {
        if let ResourceSchema::PcieDevice { id: rid, pcie } = resource {
            if rid == id {
                return Ok(pcie);
            }
        }
    }
    Err(YamlRuntimeError::ResourceNotFound { id: id.to_string() })
}

fn find_memory_resource<'a>(
    id: &str,
    resources: &'a [ResourceSchema],
) -> Result<&'a MemoryResourceSchema, YamlRuntimeError> {
    for resource in resources {
        if let ResourceSchema::Memory { id: rid, memory } = resource {
            if rid == id {
                return Ok(memory);
            }
        }
    }
    Err(YamlRuntimeError::ResourceNotFound { id: id.to_string() })
}

fn insert_pvsci_controllers(pvsci_by_bus: BTreeMap<u8, PvScsiControllerSpec>, mut q35_builder: Q35ChipsetBuilder, used_pcie_addresses: &mut HashSet<PcieAddress>) -> Result<Q35ChipsetBuilder, YamlRuntimeError> {
    for (bus, mut spec) in pvsci_by_bus {
        spec.scsi_disks
            .sort_by_key(|(addr, _, _)| (*addr.target(), *addr.lun()));

        let mut pvsci_builder = GenericScsiControllerBuilder::new()
            .with_controller_type(spec.controller_type.unwrap_or(ScsiControllerType::PvScsi));
        for (scsi_address, storage_kind, path) in spec.scsi_disks {
            let storage: Arc<dyn crate::runtime::ScsiDevice> = match storage_kind {
                StorageKind::Hdd => Arc::new(Hdd::new(path)),
                StorageKind::Ssd => Arc::new(Ssd::new(path)),
                StorageKind::Cdrom => Arc::new(Cdrom::new(path)),
            };
            pvsci_builder = pvsci_builder.with_scsi_device(Some(scsi_address), storage);
        }
        let pcie_address = spec
            .pcie_address
            .unwrap_or_else(|| PcieAddress::new(bus, 0));
        if !used_pcie_addresses.insert(pcie_address) {
            return Err(YamlRuntimeError::DuplicatePcieAddress {
                device: *pcie_address.device(),
                function: *pcie_address.function(),
            });
        }

        q35_builder = q35_builder
            .with_pcie_device(Some(pcie_address), Arc::new(pvsci_builder.build()));
    }
    Ok(q35_builder)
}

fn insert_ide_devices(ide_devices: Vec<(IdeAddress, StorageKind, String)>, mut q35_builder: Q35ChipsetBuilder) -> Q35ChipsetBuilder {
    for (ide_address, storage_kind, path) in ide_devices {
        match storage_kind {
            StorageKind::Hdd => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Hdd::new(path)));
            }
            StorageKind::Ssd => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Ssd::new(path)));
            }
            StorageKind::Cdrom => {
                q35_builder = q35_builder.with_ide_device(Some(ide_address), Arc::new(Cdrom::new(path)));
            }
        }
    }
    q35_builder
}

fn insert_sata_devices(sata_disks: Vec<(SataAddress, StorageKind, String)>, mut q35_builder: Q35ChipsetBuilder) -> Q35ChipsetBuilder {
    for (sata_address, storage_kind, path) in sata_disks {
        q35_builder = match storage_kind {
            StorageKind::Hdd => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Hdd::new(path)))
            }
            StorageKind::Ssd => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Ssd::new(path)))
            }
            StorageKind::Cdrom => {
                q35_builder.with_sata_device(Some(sata_address), Arc::new(Cdrom::new(path)))
            }
        };
    }
    q35_builder
}

fn insert_pcie_devices(pcie_devices: Vec<(PcieAddress, Arc<dyn PcieDevice + 'static>)>, mut q35_builder: Q35ChipsetBuilder, mut used_pcie_addresses: HashSet<PcieAddress>) -> Result<Q35ChipsetBuilder, YamlRuntimeError> {
    for (pcie_address, pcie_device) in pcie_devices {
        if !used_pcie_addresses.insert(pcie_address) {
            return Err(YamlRuntimeError::DuplicatePcieAddress {
                device: *pcie_address.device(),
                function: *pcie_address.function(),
            });
        }
        q35_builder = q35_builder.with_pcie_device(Some(pcie_address), pcie_device);
    }
    Ok(q35_builder)
}

fn match_ide_device(
    ide_devices: &mut Vec<(IdeAddress, StorageKind, String)>,
    ide: &crate::config::ezkvm::schema::IdeDeviceSchema,
    resources: &[ResourceSchema],
) -> Result<(), YamlRuntimeError> {
    let channel = ide.bus().unwrap_or(0);
    let address = ide.address().as_ref().map(|a| a.address).unwrap_or(0);

    let (storage_kind, resource_id) = match ide.device() {
        IdeDeviceTypeSchema::Hdd { resource } => (StorageKind::Hdd, resource),
        IdeDeviceTypeSchema::Ssd { resource } => (StorageKind::Ssd, resource),
        IdeDeviceTypeSchema::Cdrom { resource } => (StorageKind::Cdrom, resource),
    };
    let path = find_storage_resource(resource_id, resources)?;

    ide_devices.push((IdeAddress::new(channel, address), storage_kind, path));
    Ok(())
}

fn match_usb_device(usb_devices: &mut Vec<(UsbAddress, UsbDeviceKind)>, usb: &crate::config::ezkvm::schema::UsbDeviceSchema) -> Result<(), YamlRuntimeError> {
    let bus = usb.bus().unwrap_or(0);
    if bus != 0 {
        return Err(YamlRuntimeError::UnsupportedUsbBus { bus });
    }
    let address = usb
        .address()
        .as_ref()
        .map(|a| UsbAddress::new(a.port.clone()))
        .unwrap_or_else(|| UsbAddress::new("1".to_string()));
    let kind = match usb.device() {
        UsbDeviceTypeSchema::NetworkController => UsbDeviceKind::NetworkController,
        UsbDeviceTypeSchema::Tablet => UsbDeviceKind::Tablet,
        UsbDeviceTypeSchema::HostPassthrough { identity } => {
            UsbDeviceKind::HostPassthrough {
                identity: match identity {
                    UsbHostIdentitySchema::BusPort { bus, port } => UsbHostIdentity::BusPort {
                        bus: bus.clone(),
                        port: port.clone(),
                    },
                    UsbHostIdentitySchema::VendorProduct { vendor_id, product_id } => {
                        UsbHostIdentity::VendorProduct {
                            vendor_id: vendor_id.clone(),
                            product_id: product_id.clone(),
                        }
                    }
                },
            }
        }
    };
    usb_devices.push((address, kind));

    Ok(())
}

fn match_pci_device(pci_devices: &mut Vec<(PciAddress, PciDeviceKind)>, pci: &crate::config::ezkvm::schema::PciDeviceSchema) -> Result<(), YamlRuntimeError> {
    let bus = pci.bus().unwrap_or(0);
    if bus != 0 {
        return Err(YamlRuntimeError::UnsupportedPcieDevice {
            device_type: format!("legacy PCI bus {} (only bus 0 supported)", bus),
        });
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
            return Err(YamlRuntimeError::UnsupportedPciPassthroughOnPciBus);
        }
    };
    pci_devices.push((address, kind));


    Ok(())
}

fn match_sata_device(sata_disks: &mut Vec<(SataAddress, StorageKind, String)>, sata: &crate::config::ezkvm::schema::SataDeviceSchema, resources: &[ResourceSchema]) -> Result<(), YamlRuntimeError> {
    let (storage_kind, resource_id) = match sata.device() {
        SataDeviceTypeSchema::Hdd { resource } => (StorageKind::Hdd, resource),
        SataDeviceTypeSchema::Ssd { resource } => (StorageKind::Ssd, resource),
        SataDeviceTypeSchema::Cdrom { resource } => (StorageKind::Cdrom, resource),
    };
    let bus = sata.bus().unwrap_or(0);
    if bus != 0 {
        return Err(YamlRuntimeError::UnsupportedSataBus { bus });
    }
    let path = find_storage_resource(resource_id, resources)?;
    let address = sata
        .address()
        .as_ref()
        .map(|a| SataAddress::new(a.address, 0))
        .unwrap_or_else(|| SataAddress::new(0, 0));
    sata_disks.push((address, storage_kind, path));

    Ok(())
}

fn match_scsi_device(pvsci_by_bus: &mut BTreeMap<u8, PvScsiControllerSpec>, scsi: &crate::config::ezkvm::schema::ScsiDeviceSchema, resources: &[ResourceSchema]) -> Result<(), YamlRuntimeError> {
    let (storage_kind, resource_id) = match scsi.device() {
        ScsiDeviceTypeSchema::Hdd { resource } => (StorageKind::Hdd, resource),
        ScsiDeviceTypeSchema::Ssd { resource } => (StorageKind::Ssd, resource),
        ScsiDeviceTypeSchema::Cdrom { resource } => (StorageKind::Cdrom, resource),
    };
    let path = find_storage_resource(resource_id, resources)?;

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
        .push((address, storage_kind, path));

    Ok(())
}

fn match_pcie_device(pvsci_by_bus: &mut BTreeMap<u8, PvScsiControllerSpec>, pcie_devices: &mut Vec<(PcieAddress, Arc<dyn PcieDevice + 'static>)>, pcie: &crate::config::ezkvm::schema::PcieDeviceSchema, resources: &[ResourceSchema]) -> Result<(), YamlRuntimeError> {
    let bus = pcie.bus().unwrap_or(0);
    let pcie_address = pcie
        .address()
        .as_ref()
        .map(|a| PcieAddress::new(a.device, a.function));

    match pcie.device() {
        PcieDeviceTypeSchema::ScsiController { controller_type } => {
            let slot = pvsci_by_bus.entry(bus).or_default();
            if pcie_address.is_some() {
                slot.pcie_address = pcie_address;
            }
            slot.controller_type = Some(match controller_type {
                ScsiControllerTypeSchema::PvScsi => ScsiControllerType::PvScsi,
                ScsiControllerTypeSchema::VirtioScsiPci => ScsiControllerType::VirtioScsiPci,
            });
        }
        PcieDeviceTypeSchema::VirtioNet {
            resource,
            mac_address,
            rx_queue_size,
            tx_queue_size,
            vhost,
        } => {
            if bus != 0 {
                return Err(YamlRuntimeError::UnsupportedPcieDevice {
                    device_type: format!("virtio_net on pcie bus {} (only bus 0 supported)", bus),
                });
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
        PcieDeviceTypeSchema::VirtioScsiSingle { resource, index, storage_type } => {
            if bus != 0 {
                return Err(YamlRuntimeError::UnsupportedPcieDevice {
                    device_type: format!(
                        "virtio_scsi_single on pcie bus {} (only bus 0 supported)",
                        bus
                    ),
                });
            }

            let path = find_storage_resource(resource, resources)?;
            let address = pcie_address.unwrap_or_else(|| PcieAddress::new(0, 0));
            let storage_type = match storage_type {
                PcieStorageDeviceTypeSchema::Hdd => StorageDeviceType::Hdd,
                PcieStorageDeviceTypeSchema::Ssd => StorageDeviceType::Ssd,
                PcieStorageDeviceTypeSchema::Cdrom => StorageDeviceType::Odd,
            };
            pcie_devices.push((
                address,
                Arc::new(VirtioScsiSingleDisk::new(path, storage_type, *index)),
            ));
        }
        PcieDeviceTypeSchema::HostPci { resource, x_vga } => {
            let pcie_resource = find_pcie_resource(resource, resources)?;
            match pcie_resource {
                PcieResourceSchema::HostAddress { address, functions, rombar, romfile } => {
                    let addr = pcie_address.unwrap_or_else(|| PcieAddress::new(0, 0));
                    let host_pci = Arc::new(HostPci::new(
                        address.clone(),
                        functions.clone(),
                        true,
                        *x_vga,
                        *rombar,
                        romfile.clone(),
                    ));
                    pcie_devices.push((addr, host_pci));
                }
                PcieResourceSchema::Address { .. } => {
                    return Err(YamlRuntimeError::ResourceNotFound { id: resource.clone() });
                }
            }
        }
        PcieDeviceTypeSchema::IvshmemPlain { resource } => {
            let memory = find_memory_resource(resource, resources)?;
            let addr = pcie_address.unwrap_or_else(|| PcieAddress::new(0, 0));
            let ivshmem = Arc::new(Ivshmem::new(resource.clone(), memory.path.clone(), memory.size.clone()));
            pcie_devices.push((addr, ivshmem));
        }
        other => {
            return Err(YamlRuntimeError::UnsupportedPcieDevice {
                device_type: format!("{:?}", other),
            });
        }
    }
    Ok(())
}

impl TryFrom<ConfigSchema> for Runtime {
    type Error = YamlRuntimeError;

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
        PcieDeviceSchema, PcieDeviceTypeSchema, Q35ChipsetSchema, ResourceSchema,
        SataAddressSchema, SataDeviceSchema, SataDeviceTypeSchema, ScsiControllerTypeSchema,
        StorageResourceSchema, UsbAddressSchema, UsbDeviceSchema, UsbDeviceTypeSchema,
        VirtualMachineSchema,
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
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        None,
                        PcieDeviceTypeSchema::ScsiController {
                            controller_type: ScsiControllerTypeSchema::PvScsi,
                        },
                    ),
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
            HostSchema::new(
                None,
                None,
                vec![ResourceSchema::Storage {
                    id: "storage0".to_string(),
                    storage: StorageResourceSchema::File { file: "storage0".to_string() },
                }],
            ),
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
            HostSchema::new(
                None,
                None,
                vec![
                    ResourceSchema::Storage {
                        id: "iso0".to_string(),
                        storage: StorageResourceSchema::File { file: "iso0".to_string() },
                    },
                    ResourceSchema::Storage {
                        id: "storage0".to_string(),
                        storage: StorageResourceSchema::File { file: "storage0".to_string() },
                    },
                ],
            ),
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

    #[test]
    fn missing_resource_id_returns_typed_error() {
        // Q35 chipset referencing a HostPci resource ("hostpci99") that does not exist in
        // host.resources() — the conversion must fail with a typed ResourceNotFound error,
        // not panic or silently substitute an empty path.
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
                vec![DeviceSchema::Pcie {
                    pcie: PcieDeviceSchema::new(
                        Some(0),
                        None,
                        PcieDeviceTypeSchema::HostPci {
                            resource: "hostpci99".to_string(),
                            x_vga: false,
                        },
                    ),
                }],
            ),
        );

        let result = crate::runtime::Runtime::try_from(schema);
        match result {
            Err(crate::config::ezkvm::runtime::YamlRuntimeError::ResourceNotFound { id }) => {
                assert_eq!(id, "hostpci99");
            }
            other => panic!("expected ResourceNotFound {{ id: \"hostpci99\" }}, got {other:?}"),
        }
    }
}
