use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::host::HostDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct HostUsb {
    vm_port: String,
    host_port: String,
    host_bus: String
}

#[typetag::deserialize(name = "usb")]
impl HostDevice for HostUsb {}

impl QemuDevice for HostUsb {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",self.vm_port, self.host_bus, self.host_port, index),
        ]
    }
}
