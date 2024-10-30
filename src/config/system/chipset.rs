mod q35;

use crate::config::qemu_device::QemuDevice;
pub use crate::config::system::chipset::q35::Q35;

#[typetag::deserialize(tag = "type")]
pub trait Chipset: QemuDevice {}
impl Default for Box<dyn Chipset> {
    fn default() -> Self {
        Q35::boxed_default()
    }
}
