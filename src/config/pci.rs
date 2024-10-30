use crate::config::default_when_missing;
use crate::config::QemuDevice;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Pci {
    vm_id: String,
    host_id: String,
    #[serde(default, deserialize_with = "default_when_missing")]
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
