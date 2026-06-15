use std::fmt::Display;

use crate::runtime_model::{ControllerApi, PcieAddress, PcieBus, PcieDeviceApi};

#[derive(Default)]
pub struct VirtioNetController {}
impl PcieDeviceApi for VirtioNetController {
    fn qemu_args(&self, _bus: &PcieBus, _address: PcieAddress) -> Vec<String> {
        todo!()
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}
impl ControllerApi for VirtioNetController {}
impl Display for VirtioNetController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Virtio Net Controller")
    }
}
