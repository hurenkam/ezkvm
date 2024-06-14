use std::fmt::Debug;
use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::storage::StorageDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct ScsiHardDrive {
    file: String,
    discard: Option<bool>,
    boot_index: Option<String>
}

impl QemuDevice for ScsiHardDrive {
    fn get_args(&self, index: usize) -> Vec<String> {
        let boot_index = match self.boot_index.clone() {
            None => "".to_string(),
            Some(boot_index) => format!(",bootindex={}",boot_index)
        };
        let discard = match self.discard {
            None => { "" }
            Some(discard) => {
                if (discard) {
                    ",discard=on"
                } else {
                    ",discard=off"
                }
            }
        };
        vec![
            format!("-drive file={},if=none,id=drive-scsi{}{},format=raw,cache=none,aio=io_uring,detect-zeroes=unmap",self.file,index,discard),
            format!("-device scsi-hd,bus=scsihw0.0,scsi-id={},drive=drive-scsi{},id=scsi{},rotation_rate=1{}",index,index,index,boot_index),
        ]
    }
}

#[typetag::deserialize(name = "scsi-hd")]
impl StorageDevice for ScsiHardDrive {}
