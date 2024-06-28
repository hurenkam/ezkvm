use log::trace;
use serde::Deserialize;
use crate::resource::resource::Resource;

#[derive(Clone,Debug,Deserialize)]
pub struct X550VfResource {
    id: String,
    tags: Vec<String>,
    pci: String,
    parent: String,
    vf: String,
    multifunction: Option<bool>,
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
        todo!()
    }

    fn get_tags(&self) -> Vec<String> {
        todo!()
    }

    fn get_args(&self) -> Vec<String> {
        todo!()
    }
}
