use std::{collections::HashMap, sync::Arc};

use derive_getters::Getters;
use derive_new::new;

use crate::runtime::{
    IdeAddress, IdeDevice, PciDevice, PcieAddress, PcieDevice, SataAddress, SataDevice,
    pci::PciAddress,
};

pub struct Q35ChipsetBuilder {
    pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>,
    pci_bus: HashMap<PciAddress, Arc<dyn PciDevice>>,
    sata_bus: HashMap<SataAddress, Arc<dyn SataDevice>>,
    ide_bus: HashMap<IdeAddress, Arc<dyn IdeDevice>>,
}
#[allow(dead_code)]
impl Q35ChipsetBuilder {
    pub fn new() -> Self {
        Self {
            pcie_bus: HashMap::new(),
            pci_bus: HashMap::new(),
            sata_bus: HashMap::new(),
            ide_bus: HashMap::new(),
        }
    }
    pub fn with_pcie_device(
        mut self,
        address: Option<PcieAddress>,
        device: Arc<dyn PcieDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => PcieAddress::new(0, 0),
        };
        self.pcie_bus.insert(address, device);
        self
    }
    pub fn with_pci_device(
        mut self,
        address: Option<PciAddress>,
        device: Arc<dyn PciDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => PciAddress::new(0, 0),
        };
        self.pci_bus.insert(address, device);
        self
    }
    pub fn with_sata_device(
        mut self,
        address: Option<SataAddress>,
        device: Arc<dyn SataDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => SataAddress::new(0, 0),
        };
        self.sata_bus.insert(address, device);
        self
    }
    pub fn with_ide_device(
        mut self,
        address: Option<IdeAddress>,
        device: Arc<dyn IdeDevice>,
    ) -> Self {
        let address = match address {
            Some(address) => address,
            None => IdeAddress::new(0, 0),
        };
        self.ide_bus.insert(address, device);
        self
    }
    pub fn build(self) -> Q35Chipset {
        Q35Chipset::new(self.pcie_bus, self.pci_bus, self.sata_bus, self.ide_bus)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct Q35Chipset {
    pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>,
    pci_bus: HashMap<PciAddress, Arc<dyn PciDevice>>,
    sata_bus: HashMap<SataAddress, Arc<dyn SataDevice>>,
    ide_bus: HashMap<IdeAddress, Arc<dyn IdeDevice>>,
}
