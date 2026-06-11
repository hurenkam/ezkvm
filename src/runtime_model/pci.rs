use std::sync::Arc;

use derive_new::new;
use serde::{Deserialize, Serialize};

use super::ControllerApi;

pub type PciBus = u8;

#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct PciAddress {
    pub device: u8,
    pub function: u8,
}

pub trait PciDeviceApi {
    fn preferred_address(&self) -> Option<PciAddress> {
        None
    }
    fn qemu_args(&self, bus: &PciBus, address: PciAddress) -> Vec<String>;
}

pub trait PciControllerApi: ControllerApi {
    fn register_pci_device(
        &self,
        device: Arc<dyn PciDeviceApi>,
        preferred_address: PciAddress,
    ) -> Result<(), String>;
}
