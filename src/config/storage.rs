mod drive;
mod ide;
mod pvscsi;
mod sata;

#[typetag::deserialize(tag = "controller")]
pub trait Controller: 'static + Any + QemuDevice {}

use crate::config::QemuDevice;
use std::any::Any;
