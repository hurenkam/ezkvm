use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct AppleSmc {
    osk: String,
}

impl Default for AppleSmc {
    fn default() -> Self {
        Self {
            osk: "".to_string(),
        }
    }
}

impl QemuDevice for AppleSmc {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![format!("-device isa-applesmc,osk={}", self.osk)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let memory = AppleSmc {
            osk: "my-osk-key".to_string(),
        };
        let expected: Vec<String> = vec!["-device isa-applesmc,osk=my-osk-key".to_string()];
        assert_eq!(memory.get_qemu_args(0), expected);
    }
}
