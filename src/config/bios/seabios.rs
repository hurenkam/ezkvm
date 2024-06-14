use serde::Deserialize;
use crate::config::bios::Bios;
use crate::config::config::QemuDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct SeaBios {
    uuid: String,
}
impl SeaBios {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self { uuid: "".to_string() } )
    }
}

impl QemuDevice for SeaBios {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![]
    }
}

#[typetag::deserialize(name = "seabios")]
impl Bios for SeaBios {}
