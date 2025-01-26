mod i440fx;
mod q35;

#[allow(unused)]
pub use q35::Q35;

use super::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Chipset: QemuDevice {}
impl Default for Box<dyn Chipset> {
    fn default() -> Self {
        Q35::boxed_default()
    }
}
