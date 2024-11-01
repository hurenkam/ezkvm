mod ide_cd;
mod scsi_drive;

use crate::config::QemuDevice;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StorageData {
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    boot_index: Option<u8>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_drive_options: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_device_options: Vec<String>,
}
impl StorageData {
    pub fn drive(&self, child_args: Vec<String>) -> String {
        let mut args: Vec<String> = vec![format!("file={},if=none,aio=io_uring", self.file)];
        args.extend(child_args);
        args.extend(self.extra_drive_options.clone());
        format!("-drive {}", args.join(","))
    }
    pub fn device(&self, child_args: Vec<String>) -> String {
        let mut args: Vec<String> = vec![];
        args.extend(child_args);
        if let Some(index) = self.boot_index {
            args.extend(vec![format!("bootindex={}", index)])
        }
        args.extend(self.extra_drive_options.clone());
        format!("-device {}", args.join(","))
    }
}

#[typetag::deserialize(tag = "type")]
pub trait Storage: QemuDevice {}
