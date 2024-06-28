use log::trace;
use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::resource::resource::Resource;
use crate::resource::x550_vf_resource::X550VfResource;

#[derive(Clone,Debug,Deserialize)]
pub struct GpuResource {
    id: String,
    tags: Vec<String>,
    pci_id: Vec<String>,
    multi_function: Option<bool>,
}

impl GpuResource {
    pub fn get_id(&self) -> String {
        trace!("Resource.get_id()");
        self.id.clone()
    }
}

#[typetag::deserialize(name = "host_gpu")]
impl Resource for GpuResource {
    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_tags(&self) -> Vec<String> {
        self.tags.clone()
    }
}

impl QemuDevice for GpuResource {
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
        let mut result = vec![];
        let mut count = 0;
        for pci_id in &self.pci_id {
            let vm_id = format!("{}.{}",index,count);
            result.push(
                format!("-device vfio-pci,host={},id=hostpci{},bus=ich9-pcie-port-1,addr=0x{}{}",pci_id, vm_id, vm_id, multi_function),
            );
            count += 1;
        }

        result
    }
}
