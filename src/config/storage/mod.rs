use crate::config::config::QemuDevice;

mod ide_cdrom_drive;
mod scsi_hard_drive;

#[typetag::deserialize(tag = "type")]
pub trait StorageDevice: QemuDevice {}
