use log::trace;
use serde::Deserialize;
use crate::resource::resource::Resource;

#[derive(Clone,Debug,Deserialize)]
pub struct PciResource {
    id: String,
    tags: Vec<String>,
    pci: Vec<String>,
    multifunction: Option<bool>,
}

impl PciResource {
    pub fn get_id(&self) -> String {
        trace!("Resource.get_id()");
        self.id.clone()
    }
}

#[typetag::deserialize(name = "host_pci")]
impl Resource for PciResource {
    fn get_id(&self) -> String {
        todo!()
    }

    fn get_tags(&self) -> Vec<String> {
        todo!()
    }

    fn get_args(&self) -> Vec<String> {
        todo!()
    }
}