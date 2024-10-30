use crate::config::gpu::Gpu;
use crate::config::qemu_device::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct VirtioVgaGl {
    #[serde(default = "default_pci_address")]
    pci_address: String,
}

fn default_pci_address() -> String {
    "0x2".to_string()
}

impl QemuDevice for VirtioVgaGl {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        vec![format!(
            "-device virtio-vga-gl,id=vga,bus=pcie.0,addr={}",
            self.pci_address
        )]
    }
}

#[typetag::deserialize(name = "virtio-vga-gl")]
impl Gpu for VirtioVgaGl {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_test() {
        let gpu = VirtioVgaGl {
            pci_address: "0x2".to_string(),
        };
        let expected: Vec<String> =
            vec!["-device virtio-vga-gl,id=vga,bus=pcie.0,addr=0x2".to_string()];
        assert_eq!(gpu.get_qemu_args(0), expected);
    }
}
