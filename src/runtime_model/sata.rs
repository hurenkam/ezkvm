use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type SataBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct SataAddress {
    pub address: u8,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum SataDevice {
    SataDisk,
}
impl From<SataDevice> for Arc<dyn SataDeviceApi> {
    fn from(device: SataDevice) -> Self {
        match device {
            SataDevice::SataDisk => Arc::new(SataDisk {}),
        }
    }
}

pub struct SataDisk {}
impl SataDeviceApi for SataDisk {
    fn qemu_args(&self, _assigned_bus: &SataBus, _assigned_address: SataAddress) -> Vec<String> {
        todo!()
    }
}

pub trait SataDeviceApi {
    fn qemu_args(&self, assigned_bus: &SataBus, assigned_address: SataAddress) -> Vec<String>;
}

pub trait SataControllerApi: ControllerApi {
    fn register_sata_device(
        &self,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: Option<SataAddress>,
    ) -> Result<(), String>;
}
