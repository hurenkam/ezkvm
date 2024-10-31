use crate::config::QemuDevice;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Usb {
    vm_port: String,
    host_port: String,
    host_bus: String,
}

impl QemuDevice for Usb {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![format!(
            "-device usb-host,bus=xhci.0,port={},hostbus={},hostport={},id=usb{}",
            self.vm_port, self.host_bus, self.host_port, index
        )]
    }
}
