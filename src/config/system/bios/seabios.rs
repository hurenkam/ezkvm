use crate::config::qemu_device::QemuDevice;
use crate::config::system::bios::Bios;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct SeaBios {
    uuid: String,
}
impl SeaBios {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {
            uuid: "".to_string(),
        })
    }
}

impl QemuDevice for SeaBios {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}

#[typetag::deserialize(name = "seabios")]
impl Bios for SeaBios {}
