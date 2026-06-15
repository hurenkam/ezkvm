use std::sync::Arc;

use super::{ControllerApi, PcieDeviceApi};
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

pub type ScsiBus = u8;

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
#[derive(Debug, Deserialize, Serialize, Getters, new)]
pub struct ScsiDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<ScsiBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<ScsiAddress>,
    device: ScsiDeviceType,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum ScsiDeviceType {
    ScsiDisk,
}
impl From<&ScsiDeviceType> for Arc<dyn ScsiDeviceApi> {
    fn from(device: &ScsiDeviceType) -> Self {
        match device {
            ScsiDeviceType::ScsiDisk => Arc::new(ScsiDisk::default()),
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
