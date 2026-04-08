//! VM state management
//!
//! Handles PID files, configuration caching, and VM state persistence.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{PathBuf};
use crate::config::VmConfig;

/// Default directory for storing VM state (PID files, configs, logs)
pub fn get_state_dir() -> Result<PathBuf> {
    let state_dir = if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime).join("ezkvm")
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow!("HOME environment variable not set"))?;
        PathBuf::from(home).join(".local/run/ezkvm")
    };
    
    fs::create_dir_all(&state_dir)?;
    Ok(state_dir)
}

/// Get the PID file path for a VM
pub fn get_pid_file(vm_name: &str) -> Result<PathBuf> {
    let state_dir = get_state_dir()?;
    Ok(state_dir.join(format!("{}.pid", vm_name)))
}

/// Get the config cache path for a VM
pub fn get_config_cache(vm_name: &str) -> Result<PathBuf> {
    let state_dir = get_state_dir()?;
    Ok(state_dir.join(format!("{}.yaml", vm_name)))
}

/// Get the logs directory for a VM
pub fn get_logs_dir(vm_name: &str) -> Result<PathBuf> {
    let state_dir = get_state_dir()?;
    let logs_dir = state_dir.join("logs").join(vm_name);
    fs::create_dir_all(&logs_dir)?;
    Ok(logs_dir)
}

/// Save VM PID to file
pub fn save_pid(vm_name: &str, pid: i32) -> Result<()> {
    let pid_file = get_pid_file(vm_name)?;
    fs::write(&pid_file, pid.to_string())?;
    Ok(())
}

/// Read VM PID from file
pub fn read_pid(vm_name: &str) -> Result<Option<i32>> {
    let pid_file = get_pid_file(vm_name)?;
    
    if !pid_file.exists() {
        return Ok(None);
    }
    
    let content = fs::read_to_string(&pid_file)?;
    match content.trim().parse::<i32>() {
        Ok(pid) => Ok(Some(pid)),
        Err(_) => Ok(None),
    }
}

/// Delete VM PID file
pub fn delete_pid(vm_name: &str) -> Result<()> {
    let pid_file = get_pid_file(vm_name)?;
    if pid_file.exists() {
        fs::remove_file(pid_file)?;
    }
    Ok(())
}

/// Cache VM configuration
pub fn cache_config(vm_name: &str, config: &VmConfig) -> Result<()> {
    let config_file = get_config_cache(vm_name)?;
    let yaml = serde_yaml::to_string(config)?;
    fs::write(&config_file, yaml)?;
    Ok(())
}

/// Load cached VM configuration
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
pub fn delete_cached_config(vm_name: &str) -> Result<()> {
    let config_file = get_config_cache(vm_name)?;
    if config_file.exists() {
        fs::remove_file(config_file)?;
    }
    Ok(())
}

/// Get the log file for a VM
pub fn get_log_file(vm_name: &str, session: &str) -> Result<PathBuf> {
    let logs_dir = get_logs_dir(vm_name)?;
    Ok(logs_dir.join(format!("{}.log", session)))
}

/// Clean up old log files (keep last 10)
pub fn cleanup_old_logs(vm_name: &str) -> Result<()> {
    let logs_dir = get_logs_dir(vm_name)?;
    
    let mut log_files: Vec<_> = fs::read_dir(&logs_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "log").unwrap_or(false) {
                let metadata = entry.metadata().ok()?;
                let modified = metadata.modified().ok()?;
                Some((path, modified))
            } else {
                None
            }
        })
        .collect();
    
    if log_files.len() > 10 {
        // Sort by modification time
        log_files.sort_by_key(|(_, mtime)| *mtime);
        
        // Remove oldest files (keep 10)
        while log_files.len() > 10 {
            let (path, _) = log_files.remove(0);
            let _ = fs::remove_file(path);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_dir_creation() {
        let state_dir = get_state_dir();
        assert!(state_dir.is_ok());
    }

    #[test]
    fn test_pid_file_operations() {
        let test_vm = "test-vm-state";
        
        // Save PID
        let result = save_pid(test_vm, 12345);
        assert!(result.is_ok());
        
        // Read PID
        let pid = read_pid(test_vm).unwrap();
        assert_eq!(pid, Some(12345));
        
        // Delete PID
        let result = delete_pid(test_vm);
        assert!(result.is_ok());
        
        // Verify deleted
        let pid = read_pid(test_vm).unwrap();
        assert_eq!(pid, None);
    }
}
