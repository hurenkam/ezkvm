use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::cpu::Cpu;

#[derive(Deserialize,Debug,Clone)]
pub struct Memory {
    max: u32,
    balloon: Option<bool>
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
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-m {}", self.max),
        ]
    }
}
