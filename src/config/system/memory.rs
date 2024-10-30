use crate::config::qemu_device::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Memory {
    max: u32,
    balloon: Option<bool>,
}

impl Memory {
    pub fn new(max: u32, balloon: Option<bool>) -> Self {
        Self { max, balloon }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let memory = Memory {
            max: 16384,
            balloon: None,
        };
        let expected: Vec<String> = vec!["-m 16384".to_string()];
        assert_eq!(memory.get_qemu_args(0), expected);
    }
}
