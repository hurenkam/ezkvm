use crate::config::VmConfig;
use anyhow::Result;
use std::fs;

use super::get_config_cache;

/// Cache VM configuration
pub fn cache_config(vm_name: &str, config: &VmConfig) -> Result<()> {
    let config_file = get_config_cache(vm_name)?;
    let yaml = serde_yaml::to_string(config)?;
    fs::write(&config_file, yaml)?;
    Ok(())
}

/// Load cached VM configuration
#[allow(dead_code)]
pub fn load_cached_config(vm_name: &str) -> Result<Option<VmConfig>> {
    let config_file = get_config_cache(vm_name)?;

    if !config_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_file)?;
    let config = serde_yaml::from_str(&content)?;
    Ok(Some(config))
}

/// Delete cached VM configuration
#[allow(dead_code)]
pub fn delete_cached_config(vm_name: &str) -> Result<()> {
    let config_file = get_config_cache(vm_name)?;
    if config_file.exists() {
        fs::remove_file(config_file)?;
    }
    Ok(())
}
