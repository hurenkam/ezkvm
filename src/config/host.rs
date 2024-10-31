use crate::config::default_when_missing;
use crate::config::{Pci, QemuDevice, Usb};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Host {
    #[serde(default, deserialize_with = "default_when_missing")]
    pci: Vec<Pci>,
    #[serde(default, deserialize_with = "default_when_missing")]
    usb: Vec<Usb>,
}

impl QemuDevice for Host {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];

        for (index, item) in self.pci.iter().enumerate() {
            result.extend(item.get_qemu_args(index));
        }

        for (index, item) in self.usb.iter().enumerate() {
            result.extend(item.get_qemu_args(index));
        }

        result
    }
}
