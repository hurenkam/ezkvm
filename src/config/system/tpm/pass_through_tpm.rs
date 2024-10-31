use crate::config::system::tpm::Tpm;
use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct PassThroughTpm {}
impl QemuDevice for PassThroughTpm {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        todo!()
    }
}

#[typetag::deserialize(name = "passthrough")]
impl Tpm for PassThroughTpm {}
