mod ovmf;
mod seabios;

#[allow(unused)]
pub use ovmf::{OVMF, OVMFArch, OVMFSize};
#[allow(unused)]
pub use seabios::SeaBios;
use crate::config::types::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Bios: QemuDevice {}
impl Default for Box<dyn Bios> {
    fn default() -> Self {
        SeaBios::boxed_default()
    }
}
