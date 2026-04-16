use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

use super::super::{
    BallooningConfig, HugepagesConfig, IvshmemConfig, NumaConfig, SmbiosConfig, TpmConfig,
};
use super::BootConfig;

/// System-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Target architecture (x86_64, aarch64, etc.)
    pub architecture: String,

    /// Machine type (q35, pc, virt, etc.)
    pub machine: String,

    /// Additional machine-specific options appended to `-machine`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub machine_options: Vec<String>,

    /// Canonical memory configuration.
    pub memory: MemoryConfig,

    /// Canonical CPU configuration.
    #[serde(default)]
    pub cpu: CpuConfig,

    /// Canonical boot configuration.
    #[serde(default)]
    pub boot: BootConfig,

    /// Canonical TPM configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tpm: Option<TpmConfig>,

    /// Canonical SMBIOS configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smbios: Option<SmbiosConfig>,

    /// Optional QEMU config file(s) to load via -readconfig.
    /// Use this to supply machine topology files such as
    /// /usr/share/qemu-server/pve-q35-4.0.cfg which define the
    /// PCI/PCIe bridge buses (pci.0, pci.1, ich9-pcie-port-*, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readconfig: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory in MiB.
    pub size: u32,

    /// Optional canonical memory ballooning configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ballooning: Option<BallooningConfig>,

    /// Optional canonical ivshmem configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ivshmem: Option<IvshmemConfig>,

    /// Optional hugepages memory backend configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hugepages: Option<HugepagesConfig>,
}

/// Canonical CPU configuration nested under `system.cpu`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuConfig {
    /// CPU model to emulate
    #[serde(default)]
    pub model: String,

    /// Number of virtual CPUs
    #[serde(default)]
    pub vcpus: u32,

    /// CPU-specific features
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,

    /// NUMA topology configuration
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numa: Vec<NumaConfig>,
}

impl From<SystemConfig> for QemuArgs {
    fn from(config: SystemConfig) -> Self {
        let mut args = QemuArgs::new();

        // Machine type
        args.push_str("-machine");
        let mut machine_spec = format!("type={}", config.machine);
        for option in config.machine_options {
            machine_spec.push(',');
            machine_spec.push_str(&option);
        }
        args.push(machine_spec);

        // CPU configuration
        args.push_str("-cpu");
        let mut cpu_spec = config.cpu.model.clone();
        if !config.cpu.features.is_empty() {
            cpu_spec.push(',');
            cpu_spec.push_str(&config.cpu.features.join(","));
        }
        args.push(cpu_spec);

        // Memory
        args.push_str("-m");
        args.push(format!("{}M", config.memory.size));

        // SMP (symmetric multiprocessing)
        args.push_str("-smp");
        args.push(format!("cpus={}", config.cpu.vcpus));

        args
    }
}
