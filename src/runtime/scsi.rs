use crate::runtime::{BusAddress, StorageDevice};

#[allow(dead_code)]
pub trait ScsiDevice: StorageDevice {
    fn get_scsi_address(&self) -> Option<ScsiAddress>;
    fn set_scsi_address(&self, address: ScsiAddress);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct ScsiAddress {
    pub bus: u8,
    pub target: u8,
    pub lun: u8,
}
impl BusAddress for ScsiAddress {
    fn get_bus_id(&self) -> u8 {
        self.bus
    }
}
