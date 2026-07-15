use crate::runtime::{BusAddress, BusDevice};

#[allow(dead_code)]
pub trait IsaDevice: BusDevice {
    fn get_isa_address(&self) -> Option<IsaAddress>;
    fn set_isa_address(&self, address: IsaAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct IsaAddress(u16);

impl BusAddress for IsaAddress {
    fn get_bus_id(&self) -> u8 {
        0
    }
}
