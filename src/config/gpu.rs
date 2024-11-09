#[allow(unused)]
pub use crate::config::gpu::no_gpu::NoGpu;
#[allow(unused)]
pub use crate::config::gpu::passthrough_gpu::PassthroughGpu;
#[allow(unused)]
pub use crate::config::gpu::virtio_vga_gl::VirtioVgaGl;

use crate::config::QemuDevice;

mod no_gpu;
mod passthrough_gpu;
mod virtio_vga_gl;
mod vmware_svga;

#[typetag::deserialize(tag = "type")]
pub trait Gpu: QemuDevice {}
impl Default for Box<dyn Gpu> {
    fn default() -> Self {
        NoGpu::boxed_default()
    }
}
