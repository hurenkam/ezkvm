use serde::{Deserialize, Serialize};

/// Hugepages memory backend configuration.
///
/// When present and enabled, the QEMU command builder emits
/// `-object memory-backend-file,...` objects for each NUMA node
/// (or a single node synthesized from total memory when no explicit NUMA
/// topology is defined). Each NUMA node entry is then bound to its
/// corresponding memory backend via `memdev=ram-node{N}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HugepagesConfig {
    /// Enable hugepages-backed memory.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Hugepage size in KiB (e.g. 2048 = 2 MiB, 1048576 = 1 GiB).
    /// When `None`, the system default hugepage size is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_kib: Option<u64>,

    /// Path to the hugepages mount (e.g. `/run/hugepages/kvm/1048576kB`).
    /// When `None`, a path is derived from `size_kib` using the Proxmox
    /// convention `/run/hugepages/kvm/{size_kib}kB`, or
    /// `/dev/hugepages` when no size is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem_path: Option<String>,

    /// Pre-allocate hugepages at startup (QEMU `prealloc=yes`).
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub prealloc: bool,
}

fn default_true() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

impl HugepagesConfig {
    /// Resolve the effective hugepages mount path.
    pub fn effective_mem_path(&self) -> String {
        if let Some(path) = &self.mem_path {
            return path.clone();
        }
        match self.size_kib {
            Some(kib) => format!("/run/hugepages/kvm/{}kB", kib),
            None => "/dev/hugepages".to_string(),
        }
    }
}
