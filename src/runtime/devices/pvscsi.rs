use std::{collections::HashMap, sync::Arc};

use derive_new::new;

use crate::runtime::{
    PciDevice, PcieAddress, PcieDevice, ScsiAddress,
    isa::{IsaAddress, IsaDevice},
    pci::PciAddress,
    scsi::ScsiDevice,
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
#[derive(Debug, new)]
pub struct PvScsi {
    scsi_bus: HashMap<ScsiAddress, Arc<dyn ScsiDevice>>,
}

impl PcieDevice for PvScsi {}
impl PciDevice for PvScsi {}
impl IsaDevice for PvScsi {}
