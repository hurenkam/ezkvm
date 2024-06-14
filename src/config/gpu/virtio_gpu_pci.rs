use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::gpu::Gpu;

#[derive(Deserialize,Debug,Clone)]
pub struct VirtioGpuPci {

}

impl QemuDevice for VirtioGpuPci {
    fn get_args(&self, index: usize) -> Vec<String> {
        let offset = 2;
        vec![
            format!("-device virtio-gpu-pci,id=vga,bus=pcie.0,addr={:#x}", offset + index),
        ]
    }
}

#[typetag::deserialize(name = "virtio-gpu-pci")]
impl Gpu for VirtioGpuPci {}
