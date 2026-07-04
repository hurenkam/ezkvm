use derive_new::new;
use serde::{Deserialize, Serialize};

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
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn hugepages_kb(&self) -> Option<usize> {
        self.hugepages_kb
    }

    pub fn numa_enabled(&self) -> bool {
        self.numa_enabled
    }

    #[allow(dead_code)] // TODO: wire to CLI
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
    #[allow(dead_code)] // TODO: wire to CLI
    pub fn gigabytes(gb: usize) -> Self {
        Self {
            size: gb * 1024 * 1024 * 1024,
            hugepages_kb: None,
            numa_enabled: false,
        }
    }
    #[allow(dead_code)] // TODO: wire to CLI
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
}

#[cfg(test)]
mod tests {
    use super::Memory;

    #[test]
    fn stores_hugepages_and_numa_settings() {
        let memory = Memory::megabytes(16384)
            .with_hugepages_kb(Some(1048576))
            .with_numa_enabled(true);

        assert_eq!(memory.size(), 16384 * 1024 * 1024);
        assert_eq!(memory.hugepages_kb(), Some(1048576));
        assert!(memory.numa_enabled());
    }
}
