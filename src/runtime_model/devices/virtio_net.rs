use std::fmt::Display;

use crate::runtime_config::NetworkResource;
use crate::runtime_model::{ControllerApi, PcieAddress, PcieBus, PcieDeviceApi};

#[derive(Default)]
pub struct VirtioNetController {
    resource: Option<NetworkResource>,
}

impl VirtioNetController {
    pub fn new(resource: Option<NetworkResource>) -> Self {
        Self { resource }
    }
}
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
        match &self.resource {
            Some(resource) => write!(f, "Virtio Net Controller ({resource:?})"),
            None => write!(f, "Virtio Net Controller"),
        }
    }
}
