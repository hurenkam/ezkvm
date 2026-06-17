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
    fn normalized_cores(&self) -> u8 {
        self.cores.max(1)
    }

    fn normalized_threads(&self) -> u8 {
        self.threads.max(1)
    }

    fn normalized_sockets(&self) -> u8 {
        self.sockets.max(1)
    }

    fn model_name(&self) -> &'static str {
        match self.model {
            CpuModel::Host => "host",
        }
    }

    pub fn qemu_args(&self) -> Vec<String> {
        let sockets = self.normalized_sockets();
        let cores = self.normalized_cores();
        let threads = self.normalized_threads();
        let total_vcpus = sockets as u32 * cores as u32 * threads as u32;

        vec![
            "-cpu".to_string(),
            self.model_name().to_string(),
            "-smp".to_string(),
            format!("{total_vcpus},sockets={sockets},cores={cores},threads={threads}"),
        ]
    }
}
