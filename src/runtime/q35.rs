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
        let sata_bus_id = bus_devices.add_bus(TypeId::of::<dyn SataDevice>());
        let pci_bus_id = bus_devices.add_bus(TypeId::of::<dyn PciDevice>());
        let ide_bus_id = bus_devices.add_bus(TypeId::of::<dyn IdeDevice>());

        Q35Chipset {
            pcie_bus_id,
            sata_bus_id,
            pci_bus_id,
            ide_bus_id,
        }
    }
}
