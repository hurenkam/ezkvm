use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type PcieBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PcieAddress {
    device: u8,
    function: u8,
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
        preferred_address: PcieAddress,
    ) -> Result<(), String>;
}
