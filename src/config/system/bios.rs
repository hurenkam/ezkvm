use crate::config::qemu_device::QemuDevice;
pub use crate::config::system::bios::ovmf::OVMF;
pub use crate::config::system::bios::seabios::SeaBios;

mod ovmf;
mod seabios;

#[typetag::deserialize(tag = "type")]
pub trait Bios: QemuDevice {}
impl Default for Box<dyn Bios> {
    fn default() -> Self {
        SeaBios::boxed_default()
    }
}
