use crate::yaml::QemuArgs;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Storage {
    driver: String,
    file: String,
    discard: Option<bool>,
    boot_index: Option<String>,
}

impl QemuArgs for Storage {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let boot_index = match self.boot_index.clone() {
            None => "".to_string(),
            Some(boot_index) => format!(",bootindex={}", boot_index),
        };
        let discard = match self.discard {
            None => "",
            Some(discard) => {
                if discard {
                    ",discard=on"
                } else {
                    ",discard=off"
                }
            }
        };
        match self.driver.as_str() {
            "scsi-hd" => {
                vec![
                    format!("-drive file={},if=none,id=drive-scsi{}{},format=raw,cache=none,aio=io_uring,detect-zeroes=unmap",self.file,index,discard),
                    format!("-device scsi-hd,bus=scsihw0.0,scsi-id={},drive=drive-scsi{},id=scsi{},rotation_rate=1{}",index,index,index,boot_index),
                ]
            }
            "ide-cd" => {
                vec![
                    format!(
                        "-drive file={},if=none,id=drive-ide{},media=cdrom,aio=io_uring",
                        self.file, index
                    ),
                    format!(
                        "-device ide-cd,bus=ide.{},unit=0,drive=drive-ide{},id=ide{},bootindex={}",
                        index, index, index, boot_index
                    ),
                ]
            }
            _ => {
                vec![]
            }
        }
    }
}
