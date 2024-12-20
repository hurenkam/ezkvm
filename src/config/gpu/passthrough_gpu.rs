use crate::config::gpu::Gpu;
use crate::config::types::Pci;
use crate::config::types::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PassthroughGpu {
    pci: Vec<Pci>,
}

impl QemuDevice for PassthroughGpu {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![];

        for (index, item) in self.pci.iter().enumerate() {
            result.extend(item.get_qemu_args(index));
        }

        result
    }
}

#[typetag::deserialize(name = "passthrough")]
impl Gpu for PassthroughGpu {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let gpu = PassthroughGpu { pci: vec![] };
        let expected: Vec<String> = vec![];
        assert_eq!(gpu.get_qemu_args(0), expected);
    }
}
