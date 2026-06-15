use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::ControllerApi;

pub type IdeBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct IdeAddress {
    pub address: u8,
}
impl Display for IdeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "address {}", self.address)
    }
}
#[derive(Debug, Deserialize, Serialize, Getters, new)]
pub struct IdeDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<IdeBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<IdeAddress>,
    device: IdeDeviceType,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum IdeDeviceType {
    IdeDisk,
}
impl From<&IdeDeviceType> for Arc<dyn IdeDeviceApi> {
    fn from(device: &IdeDeviceType) -> Self {
        match device {
            IdeDeviceType::IdeDisk => Arc::new(IdeDisk {}),
        }
    }
}
pub struct IdeDisk {}
impl IdeDeviceApi for IdeDisk {
    fn qemu_args(&self, _assigned_bus: &IdeBus, _assigned_address: IdeAddress) -> Vec<String> {
        todo!()
    }
}
impl Display for IdeDisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IDE Disk")
    }
}

pub trait IdeDeviceApi: Display {
    fn qemu_args(&self, assigned_bus: &IdeBus, assigned_address: IdeAddress) -> Vec<String>;
}

pub trait IdeControllerApi: ControllerApi + Display {
    fn register_ide_device(
        &self,
        device: Arc<dyn IdeDeviceApi>,
        preferred_address: Option<IdeAddress>,
    ) -> Result<(), String>;
}
