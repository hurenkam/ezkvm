use crate::config::qemu_device::QemuDevice;
use crate::config::system::tpm::Tpm;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NoTpm {}
impl NoTpm {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {})
    }
}

impl QemuDevice for NoTpm {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}

#[typetag::deserialize(name = "notpm")]
impl Tpm for NoTpm {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let notpm = NoTpm {};
        let expected: Vec<String> = vec![];
        assert_eq!(notpm.get_qemu_args(0), expected);
    }
}
