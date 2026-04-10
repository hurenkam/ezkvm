#[allow(unused)]
pub use no_gpu::NoGpu;
#[allow(unused)]
pub use passthrough_gpu::PassthroughGpu;

use super::QemuDevice;

mod no_gpu;
mod passthrough_gpu;
mod virtio;
mod vmware_svga;

#[typetag::deserialize(tag = "type")]
pub trait Gpu: QemuDevice {
    fn use_gl(&self) -> bool {
        false
    }
}
impl Default for Box<dyn Gpu> {
    fn default() -> Self {
        NoGpu::boxed_default()
    }
}
