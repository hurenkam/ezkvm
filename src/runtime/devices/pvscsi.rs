use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::runtime::{
    PciDevice, PcieAddress, PcieDevice, ScsiAddress,
    isa::{IsaAddress, IsaDevice},
    pci::PciAddress,
    scsi::ScsiDevice,
    storage::StorageDeviceType,
};

#[allow(dead_code)]
#[derive(Debug)]
enum Address {
    Pcie(PcieAddress),
    Pci(PciAddress),
    Isa(IsaAddress),
}

pub struct PvScsiBuilder {
    scsi_bus: HashMap<ScsiAddress, Arc<dyn ScsiDevice>>,
}
impl PvScsiBuilder {
    pub fn new() -> Self {
        Self {
            scsi_bus: HashMap::new(),
        }
    }
    pub fn build(self) -> PvScsi {
        PvScsi::new(self.scsi_bus)
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
pub struct PvScsi {
    scsi_bus: HashMap<ScsiAddress, Arc<dyn ScsiDevice>>,
}

impl PciDevice for PvScsi {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PciBusDeviceKind {
        crate::runtime::PciBusDeviceKind::PvScsi
    }
}
impl IsaDevice for PvScsi {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::isa::IsaBusDeviceKind {
        crate::runtime::isa::IsaBusDeviceKind::PvScsi
    }
}

impl PcieDevice for PvScsi {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
        crate::runtime::PcieBusDeviceKind::PvScsi
    }
}

impl std::fmt::Display for PvScsi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PvScsi Controller")?;
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
