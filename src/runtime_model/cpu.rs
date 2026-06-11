use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum CpuModel {
    #[default]
    Host,
}
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct Cpu {
    model: CpuModel,
    cores: u8,
    threads: u8,
    sockets: u8,
}
