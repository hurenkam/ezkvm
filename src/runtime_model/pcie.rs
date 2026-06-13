use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type PcieBus = String;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum PcieDevice {
    PvScsiController,
}
impl From<PcieDevice> for Arc<dyn PcieDeviceApi> {
    fn from(device: PcieDevice) -> Self {
        match device {
            PcieDevice::PvScsiController => Arc::new(super::PvScsiController::default()),
        }
    }
}

pub trait PcieDeviceApi {
    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String>;
}

pub trait PcieControllerApi: ControllerApi {
    fn register_pcie_device(
        &self,
        device: Arc<dyn PcieDeviceApi>,
        preferred_address: Option<PcieAddress>,
    ) -> Result<(), String>;
}
