use derive_getters::Getters;
use derive_new::new;

use crate::runtime::PcieDevice;

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct VirtioNetPcie {
    resource: Option<String>,
    mac_address: Option<String>,
    rx_queue_size: Option<u16>,
    tx_queue_size: Option<u16>,
    vhost: Option<bool>,
}

impl PcieDevice for VirtioNetPcie {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
        crate::runtime::PcieBusDeviceKind::VirtioNet
    }
}
