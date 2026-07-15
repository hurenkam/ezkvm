use std::{
    any::TypeId,
    sync::{Arc, Mutex},
};

use crate::runtime::{BusDeviceRegistry, IdeDevice, PciDevice, PcieDevice, SataDevice};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Q35Chipset {
    pcie_bus_id: u8,
    sata_bus_id: u8,
    pci_bus_id: u8,
    ide_bus_id: u8,
}

impl Q35Chipset {
    pub fn new(bus_devices: Arc<Mutex<BusDeviceRegistry>>) -> Self {
        let mut bus_devices = bus_devices.lock().unwrap();
        let pcie_bus_id = bus_devices.add_bus(TypeId::of::<dyn PcieDevice>());
        println!("Q35Chipset::new(): pcie bus id: {}, type id: {:?}, type name: {}", pcie_bus_id, TypeId::of::<dyn PcieDevice>(), std::any::type_name::<dyn PcieDevice>());
        let sata_bus_id = bus_devices.add_bus(TypeId::of::<dyn SataDevice>());
        println!("Q35Chipset::new(): sata bus id: {}, type id: {:?}, type name: {}", sata_bus_id, TypeId::of::<dyn SataDevice>(), std::any::type_name::<dyn SataDevice>());
        let pci_bus_id = bus_devices.add_bus(TypeId::of::<dyn PciDevice>());
        println!("Q35Chipset::new(): pci bus id: {}, type id: {:?}, type name: {}", pci_bus_id, TypeId::of::<dyn PciDevice>(), std::any::type_name::<dyn PciDevice>());
        let ide_bus_id = bus_devices.add_bus(TypeId::of::<dyn IdeDevice>());
        println!("Q35Chipset::new(): ide bus id: {}, type id: {:?}, type name: {}", ide_bus_id, TypeId::of::<dyn IdeDevice>(), std::any::type_name::<dyn IdeDevice>());

        Q35Chipset {
            pcie_bus_id,
            sata_bus_id,
            pci_bus_id,
            ide_bus_id,
        }
    }
}
