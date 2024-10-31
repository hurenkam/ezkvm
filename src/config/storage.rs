use crate::config::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Storage: QemuDevice {}
