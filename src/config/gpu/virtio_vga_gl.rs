use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::gpu::Gpu;

#[derive(Deserialize,Debug,Clone)]
pub struct VirtioVgaGl {

}

impl QemuDevice for VirtioVgaGl {
    fn get_args(&self, index: usize) -> Vec<String> {
        let offset = 2;
        vec![
            format!("-device virtio-vga-gl,id=vga,bus=pcie.0,addr={:#x}", offset + index),
        ]
    }
}

#[typetag::deserialize(name = "virtio-vga-gl")]
impl Gpu for VirtioVgaGl {}
