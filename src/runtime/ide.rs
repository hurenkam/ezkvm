use crate::runtime::{BusAddress, StorageDevice};

#[allow(dead_code)]
pub trait IdeDevice: StorageDevice {
    fn get_ide_address(&self) -> IdeAddress;
    fn set_ide_address(&self, address: IdeAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct IdeAddress {
    pub channel: u8,
    pub device: u8,
}
impl BusAddress for IdeAddress {
    fn get_bus_id(&self) -> u8 {
        0
    }
}
