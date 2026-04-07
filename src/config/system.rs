//! System configuration module
//!
//! Handles CPU, memory, and machine-specific configuration.

use super::{SystemConfig, CpuFeature};
use crate::qemu::types::QemuArgs;

impl SystemConfig {
    /// Get the QEMU system binary name for this architecture
    pub fn qemu_binary(&self) -> String {
        format!("qemu-system-{}", self.architecture)
    }
}

impl From<SystemConfig> for QemuArgs {
    fn from(config: SystemConfig) -> Self {
        let mut args = QemuArgs::new();
        
        // Machine type
        args.push_str("-machine");
        args.push(format!("type={}", config.machine));
        
        // CPU configuration
        args.push_str("-cpu");
        let mut cpu_spec = config.cpu_model.clone();
        if !config.cpu_features.is_empty() {
            let features: Vec<String> = config.cpu_features.iter()
                .map(|f| f.name.clone())
                .collect();
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

impl CpuFeature {
    /// Create a new CPU feature
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
    
    /// Check if this is an enabled feature (+prefix)
    pub fn is_enabled(&self) -> bool {
        self.name.starts_with('+')
    }
    
    /// Check if this is a disabled feature (-prefix)
    pub fn is_disabled(&self) -> bool {
        self.name.starts_with('-')
    }
}