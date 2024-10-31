mod q35;

#[allow(unused)]
pub use crate::config::system::chipset::q35::Q35;

use crate::config::types::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Chipset: QemuDevice {}
impl Default for Box<dyn Chipset> {
    fn default() -> Self {
        Q35::boxed_default()
    }
}
