use std::{collections::HashMap, fmt::Display, sync::Arc};

use derive_getters::Getters;

use crate::runtime_model::{
    IdeBus, IdeControllerApi, PciBus, PciControllerApi, PcieBus, PcieControllerApi, SataBus,
    SataControllerApi, ScsiBus, ScsiControllerApi, UsbBus, UsbControllerApi,
};

pub trait BusRegistrationApi {
    fn register_pcie_bus(&mut self, controller: Arc<dyn PcieControllerApi>) -> Result<u8, String>;
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
    pub fn qemu_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        let mut pcie_keys: Vec<_> = self.pcie_busses.keys().copied().collect();
        pcie_keys.sort_unstable();
        for key in pcie_keys {
            if let Some(controller) = self.pcie_busses.get(&key) {
                args.extend(controller.qemu_args());
            }
        }

        let mut pci_keys: Vec<_> = self.pci_busses.keys().copied().collect();
        pci_keys.sort_unstable();
        for key in pci_keys {
            if let Some(controller) = self.pci_busses.get(&key) {
                args.extend(controller.qemu_args());
            }
        }

        let mut usb_keys: Vec<_> = self.usb_busses.keys().copied().collect();
        usb_keys.sort_unstable();
        for key in usb_keys {
            if let Some(controller) = self.usb_busses.get(&key) {
                args.extend(controller.qemu_args());
            }
        }

        let mut sata_keys: Vec<_> = self.sata_busses.keys().copied().collect();
        sata_keys.sort_unstable();
        for key in sata_keys {
            if let Some(controller) = self.sata_busses.get(&key) {
                args.extend(controller.qemu_args());
            }
        }

        let mut ide_keys: Vec<_> = self.ide_busses.keys().copied().collect();
        ide_keys.sort_unstable();
        for key in ide_keys {
            if let Some(controller) = self.ide_busses.get(&key) {
                args.extend(controller.qemu_args());
            }
        }

        let mut scsi_keys: Vec<_> = self.scsi_busses.keys().copied().collect();
        scsi_keys.sort_unstable();
        for key in scsi_keys {
            if let Some(_controller) = self.scsi_busses.get(&key) {
                // SCSI device rendering is owned by the controller's PCIe device emitter.
            }
        }
        args
    }
}
impl Display for BusRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n")?;
        for (id, value) in &self.pcie_busses {
            for (address, device) in value.devices() {
                write!(f, "    pcie bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    pcie bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.pci_busses {
            for (address, device) in value.devices() {
                write!(f, "    pci  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    pci  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.usb_busses {
            for (address, device) in value.devices() {
                write!(f, "    usb  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    usb  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.sata_busses {
            for (address, device) in value.devices() {
                write!(f, "    sata bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    sata bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.ide_busses {
            for (address, device) in value.devices() {
                write!(f, "    ide  bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    ide  bus {id}, (no devices)\n")?;
            }
        }
        for (id, value) in &self.scsi_busses {
            for (address, device) in value.devices() {
                write!(f, "    scsi bus {id}, {address}: {device}\n")?;
            }
            if value.devices().len() == 0 {
                write!(f, "    scsi bus {id}, (no devices)\n")?;
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
