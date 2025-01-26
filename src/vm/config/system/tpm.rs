#[allow(unused)]
pub use no_tpm::NoTpm;
#[allow(unused)]
pub use pass_through_tpm::PassThroughTpm;
#[allow(unused)]
pub use swtpm::SwTpm;

use super::QemuDevice;

mod no_tpm;
mod pass_through_tpm;
mod swtpm;

#[typetag::deserialize(tag = "type")]
pub trait Tpm: QemuDevice {}
impl Default for Box<dyn Tpm> {
    fn default() -> Self {
        NoTpm::boxed_default()
    }
}
