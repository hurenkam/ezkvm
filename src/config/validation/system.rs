use anyhow::{Result, anyhow};

use crate::config::SystemConfig;

pub(crate) fn validate_system_config(system: &SystemConfig) -> Result<()> {
    let valid_architectures = ["x86_64", "aarch64", "x86", "ppc64", "riscv64"];
    if !valid_architectures.contains(&system.architecture.as_str()) {
        return Err(anyhow!(
            "Unsupported architecture: {}. Supported: {:?}",
            system.architecture,
            valid_architectures
        ));
    }

    if system.memory < 128 {
        return Err(anyhow!("Memory must be at least 128 MiB"));
    }
    if system.memory > 1024 * 1024 {
        return Err(anyhow!("Memory cannot exceed 1 TiB"));
    }

    if system.cpu.vcpus == 0 {
        return Err(anyhow!("Must have at least 1 vCPU"));
    }
    if system.cpu.vcpus > 1024 {
        return Err(anyhow!("Cannot have more than 1024 vCPUs"));
    }

    if system.cpu.model.trim().is_empty() {
        return Err(anyhow!("CPU model cannot be empty"));
    }

    for feature in &system.cpu.features {
        if feature.trim().is_empty() {
            return Err(anyhow!("CPU feature name cannot be empty"));
        }
    }

    for option in &system.machine_options {
        if option.trim().is_empty() {
            return Err(anyhow!("Machine options cannot be empty"));
        }
        if !option.contains('=') {
            return Err(anyhow!(
                "Machine option '{}' must use key=value format",
                option
            ));
        }
    }

    Ok(())
}
