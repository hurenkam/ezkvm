use std::{fmt::Display, sync::Arc};

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type PcieBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, Eq, PartialEq, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}
impl Display for PcieAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dev {}, func {}", self.device, self.function)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PcieDevice {
    PvScsi,
    VirtioNet,
}
impl From<PcieDevice> for Arc<dyn PcieDeviceApi> {
    fn from(device: PcieDevice) -> Self {
        match device {
            PcieDevice::PvScsi => Arc::new(super::PvScsiController::default()),
            PcieDevice::VirtioNet => Arc::new(super::VirtioNetController::default()),
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
}
