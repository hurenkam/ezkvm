use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;

use crate::runtime_model::{
    IdeBus, IdeControllerApi, PciBus, PciControllerApi, PcieBus, PcieControllerApi, SataBus,
    SataControllerApi, ScsiBus, ScsiControllerApi, UsbBus, UsbControllerApi,
};

pub trait BusRegistrationApi {
    fn register_pcie_bus(&mut self, controller: Arc<dyn PcieControllerApi>) -> Result<u8, String>;
    #[allow(dead_code)] // TODO: wire to CLI
    fn register_pci_bus(&mut self, controller: Arc<dyn PciControllerApi>) -> Result<u8, String>;
    fn register_usb_bus(&mut self, controller: Arc<dyn UsbControllerApi>) -> Result<u8, String>;
    fn register_sata_bus(&mut self, controller: Arc<dyn SataControllerApi>) -> Result<u8, String>;
    fn register_ide_bus(&mut self, controller: Arc<dyn IdeControllerApi>) -> Result<u8, String>;
    fn register_scsi_bus(&mut self, controller: Arc<dyn ScsiControllerApi>) -> Result<u8, String>;
}

#[derive(Getters)]
pub struct BusRegister {
    pcie_busses: HashMap<PcieBus, Arc<dyn PcieControllerApi>>,
    pci_busses: HashMap<PciBus, Arc<dyn PciControllerApi>>,
    usb_busses: HashMap<UsbBus, Arc<dyn UsbControllerApi>>,
    sata_busses: HashMap<SataBus, Arc<dyn SataControllerApi>>,
    ide_busses: HashMap<IdeBus, Arc<dyn IdeControllerApi>>,
    scsi_busses: HashMap<ScsiBus, Arc<dyn ScsiControllerApi>>,
}
impl Default for BusRegister {
    fn default() -> Self {
        Self::new()
    }
}
impl BusRegister {
    pub fn new() -> Self {
        Self {
            pcie_busses: HashMap::new(),
            pci_busses: HashMap::new(),
            usb_busses: HashMap::new(),
            sata_busses: HashMap::new(),
            ide_busses: HashMap::new(),
            scsi_busses: HashMap::new(),
        }
    }
}
impl Display for BusRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for (id, value) in &self.pcie_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    pcie bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    pcie bus {id}, (no devices)")?;
            }
        }
        for (id, value) in &self.pci_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    pci  bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    pci  bus {id}, (no devices)")?;
            }
        }
        for (id, value) in &self.usb_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    usb  bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    usb  bus {id}, (no devices)")?;
            }
        }
        for (id, value) in &self.sata_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    sata bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    sata bus {id}, (no devices)")?;
            }
        }
        for (id, value) in &self.ide_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    ide  bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    ide  bus {id}, (no devices)")?;
            }
        }
        for (id, value) in &self.scsi_busses {
            for (address, device) in value.devices() {
                writeln!(f, "    scsi bus {id}, {address}: {device}")?;
            }
            if value.devices().is_empty() {
                writeln!(f, "    scsi bus {id}, (no devices)")?;
            }
        }
        Ok(())
    }
}
impl BusRegistrationApi for BusRegister {
    fn register_pcie_bus(&mut self, controller: Arc<dyn PcieControllerApi>) -> Result<u8, String> {
        let bus_id = self.pcie_busses.len() as u8;
        self.pcie_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_pci_bus(&mut self, controller: Arc<dyn PciControllerApi>) -> Result<u8, String> {
        let bus_id = self.pci_busses.len() as u8;
        self.pci_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_usb_bus(&mut self, controller: Arc<dyn UsbControllerApi>) -> Result<u8, String> {
        let bus_id = self.usb_busses.len() as u8;
        self.usb_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_sata_bus(&mut self, controller: Arc<dyn SataControllerApi>) -> Result<u8, String> {
        let bus_id = self.sata_busses.len() as u8;
        self.sata_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_ide_bus(&mut self, controller: Arc<dyn IdeControllerApi>) -> Result<u8, String> {
        let bus_id = self.ide_busses.len() as u8;
        self.ide_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
    fn register_scsi_bus(&mut self, controller: Arc<dyn ScsiControllerApi>) -> Result<u8, String> {
        let bus_id = self.scsi_busses.len() as u8;
        self.scsi_busses.insert(bus_id, controller);
        Ok(bus_id)
    }
}
