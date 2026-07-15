use crate::runtime::{BusAddress, BusDevice};

#[allow(dead_code)]
pub trait PcieDevice: BusDevice {
    fn get_pcie_address(&self) -> Option<PcieAddress>;
    fn set_pcie_address(&self, address: PcieAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PcieAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}
impl BusAddress for PcieAddress {
    fn get_bus_id(&self) -> u8 {
        self.bus
    }
}
