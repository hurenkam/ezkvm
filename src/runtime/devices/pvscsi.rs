use std::sync::{Arc, Mutex};

use crate::runtime::{
    BusDevice, BusDeviceRegistry, PciDevice, PcieAddress, PcieDevice, isa::{IsaAddress, IsaDevice}, pci::PciAddress, scsi::ScsiDevice,
};

#[allow(dead_code)]
#[derive(Debug)]
enum Address {
    Pcie(PcieAddress),
    Pci(PciAddress),
    Isa(IsaAddress),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PvScsi {
    address: Mutex<Option<Address>>,
    bus_id: u8,
}

impl PvScsi {
    pub fn new(bus_devices: Arc<Mutex<BusDeviceRegistry>>) -> Self {
        let bus_id = bus_devices
            .lock()
            .unwrap()
            .add_bus(std::any::TypeId::of::<dyn ScsiDevice>());
        println!("PvScsi::new(): scsi bus id: {}, type id: {:?}, type name: {}", bus_id, std::any::TypeId::of::<dyn ScsiDevice>(), std::any::type_name::<dyn ScsiDevice>());
        PvScsi {
            address: Mutex::new(None),
            bus_id,
        }
    }
}

impl PcieDevice for PvScsi {
    fn get_pcie_address(&self) -> Option<PcieAddress> {
        match *self.address.lock().unwrap() {
            Some(Address::Pcie(addr)) => Some(addr),
            _ => None,
        }
    }
    fn set_pcie_address(&self, address: PcieAddress) {
        println!("Setting PCIe address to {:?}", address);
        let mut addr = self.address.lock().unwrap();
        *addr = Some(Address::Pcie(address));
    }
}

impl PciDevice for PvScsi {
    fn get_pci_address(&self) -> Option<PciAddress> {
        match *self.address.lock().unwrap() {
            Some(Address::Pci(addr)) => Some(addr),
            _ => None,
        }
    }
    fn set_pci_address(&self, address: PciAddress) {
        println!("Setting PCI address to {:?}", address);
        let mut addr = self.address.lock().unwrap();
        *addr = Some(Address::Pci(address));
    }
}

impl IsaDevice for PvScsi {
    fn get_isa_address(&self) -> Option<IsaAddress> {
        match *self.address.lock().unwrap() {
            Some(Address::Isa(addr)) => Some(addr),
            _ => None,
        }
    }
    fn set_isa_address(&self, address: IsaAddress) {
        println!("Setting ISA address to {:?}", address);
        let mut addr = self.address.lock().unwrap();
        *addr = Some(Address::Isa(address));
    }
}

impl BusDevice for PvScsi {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_name(&self) -> &str {
        "pvscsi"
    }

    fn get_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}
