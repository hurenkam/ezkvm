use super::QemuDevice;
use super::Tpm;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct PassThroughTpm {}
impl QemuDevice for PassThroughTpm {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-tpmdev passthrough,id=tpm{},path=/dev/tpm0", index),
            format!("-device tpm-tis,tpmdev=tpm{}", index),
        ]
    }
}

#[typetag::deserialize(name = "passthrough")]
impl Tpm for PassThroughTpm {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let passthrough_tpm = PassThroughTpm {};
        assert_eq!(
            passthrough_tpm.get_qemu_args(0),
            vec![
                "-tpmdev passthrough,id=tpm0,path=/dev/tpm0".to_string(),
                "-device tpm-tis,tpmdev=tpm0".to_string(),
            ]
        );
    }
}
