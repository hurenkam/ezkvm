use serde::Deserialize;
use crate::yaml::QemuArgs;

#[derive(Debug,Deserialize)]
pub struct LookingGlassHost {
    path: String,
    size: String
}
impl QemuArgs for LookingGlassHost {
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
