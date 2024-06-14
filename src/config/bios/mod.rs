use crate::config::bios::seabios::SeaBios;
use crate::config::config::QemuDevice;

pub mod ovmf;
pub mod seabios;

#[typetag::deserialize(tag = "type")]
pub trait Bios: QemuDevice {}
impl Default for Box<dyn Bios> {
    fn default() -> Self {
        SeaBios::boxed_default()
    }
}
