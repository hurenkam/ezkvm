use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::ControllerApi;

pub type IdeBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct IdeAddress {
    pub address: u8,
}

pub trait IdeDeviceApi {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String>;
}

pub trait IdeControllerApi: ControllerApi {
    fn register_ide_device(
        &self,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: IdeAddress,
    ) -> Result<(), String>;
}
