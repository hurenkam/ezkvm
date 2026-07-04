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

impl Cpu {
    pub fn model(&self) -> &CpuModel {
        &self.model
    }

    pub fn cores(&self) -> u8 {
        self.cores
    }

    pub fn threads(&self) -> u8 {
        self.threads
    }

    pub fn sockets(&self) -> u8 {
        self.sockets
    }

    fn normalized_cores(&self) -> u8 {
        self.cores.max(1)
    }

    fn normalized_threads(&self) -> u8 {
        self.threads.max(1)
    }

    fn normalized_sockets(&self) -> u8 {
        self.sockets.max(1)
    }

    pub fn total_vcpus(&self) -> u32 {
        self.normalized_sockets() as u32
            * self.normalized_cores() as u32
            * self.normalized_threads() as u32
    }
}
