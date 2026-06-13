use std::sync::Arc;

use super::{ControllerApi, PcieAddress, PcieBus, PcieDeviceApi};
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type ScsiBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct ScsiAddress {
    pub target: u8,
    pub lun: u8,
}

pub trait ScsiDeviceApi {
    fn qemu_args(&self, assigned_bus: &ScsiBus, assigned_address: ScsiAddress) -> Vec<String>;
}
pub trait ScsiControllerApi: ControllerApi + PcieDeviceApi {
    fn register_scsi_device(
        &self,
        device: Arc<dyn ScsiDeviceApi>,
        preferred_address: Option<ScsiAddress>,
    ) -> Result<(), String>;
}

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
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ScsiDevice {
    ScsiDisk,
}
impl From<ScsiDevice> for Arc<dyn ScsiDeviceApi> {
    fn from(device: ScsiDevice) -> Self {
        match device {
            ScsiDevice::ScsiDisk => Arc::new(ScsiDisk::default()),
        }
    }
}

#[derive(Default)]
pub struct ScsiDisk {}

impl ScsiDeviceApi for ScsiDisk {
    fn qemu_args(&self, _assigned_bus: &ScsiBus, _assigned_address: ScsiAddress) -> Vec<String> {
        todo!()
    }
}
