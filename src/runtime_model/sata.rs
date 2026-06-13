use std::{fmt::Display, sync::Arc};

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type SataBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct SataAddress {
    pub address: u8,
}
impl Display for SataAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SataDevice {
    Disk,
}
impl From<SataDevice> for Arc<dyn SataDeviceApi> {
    fn from(device: SataDevice) -> Self {
        match device {
            SataDevice::Disk => Arc::new(SataDisk {}),
        }
    }
}

pub struct SataDisk {}
impl SataDeviceApi for SataDisk {
    fn qemu_args(&self, _assigned_bus: &SataBus, _assigned_address: SataAddress) -> Vec<String> {
        todo!()
    }
}
impl Display for SataDisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SATA Disk")
    }
}

pub trait SataDeviceApi: Display {
    fn qemu_args(&self, assigned_bus: &SataBus, assigned_address: SataAddress) -> Vec<String>;
}

pub trait SataControllerApi: ControllerApi + Display {
    fn register_sata_device(
        &self,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: Option<SataAddress>,
    ) -> Result<(), String>;
}
