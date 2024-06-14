use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::host::HostDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct HostPci {
    vm_id: String,
    host_id: String,
    multi_function: Option<bool>
}

#[typetag::deserialize(name = "pci")]
impl HostDevice for HostPci {}

impl QemuDevice for HostPci {
    fn get_args(&self, index: usize) -> Vec<String> {
        let multi_function = match self.multi_function {
            None => "".to_string(),
            Some(multi_function) => {
                match multi_function {
                    true => String::from(",multifunction=on"),
                    false => String::from(",multifunction=off")
                }
            }
        };
        vec![
            format!("-device vfio-pci,host={},id=hostpci{},bus=ich9-pcie-port-1,addr=0x{}{}",self.host_id, self.vm_id, self.vm_id, multi_function),
        ]
    }
}
