use crate::qemu::types::QemuArgs;
use serde::{Deserialize, Serialize};

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

    /// Number of virtual CPUs
    pub vcpus: u32,

    /// CPU model to emulate
    pub cpu_model: String,

    /// CPU-specific features
    #[serde(default)]
    pub cpu_features: Vec<CpuFeature>,

    /// Optional QEMU config file(s) to load via -readconfig.
    /// Use this to supply machine topology files such as
    /// /usr/share/qemu-server/pve-q35-4.0.cfg which define the
    /// PCI/PCIe bridge buses (pci.0, pci.1, ich9-pcie-port-*, etc.)
    #[serde(default)]
    pub readconfig: Vec<String>,
}

/// CPU feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFeature {
    /// Feature name (e.g., "+vmx", "-avx")
    pub name: String,
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
        let mut cpu_spec = config.cpu_model.clone();
        if !config.cpu_features.is_empty() {
            let features: Vec<String> =
                config.cpu_features.iter().map(|f| f.name.clone()).collect();
            cpu_spec.push(',');
            cpu_spec.push_str(&features.join(","));
        }
        args.push(cpu_spec);

        // Memory
        args.push_str("-m");
        args.push(format!("{}M", config.memory));

        // SMP (symmetric multiprocessing)
        args.push_str("-smp");
        args.push(format!("cpus={}", config.vcpus));

        args
    }
}
