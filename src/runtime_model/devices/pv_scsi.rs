use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::runtime_model::{
    ControllerApi, PcieAddress, PcieBus, PcieDeviceApi, ScsiAddress, ScsiControllerApi,
    ScsiDeviceApi,
};

#[derive(Default)]
pub struct PvScsiController {}

impl PcieDeviceApi for PvScsiController {
    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        todo!()
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}
impl ControllerApi for PvScsiController {}
impl ScsiControllerApi for PvScsiController {
    fn register_scsi_device(
        &self,
        _device: Arc<dyn ScsiDeviceApi>,
        _preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String> {
        todo!()
    }
    
    fn devices(&self) -> HashMap<ScsiAddress, Arc<dyn ScsiDeviceApi>> {
        todo!()
    }
}
impl Display for PvScsiController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PV SCSI Controller")
    }
}
