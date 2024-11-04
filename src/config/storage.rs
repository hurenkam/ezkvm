mod ide_cd;
mod scsi_drive;

use crate::config::QemuDevice;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct StorageHeader {
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

#[typetag::deserialize(tag = "type")]
pub trait StoragePayload: Debug {
    fn get_drive_options(&self, index: usize) -> Vec<String> {
        vec![]
    }
    fn get_device_options(&self, index: usize) -> Vec<String> {
        vec![]
    }
}

#[derive(Deserialize, Debug)]
pub struct StorageItem {
    #[serde(flatten)]
    header: StorageHeader,
    #[serde(flatten)]
    payload: Box<dyn StoragePayload>,
}

impl QemuDevice for StorageItem {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut drive_args: Vec<String> =
            vec![format!("file={},if=none,aio=io_uring", self.header.file)];
        drive_args.extend(self.payload.get_drive_options(index));
        drive_args.extend(self.header.extra_drive_options.clone());

        let mut device_args: Vec<String> = vec![];
        device_args.extend(self.payload.get_device_options(index));
        if let Some(boot_index) = self.header.boot_index {
            device_args.extend(vec![format!("bootindex={}", boot_index)])
        }
        device_args.extend(self.header.extra_device_options.clone());

        vec![
            format!("-drive {}", drive_args.join(",")),
            format!("-device {}", device_args.join(",")),
        ]
    }
}
