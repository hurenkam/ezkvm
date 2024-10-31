#[allow(unused)]
pub use crate::config::system::bios::ovmf::OVMF;
#[allow(unused)]
pub use crate::config::system::bios::seabios::SeaBios;
use crate::config::types::QemuDevice;

mod ovmf;
mod seabios;

#[typetag::deserialize(tag = "type")]
pub trait Bios: QemuDevice {}
impl Default for Box<dyn Bios> {
    fn default() -> Self {
        SeaBios::boxed_default()
    }
}
