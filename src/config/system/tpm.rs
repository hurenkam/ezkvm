use crate::config::qemu_device::QemuDevice;
use crate::config::system::tpm::notpm::NoTpm;

mod notpm;
mod pass_through_tpm;
mod swtpm;

#[typetag::deserialize(tag = "type")]
pub trait Tpm: QemuDevice {}
impl Default for Box<dyn Tpm> {
    fn default() -> Self {
        NoTpm::boxed_default()
    }
}
