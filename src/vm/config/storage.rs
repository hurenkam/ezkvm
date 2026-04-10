mod drive;
mod ide;
mod pvscsi;
mod sata;
mod virtio_scsi_single;

#[typetag::deserialize(tag = "controller")]
pub trait Controller: 'static + Any + QemuDevice {}

use super::QemuDevice;
use std::any::Any;
