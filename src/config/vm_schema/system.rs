use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Deserializer, Serialize};

use super::super::NumaConfig;

/// System-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Target architecture (x86_64, aarch64, etc.)
    pub architecture: String,

    /// Machine type (q35, pc, virt, etc.)
    pub machine: String,

    /// Additional machine-specific options appended to `-machine`
    #[serde(default)]
    pub machine_options: Vec<String>,

    /// Memory in MiB
    pub memory: u32,

    /// Canonical CPU configuration.
    #[serde(default)]
    pub cpu: CpuConfig,

    /// Legacy number of virtual CPUs (kept for compatibility input).
    #[serde(default)]
    pub vcpus: u32,

    /// Legacy CPU model (kept for compatibility input).
    #[serde(default)]
    pub cpu_model: String,

    /// Legacy CPU-specific features (kept for compatibility input).
    #[serde(default, deserialize_with = "deserialize_cpu_feature_list")]
    pub cpu_features: Vec<String>,

    /// Optional QEMU config file(s) to load via -readconfig.
    /// Use this to supply machine topology files such as
    /// /usr/share/qemu-server/pve-q35-4.0.cfg which define the
    /// PCI/PCIe bridge buses (pci.0, pci.1, ich9-pcie-port-*, etc.)
    #[serde(default)]
    pub readconfig: Vec<String>,
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
    #[serde(default, deserialize_with = "deserialize_cpu_feature_list")]
    pub features: Vec<String>,

    /// NUMA topology configuration
    #[serde(default)]
    pub numa: Vec<NumaConfig>,
}

impl SystemConfig {
    pub fn normalize_cpu_fields(&mut self, legacy_numa: Vec<NumaConfig>) {
        // Canonical system.cpu wins whenever present.
        if self.cpu.model.trim().is_empty() {
            self.cpu.model = self.cpu_model.trim().to_string();
        }
        if self.cpu.vcpus == 0 {
            self.cpu.vcpus = self.vcpus;
        }
        if self.cpu.features.is_empty() {
            self.cpu.features = self.cpu_features.clone();
        }
        if self.cpu.numa.is_empty() {
            self.cpu.numa = legacy_numa;
        }

        // Mirror canonical values back to legacy fields for compatibility callers.
        self.cpu_model = self.cpu.model.clone();
        self.vcpus = self.cpu.vcpus;
        self.cpu_features = self.cpu.features.clone();
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CpuFeatureEntry {
    Name(String),
    Named { name: String },
}

fn deserialize_cpu_feature_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let entries = Vec::<CpuFeatureEntry>::deserialize(deserializer)?;
    Ok(entries
        .into_iter()
        .map(|entry| match entry {
            CpuFeatureEntry::Name(value) => value,
            CpuFeatureEntry::Named { name } => name,
        })
        .collect())
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
        args.push(format!("{}M", config.memory));

        // SMP (symmetric multiprocessing)
        args.push_str("-smp");
        args.push(format!("cpus={}", config.cpu.vcpus));

        args
    }
}
