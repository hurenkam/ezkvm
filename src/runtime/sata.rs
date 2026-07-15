use crate::runtime::{BusAddress, StorageDevice};

#[allow(dead_code)]
pub trait SataDevice: StorageDevice {
    fn get_sata_address(&self) -> Option<SataAddress>;
    fn set_sata_address(&self, address: SataAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct SataAddress {
    pub bus: u8,
    pub port: u8,
    pub device: u8,
}
impl BusAddress for SataAddress {
    fn get_bus_id(&self) -> u8 {
        self.bus
    }
}
