use crate::config::gpu::Gpu;
use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NoGpu {}
impl NoGpu {
    pub fn boxed_default() -> Box<Self> {
        Box::new(Self {})
    }
}

impl QemuDevice for NoGpu {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![]
    }
}

#[typetag::deserialize(name = "no_gpu")]
impl Gpu for NoGpu {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let no_gpu = NoGpu {};
        let expected: Vec<String> = vec![];
        assert_eq!(no_gpu.get_qemu_args(0), expected);
    }
}
