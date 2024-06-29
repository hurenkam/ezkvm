use crate::config::config::QemuDevice;

mod virtio_vga_gl;
mod virtio_gpu_pci;
mod qxl_vga;
mod pool_gpu;

#[typetag::deserialize(tag = "type")]
pub trait Gpu: QemuDevice {}
