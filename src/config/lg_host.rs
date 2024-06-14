use std::fmt::Debug;
use serde::Deserialize;
use crate::config::config::QemuDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct LgHost {
    path: String,
    size: String
}

impl QemuDevice for LgHost {
    fn get_args(&self, index: usize) -> Vec<String> {
        vec![
            "-vga none".to_string(),
            "-nographic".to_string(),
            "-device virtio-mouse".to_string(),
            "-device virtio-keyboard".to_string(),
            format!("-device ivshmem-plain,memdev=ivshmem{},bus=pcie.0",index),
            format!("-object memory-backend-file,id=ivshmem{},share=on,mem-path={},size={}",index,self.path,self.size),
        ]
    }
}