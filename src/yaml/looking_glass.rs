use serde::Deserialize;
use crate::yaml::{LgClientArgs, QemuArgs};

#[derive(Debug,Deserialize,PartialEq)]
pub struct LookingGlass {
    device: Device,
    window: Option<Window>,
    input: Option<Input>
}
impl QemuArgs for LookingGlass {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![
            "-vga none".to_string(),
            "-nographic".to_string(),
            "-device virtio-mouse".to_string(),
            "-device virtio-keyboard".to_string(),
        ];

        result.extend(self.device.get_qemu_args(index));

        result
    }
}

impl LgClientArgs for LookingGlass {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
        let mut result = vec![];

        result.extend(self.device.get_lg_client_args(index));
        if let Some(window) = &self.window {
            result.extend(window.get_lg_client_args(index));
        }
        if let Some(input) = &self.input {
            result.extend(input.get_lg_client_args(index));
        }

        result
    }
}

#[derive(Debug,Deserialize,PartialEq)]
pub struct Device {
    path: String,
    size: String
}

impl QemuArgs for Device {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("-device ivshmem-plain,memdev=ivshmem{},bus=pcie.0",index),
            format!("-object memory-backend-file,id=ivshmem{},share=on,mem-path={},size={}",index,self.path,self.size),
        ]
    }
}

impl LgClientArgs for Device {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("app:shmFile={}",self.path),
        ]
    }
}

#[derive(Debug,Deserialize,PartialEq)]
pub struct Window {
    size: String,
    full_screen: bool
}

impl LgClientArgs for Window {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("win:fullScreen={}",self.full_screen),
            format!("win:size={}",self.size),
        ]
    }
}

#[derive(Debug,Deserialize,PartialEq)]
pub struct Input {
    grab_keyboard: bool,
    escape_key: String,
}

impl LgClientArgs for Input {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("input:grabKeyboard={}",self.grab_keyboard),
            format!("input:escapeKey={}",self.escape_key),
        ]
    }
}
