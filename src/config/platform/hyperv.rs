use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

/// Hyper-V enlightenments configuration.
/// Kept as a single type because feature flags are serialized/deserialized together and
/// map directly to one cohesive QEMU Hyper-V feature block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypervConfig {
    /// Enable Hyper-V enlightenments
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Enable Hyper-V relaxed timing
    #[serde(default = "default_true")]
    pub relaxed: bool,

    /// Enable Hyper-V virtual APIC
    #[serde(default = "default_true")]
    pub vapic: bool,

    /// Enable Hyper-V time reference counter
    #[serde(default = "default_true")]
    pub time: bool,

    /// Enable Hyper-V crash MSRs
    #[serde(default)]
    pub crash: bool,

    /// Enable Hyper-V reset MSR
    #[serde(default)]
    pub reset: bool,

    /// Enable Hyper-V vendor ID spoofing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,

    /// Enable Hyper-V frequency MSRs
    #[serde(default)]
    pub frequencies: bool,

    /// Enable Hyper-V reenlightenment MSRs
    #[serde(default)]
    pub reenlightenment: bool,

    /// Enable Hyper-V TLB flush
    #[serde(default)]
    pub tlbflush: bool,

    /// Enable Hyper-V IPI optimization
    #[serde(default)]
    pub ipi: bool,

    /// Enable Hyper-V spinlock retry
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spinlock_retry: Option<u32>,
}
