use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::ControllerApi;

pub type IdeBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct IdeAddress {
    pub address: u8,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum IdeDevice {
    IdeDisk,
}
impl From<IdeDevice> for Arc<dyn IdeDeviceApi> {
    fn from(device: IdeDevice) -> Self {
        match device {
            IdeDevice::IdeDisk => Arc::new(IdeDisk {}),
        }
    }
}
pub struct IdeDisk {}
impl IdeDeviceApi for IdeDisk {
    fn qemu_args(&self, _assigned_bus: &IdeBus, _assigned_address: IdeAddress) -> Vec<String> {
        todo!()
    }
}

pub trait IdeDeviceApi {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String>;
}

pub trait IdeControllerApi: ControllerApi {
    fn register_ide_device(
        &self,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String>;
}
