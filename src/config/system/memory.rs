use crate::config::qemu_device::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Memory {
    max: u32,
    balloon: Option<bool>,
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            max: 2048,
            balloon: None,
        }
    }
}

impl QemuDevice for Memory {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![format!("-m {}", self.max)]
    }
}
