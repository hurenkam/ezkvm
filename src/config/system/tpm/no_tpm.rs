use crate::config::system::tpm::Tpm;
use crate::config::types::QemuDevice;
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

#[typetag::deserialize(name = "no_tpm")]
impl Tpm for NoTpm {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let no_tpm = NoTpm {};
        let expected: Vec<String> = vec![];
        assert_eq!(no_tpm.get_qemu_args(0), expected);
    }
}
