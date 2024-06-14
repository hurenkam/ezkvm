use serde::Deserialize;
use crate::config::config::QemuDevice;
use crate::config::storage::StorageDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct IdeCdromDevice {
    file: String,
    boot_index: Option<String>
}

impl QemuDevice for IdeCdromDevice {
    fn get_args(&self, index: usize) -> Vec<String> {
        let boot_index = match self.boot_index.clone() {
            None => "".to_string(),
            Some(boot_index) => format!(",bootindex={}",boot_index)
        };
        vec![
            format!("-drive file={},if=none,id=drive-ide{},media=cdrom,aio=io_uring",self.file,index),
            format!("-device ide-cd,bus=ide.{},unit=0,drive=drive-ide{},id=ide{},bootindex={}",index,index,index,boot_index),
        ]
    }
}

#[typetag::deserialize(name = "ide-cd")]
impl StorageDevice for IdeCdromDevice {}
