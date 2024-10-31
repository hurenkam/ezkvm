mod bridge;

use crate::config::QemuDevice;

#[typetag::deserialize(tag = "type")]
pub trait Network: QemuDevice {}
