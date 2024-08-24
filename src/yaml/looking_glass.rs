use serde::Deserialize;
use crate::yaml::{LgClientArgs, QemuArgs};

#[derive(Debug,Deserialize)]
pub struct LookingGlass {
    path: String,
    size: String,
    grab_keyboard: bool,
    escape_key: String,
    win_size: String,
    full_screen: bool
}
impl QemuArgs for LookingGlass {
    fn get_qemu_args(&self, index: usize) -> Vec<String> {
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

// looking-glass-client app:shmFile=/dev/kvmfr0 spice:host=0.0.0.0 spice:port=5903 input:escapeKey=KEY_F12 input:grabKeyboard win:size=1707x1067 win:fullscreen
impl LgClientArgs for LookingGlass {
    fn get_lg_client_args(&self, index: usize) -> Vec<String> {
        vec![
            format!("app:shmFile={}",self.path),
            format!("input:grabKeyboard={}",self.grab_keyboard),
            format!("input:escapeKey={}",self.escape_key),
            format!("win:fullScreen={}",self.full_screen),
            format!("win:size={}",self.win_size),
        ]
    }
}
