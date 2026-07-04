use std::any::Any;
use std::fmt::Display;

use crate::runtime_model::{NetworkResource, PcieAddress, PcieDeviceApi, PcieDeviceType};

#[derive(Default)]
pub struct VirtioNetController {
    resource: Option<NetworkResource>,
    mac_address: Option<String>,
    rx_queue_size: Option<u16>,
    tx_queue_size: Option<u16>,
    vhost: Option<bool>,
}

impl VirtioNetController {
    pub fn new(
        resource: Option<NetworkResource>,
        mac_address: Option<String>,
        rx_queue_size: Option<u16>,
        tx_queue_size: Option<u16>,
        vhost: Option<bool>,
    ) -> Self {
        Self {
            resource,
            mac_address,
            rx_queue_size,
            tx_queue_size,
            vhost,
        }
    }

    pub fn resource(&self) -> Option<&NetworkResource> {
        self.resource.as_ref()
    }

    pub fn mac_address(&self) -> Option<&str> {
        self.mac_address.as_deref()
    }

    pub fn rx_queue_size(&self) -> Option<u16> {
        self.rx_queue_size
    }

    pub fn tx_queue_size(&self) -> Option<u16> {
        self.tx_queue_size
    }

    pub fn vhost(&self) -> Option<bool> {
        self.vhost
    }
}
impl PcieDeviceApi for VirtioNetController {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn device_kind(&self) -> PcieDeviceType {
        PcieDeviceType::VirtioNet {
            resource: None,
            mac_address: self.mac_address.clone(),
            rx_queue_size: self.rx_queue_size,
            tx_queue_size: self.tx_queue_size,
            vhost: self.vhost,
        }
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
