use std::{fmt::Display, sync::Arc};

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type PciBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PciAddress {
    pub device: u8,
    pub function: u8,
}
impl Display for PciAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct PciDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PciBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<PciAddress>,
    device: PciDeviceType,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PciDeviceType {
    NetworkController,
}
impl From<&PciDeviceType> for Arc<dyn PciDeviceApi> {
    fn from(device: &PciDeviceType) -> Self {
        match device {
            PciDeviceType::NetworkController => {
                todo!();
            }
        }
    }
}

pub trait PciDeviceApi: Display {
    fn preferred_address(&self) -> Option<PciAddress> {
        None
    }
    fn qemu_args(&self, bus: &PciBus, address: PciAddress) -> Vec<String>;
}

pub trait PciControllerApi: ControllerApi + Display {
    fn register_pci_device(
        &self,
        device: Arc<dyn PciDeviceApi>,
        preferred_address: Option<PciAddress>,
    ) -> Result<(), String>;
}
