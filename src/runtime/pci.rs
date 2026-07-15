use crate::runtime::{BusAddress, BusDevice};

#[allow(dead_code)]
pub trait PciDevice: BusDevice {
    fn get_pci_address(&self) -> Option<PciAddress>;
    fn set_pci_address(&self, address: PciAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}
impl BusAddress for PciAddress {
    fn get_bus_id(&self) -> u8 {
        self.bus
    }
}
