use crate::config::config::QemuDevice;
use crate::config::system::system_q35::SystemQ35;

mod system_q35;

#[typetag::deserialize(tag = "type")]
pub trait System: QemuDevice {}
impl Default for Box<dyn System> {
    fn default() -> Self {
        SystemQ35::boxed_default()
    }
}
