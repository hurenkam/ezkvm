use log::trace;
use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::resource::resource::Resource;

#[derive(Clone,Debug,Deserialize)]
pub struct X550VfResource {
    id: String,
    tags: Vec<String>,
    pci_id: String,
    parent: String,
    vf: String,
    multi_function: Option<bool>,
}

impl X550VfResource {
    pub fn get_id(&self) -> String {
        trace!("Resource.get_id()");
        self.id.clone()
    }
}

#[typetag::deserialize(name = "x550_vf")]
impl Resource for X550VfResource {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

impl QemuDevice for X550VfResource {
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
        let vm_id = format!("{}.0",index);
        vec![
            format!("-device vfio-pci,host={},id=hostpci{},bus=ich9-pcie-port-1,addr=0x{}{}",self.pci_id, vm_id, vm_id, multi_function),
        ]
    }
}
