use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, sync::Arc};

use super::ControllerApi;
pub type PcieBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}
impl PcieAddress {
    pub fn device(&self) -> u8 {
        self.device
    }

    pub fn function(&self) -> u8 {
        self.function
    }
}
impl Display for PcieAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct PcieDevice {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bus: Option<PcieBus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    address: Option<PcieAddress>,
    #[serde(flatten)]
    device: PcieDeviceType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PcieDeviceType {
    PvScsi,
    VirtioNet {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
    },
}
impl From<&PcieDeviceType> for Arc<dyn PcieDeviceApi> {
    fn from(device: &PcieDeviceType) -> Self {
        match device {
            PcieDeviceType::PvScsi => Arc::new(super::PvScsiController::default()),
            PcieDeviceType::VirtioNet { .. } => Arc::new(super::VirtioNetController::default()),
        }
    }
}

pub trait PcieDeviceApi: Display {
    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String>;
}

pub trait PcieControllerApi: ControllerApi + Display {
    fn register_pcie_device(
        &self,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String>;
    fn devices(&self) -> HashMap<PcieAddress, Arc<dyn PcieDeviceApi>>;
}
