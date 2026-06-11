use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type SataBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct SataAddress {
    pub address: u8,
}

pub trait SataDeviceApi {
    fn qemu_args(&self, assigned_bus: &SataBus, assigned_address: SataAddress) -> Vec<String>;
}

pub trait SataControllerApi: ControllerApi {
    fn register_sata_device(
        &self,
        device: Arc<dyn SataDeviceApi>,
        preferred_address: SataAddress,
    ) -> Result<(), String>;
}
