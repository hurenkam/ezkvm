use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::runtime::{
    PciDevice, PcieAddress, PcieDevice, ScsiAddress,
    isa::{IsaAddress, IsaDevice},
    pci::PciAddress,
    scsi::ScsiDevice,
    storage::{StorageDevice, StorageDeviceType, StorageOptions},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScsiControllerType {
    PvScsi,
    VirtioScsiPci,
}

#[allow(dead_code)]
#[derive(Debug)]
enum Address {
    Pcie(PcieAddress),
    Pci(PciAddress),
    Isa(IsaAddress),
}

pub struct GenericScsiControllerBuilder {
    controller_type: ScsiControllerType,
    scsi_bus: HashMap<ScsiAddress, Arc<dyn ScsiDevice>>,
}
impl GenericScsiControllerBuilder {
    pub fn new() -> Self {
        Self {
            controller_type: ScsiControllerType::PvScsi,
            scsi_bus: HashMap::new(),
        }
    }

    pub fn build(self) -> GenericScsiController {
        GenericScsiController::new(self.controller_type, self.scsi_bus)
    }

    pub fn with_controller_type(mut self, controller_type: ScsiControllerType) -> Self {
        self.controller_type = controller_type;
        self
    }

    pub fn with_scsi_device(
        mut self,
        address: Option<ScsiAddress>,
        device: Arc<dyn ScsiDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => ScsiAddress::new(0, 0),
        };
        self.scsi_bus.insert(address, device);
        self
    }
}

#[allow(dead_code)]
#[derive(Debug, Getters, new)]
pub struct GenericScsiController {
    controller_type: ScsiControllerType,
    scsi_bus: HashMap<ScsiAddress, Arc<dyn ScsiDevice>>,
}

impl PciDevice for GenericScsiController {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PciBusDeviceKind {
        crate::runtime::PciBusDeviceKind::PvScsi
    }
}
impl IsaDevice for GenericScsiController {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::isa::IsaBusDeviceKind {
        crate::runtime::isa::IsaBusDeviceKind::PvScsi
    }
}

impl PcieDevice for GenericScsiController {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
        crate::runtime::PcieBusDeviceKind::ScsiController
    }
}

impl std::fmt::Display for GenericScsiController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} Controller",
            match self.controller_type {
                ScsiControllerType::PvScsi => "PvScsi",
                ScsiControllerType::VirtioScsiPci => "VirtioScsiPci",
            }
        )?;
        if self.scsi_bus.is_empty() {
            return Ok(());
        }

        writeln!(f, "  +-scsi:")?;
        let mut entries: Vec<_> = self.scsi_bus.iter().collect();
        entries.sort_by_key(|(address, _)| (*address.target(), *address.lun()));

        for (address, device) in entries {
            writeln!(f, "      +-{}: {}", address, format_storage_device(device.as_ref()))?;
        }
        Ok(())
    }
}

fn format_storage_device(device: &dyn ScsiDevice) -> &'static str {
    match device.storage_options().device_type {
        StorageDeviceType::Ssd => "Ssd",
        StorageDeviceType::Hdd => "Hdd",
        StorageDeviceType::Odd => "Cdrom",
    }
}

#[derive(Debug, Getters, new)]
pub struct VirtioScsiSingleDisk {
    resource: String,
    storage_type: StorageDeviceType,
    index: u8,
}

impl PcieDevice for VirtioScsiSingleDisk {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
        crate::runtime::PcieBusDeviceKind::VirtioScsiSingleDisk
    }
}

impl ScsiDevice for VirtioScsiSingleDisk {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl StorageDevice for VirtioScsiSingleDisk {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: self.storage_type,
            read_only: false,
            cache_mode: None,
        }
    }
}

impl std::fmt::Display for VirtioScsiSingleDisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "VirtioScsiSingleDisk(index={}, type={}, resource={})",
            self.index,
            match self.storage_type {
                StorageDeviceType::Ssd => "Ssd",
                StorageDeviceType::Hdd => "Hdd",
                StorageDeviceType::Odd => "Cdrom",
            },
            self.resource
        )
    }
}
