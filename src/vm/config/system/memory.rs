use super::QemuDevice;
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Memory {
    max: u32,
    #[serde(default)]
    balloon: Option<bool>,
    /// Enable huge page backing. Sets mem_path to /dev/hugepages when no
    /// explicit mem_path is provided.
    #[serde(default)]
    hugepages: Option<bool>,
    /// Pre-allocate all guest memory at startup.
    #[serde(default)]
    prealloc: Option<bool>,
    /// Custom path for memory backing file (e.g. /dev/hugepages or a tmpfs
    /// mount). Overrides the path implied by `hugepages: true`.
    #[serde(default)]
    mem_path: Option<String>,
}

impl Memory {
    pub fn new(max: u32, balloon: Option<bool>) -> Self {
        Self {
            max,
            balloon,
            hugepages: None,
            prealloc: None,
            mem_path: None,
        }
    }

    /// True when hugepages backing is enabled.
    pub fn has_hugepages(&self) -> bool {
        self.hugepages == Some(true)
    }

    /// The resolved backing path: explicit `mem_path` wins, otherwise
    /// `/dev/hugepages` when `hugepages: true`.
    pub fn resolved_mem_path(&self) -> Option<&str> {
        self.mem_path.as_deref().or_else(|| {
            if self.hugepages == Some(true) {
                Some("/dev/hugepages")
            } else {
                None
            }
        })
    }

    /// The QEMU `-m` argument only, without any backing-file args.
    /// Used when NUMA memory backends take over from the flat memory args.
    pub fn get_size_arg(&self) -> String {
        format!("-m {}", self.max)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            max: 16384,
            balloon: None,
            hugepages: None,
            prealloc: None,
            mem_path: None,
        }
    }
}

impl QemuDevice for Memory {
    fn get_qemu_args(&self, _index: usize) -> Vec<String> {
        let mut result = vec![format!("-m {}", self.max)];

        // Resolve backing path: explicit mem_path wins, otherwise /dev/hugepages
        // when hugepages: true.
        let resolved_path = self.mem_path.as_deref().or_else(|| {
            if self.hugepages == Some(true) {
                Some("/dev/hugepages")
            } else {
                None
            }
        });

        if let Some(path) = resolved_path {
            result.push(format!("-mem-path {}", path));
        }

        if self.prealloc == Some(true) || self.hugepages == Some(true) {
            result.push("-mem-prealloc".to_string());
        }

        if self.balloon == Some(true) {
            result.push("-device virtio-balloon-pci".to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let memory = Memory::default();
        let expected: Vec<String> = vec!["-m 16384".to_string()];
        assert_eq!(memory.get_qemu_args(0), expected);
    }

    #[test]
    fn test_balloon_true() {
        let memory = Memory {
            max: 4096,
            balloon: Some(true),
            hugepages: None,
            prealloc: None,
            mem_path: None,
        };
        let args = memory.get_qemu_args(0);
        assert!(args.contains(&"-device virtio-balloon-pci".to_string()));
    }

    #[test]
    fn test_balloon_false() {
        let memory = Memory {
            max: 4096,
            balloon: Some(false),
            hugepages: None,
            prealloc: None,
            mem_path: None,
        };
        let args = memory.get_qemu_args(0);
        assert!(!args.contains(&"-device virtio-balloon-pci".to_string()));
    }

    #[test]
    fn test_hugepages_sets_mem_path_and_prealloc() {
        let memory = Memory {
            max: 4096,
            balloon: None,
            hugepages: Some(true),
            prealloc: None,
            mem_path: None,
        };
        let args = memory.get_qemu_args(0);
        assert!(args.contains(&"-mem-path /dev/hugepages".to_string()));
        assert!(args.contains(&"-mem-prealloc".to_string()));
    }

    #[test]
    fn test_explicit_mem_path_overrides_hugepages_path() {
        let memory = Memory {
            max: 4096,
            balloon: None,
            hugepages: Some(true),
            prealloc: None,
            mem_path: Some("/mnt/huge1g".to_string()),
        };
        let args = memory.get_qemu_args(0);
        assert!(args.contains(&"-mem-path /mnt/huge1g".to_string()));
        assert!(!args.contains(&"-mem-path /dev/hugepages".to_string()));
    }

    #[test]
    fn test_prealloc_alone() {
        let memory = Memory {
            max: 2048,
            balloon: None,
            hugepages: None,
            prealloc: Some(true),
            mem_path: None,
        };
        let args = memory.get_qemu_args(0);
        assert!(args.contains(&"-mem-prealloc".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("-mem-path")));
    }

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
            max: 8192
            hugepages: true
            balloon: true
        "#;
        let memory: Memory = serde_yaml::from_str(yaml).unwrap();
        let args = memory.get_qemu_args(0);
        assert!(args.contains(&"-m 8192".to_string()));
        assert!(args.contains(&"-mem-path /dev/hugepages".to_string()));
        assert!(args.contains(&"-mem-prealloc".to_string()));
        assert!(args.contains(&"-device virtio-balloon-pci".to_string()));
    }
}
