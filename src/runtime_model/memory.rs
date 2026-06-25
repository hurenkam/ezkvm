use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::runtime_model::Cpu;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, new)]
pub struct Memory {
    size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hugepages_kb: Option<usize>,
    #[serde(default)]
    numa_enabled: bool,
}
impl Memory {
    pub fn kilobytes(kb: usize) -> Self {
        Self {
            size: kb * 1024,
            hugepages_kb: None,
            numa_enabled: false,
        }
    }
    pub fn megabytes(mb: usize) -> Self {
        Self {
            size: mb * 1024 * 1024,
            hugepages_kb: None,
            numa_enabled: false,
        }
    }
    pub fn gigabytes(gb: usize) -> Self {
        Self {
            size: gb * 1024 * 1024 * 1024,
            hugepages_kb: None,
            numa_enabled: false,
        }
    }
    pub fn terabytes(tb: usize) -> Self {
        Self {
            size: tb * 1024 * 1024 * 1024 * 1024,
            hugepages_kb: None,
            numa_enabled: false,
        }
    }

    pub fn with_hugepages_kb(mut self, hugepages_kb: Option<usize>) -> Self {
        self.hugepages_kb = hugepages_kb.filter(|value| *value > 0);
        self
    }

    pub fn with_numa_enabled(mut self, numa_enabled: bool) -> Self {
        self.numa_enabled = numa_enabled;
        self
    }

    pub fn qemu_args(&self, cpu: &Cpu) -> Vec<String> {
        let size_mb = self.size / 1024 / 1024;
        let mut args = vec!["-m".to_string(), format!("{size_mb}M")];

        if let Some(hugepages_kb) = self.hugepages_kb {
            args.extend([
                "-object".to_string(),
                format!(
                    "memory-backend-file,id=ram-node0,size={size_mb}M,mem-path=/run/hugepages/kvm/{hugepages_kb}kB,share=on,prealloc=yes"
                ),
            ]);
        }

        if self.numa_enabled {
            let total_vcpus = cpu.total_vcpus().max(1);
            let cpus = if total_vcpus == 1 {
                "0".to_string()
            } else {
                format!("0-{}", total_vcpus - 1)
            };

            let node = if self.hugepages_kb.is_some() {
                format!("node,nodeid=0,cpus={cpus},memdev=ram-node0")
            } else {
                format!("node,nodeid=0,cpus={cpus},mem={size_mb}")
            };

            args.extend(["-numa".to_string(), node]);
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::Memory;
    use crate::runtime_model::{Cpu, CpuModel};

    #[test]
    fn emits_hugepages_object_and_numa_memdev_when_configured() {
        let memory = Memory::megabytes(16384)
            .with_hugepages_kb(Some(1048576))
            .with_numa_enabled(true);
        let cpu = Cpu::new(CpuModel::Host, 8, 1, 1);

        let args = memory.qemu_args(&cpu);

        assert!(args.iter().any(|arg| arg == "-m"));
        assert!(args.iter().any(|arg| arg == "16384M"));
        assert!(args.iter().any(|arg| {
            arg.contains("memory-backend-file")
                && arg.contains("mem-path=/run/hugepages/kvm/1048576kB")
                && arg.contains("prealloc=yes")
        }));
        assert!(
            args.iter()
                .any(|arg| arg.contains("node,nodeid=0,cpus=0-7,memdev=ram-node0"))
        );
    }
}
