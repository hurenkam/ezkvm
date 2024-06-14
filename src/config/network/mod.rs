use crate::config::config::QemuDevice;

mod network_pool_device;
mod network_bridge_device;

#[typetag::deserialize(tag = "type")]
pub trait NetworkDevice: QemuDevice {
    fn pre_start(&self) {}
    fn post_stop(&self) {}
}
