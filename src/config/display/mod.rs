use crate::config::config::QemuDevice;

mod gtk;

#[typetag::deserialize(tag = "type")]
pub trait Display: QemuDevice {}
