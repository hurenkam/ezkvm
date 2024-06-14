mod host_pci;
mod host_usb;

use crate::config::config::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait HostDevice: QemuDevice {}
