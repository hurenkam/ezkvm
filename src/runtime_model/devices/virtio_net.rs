use std::fmt::Display;
use std::vec;

use crate::runtime_config::NetworkResource;
use crate::runtime_model::{PcieAddress, PcieBus, PcieDeviceApi};

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
    fn qemu_args(&self, bus: &PcieBus, address: PcieAddress) -> Vec<String> {
        let device_id = format!("net{}f{}", address.device(), address.function());
        let netdev = match &self.resource {
            Some(NetworkResource::Tap { tap }) => {
                format!("tap,id={device_id},ifname={tap}")
            }
            Some(NetworkResource::Bridge { bridge }) => {
                format!("bridge,id={device_id},br={bridge}")
            }
            None => format!("user,id={device_id}"),
        };

        vec![
            "-netdev".to_string(),
            netdev,
            "-device".to_string(),
            format!(
                "virtio-net-pci,id={device_id},netdev={device_id},bus=pcie.{bus},addr=0x{:x}.{:x}",
                address.device(),
                address.function()
            ),
        ]
    }

    fn preferred_address(&self) -> Option<PcieAddress> {
        None
    }
}
impl Display for VirtioNetController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.resource {
            Some(resource) => write!(f, "Virtio Net Controller ({resource:?})"),
            None => write!(f, "Virtio Net Controller"),
        }
    }
}
