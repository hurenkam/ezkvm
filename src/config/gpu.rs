pub use crate::config::gpu::no_gpu::NoGpu;
use crate::config::QemuDevice;

mod no_gpu;
mod passthrough_gpu;
mod virtio_vga_gl;

#[typetag::deserialize(tag = "type")]
pub trait Gpu: QemuDevice {}
impl Default for Box<dyn Gpu> {
    fn default() -> Self {
        NoGpu::boxed_default()
    }
}
