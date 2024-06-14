use serde::Deserialize;
use crate::config::config::QemuDevice;

#[derive(Deserialize,Debug,Clone)]
pub struct Cpu {
    model: String,
    sockets: u32,
    cores: u32,
    flags: String
}
impl Default for Cpu {
    fn default() -> Self {
        Self {
            model: "qemu64".to_string(),
            sockets: 1,
            cores: 1,
            flags: "".to_string(),
        }
    }
}
impl QemuDevice for Cpu {
    fn get_args(&self, index: usize) -> Vec<String> {
        let total = self.sockets * self.cores;
        vec![
            format!("-smp {},sockets={},cores={},maxcpus={}", total,self.sockets,self.cores,total),
            format!("-cpu {},{}", self.model,self.flags),
        ]
    }
}
