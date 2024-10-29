//use crate::yaml::QemuDevice;
use crate::config::QemuDevice;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Host {
    pci: Vec<Pci>,
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

#[derive(Debug, Deserialize)]
struct Pci {
    vm_id: String,
    host_id: String,
    multi_function: Option<bool>,
}

impl QemuDevice for Pci {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let multi_function = match self.multi_function {
            None => "".to_string(),
            Some(multi_function) => match multi_function {
                true => String::from(",multifunction=on"),
                false => String::from(",multifunction=off"),
            },
        };
        vec![format!(
            "-device vfio-pci,host={},id=hostpci{},bus=ich9-pcie-port-1,addr=0x{}{}",
            self.host_id, self.vm_id, self.vm_id, multi_function
        )]
    }
}

#[derive(Debug, Deserialize)]
struct Usb {
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
