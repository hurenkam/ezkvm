mod ovmf;
mod seabios;

use super::QemuDevice;
#[allow(unused)]
pub use ovmf::{Ovmf, OvmfArch, OvmfSize};
#[allow(unused)]
pub use seabios::SeaBios;

#[typetag::deserialize(tag = "type")]
pub trait Bios: QemuDevice {}
impl Default for Box<dyn Bios> {
    fn default() -> Self {
        SeaBios::boxed_default()
    }
}
