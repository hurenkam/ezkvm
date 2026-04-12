//! VM state management
//!
//! Handles PID files, configuration caching, and VM state persistence.

use crate::config::VmConfig;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_LOG_KEEP: usize = 10;

/// Default directory for storing VM state (PID files, configs, logs)
pub fn get_state_dir() -> Result<PathBuf> {
    let state_dir = if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime).join("ezkvm")
    } else {
        let home =
            std::env::var("HOME").map_err(|_| anyhow!("HOME environment variable not set"))?;
        PathBuf::from(home).join(".local/run/ezkvm")
    };

    fs::create_dir_all(&state_dir)?;
    Ok(state_dir)
}

/// Get the PID file path for a VM
#[allow(dead_code)]
pub fn get_pid_file(vm_name: &str) -> Result<PathBuf> {
    get_pid_file_at(vm_name, None)
}

/// Get the PID file path for a VM, allowing a custom location.
pub fn get_pid_file_at(vm_name: &str, custom_path: Option<&str>) -> Result<PathBuf> {
    if let Some(custom_path) = custom_path {
        return Ok(PathBuf::from(custom_path));
    }

    let state_dir = get_state_dir()?;
    Ok(state_dir.join(format!("{}.pid", vm_name)))
}

/// Get the config cache path for a VM
pub fn get_config_cache(vm_name: &str) -> Result<PathBuf> {
    let state_dir = get_state_dir()?;
    Ok(state_dir.join(format!("{}.yaml", vm_name)))
}

/// Get the logs directory for a VM
#[allow(dead_code)]
pub fn get_logs_dir(vm_name: &str) -> Result<PathBuf> {
    get_logs_dir_at(vm_name, None)
}

/// Get the logs directory for a VM, allowing a custom location.
pub fn get_logs_dir_at(vm_name: &str, custom_dir: Option<&str>) -> Result<PathBuf> {
    let logs_dir = if let Some(custom_dir) = custom_dir {
        PathBuf::from(custom_dir).join(vm_name)
    } else {
        let state_dir = get_state_dir()?;
        state_dir.join("logs").join(vm_name)
    };

    fs::create_dir_all(&logs_dir)?;
    Ok(logs_dir)
}

/// Save VM PID to file
#[allow(dead_code)]
pub fn save_pid(vm_name: &str, pid: i32) -> Result<()> {
    save_pid_at(vm_name, pid, None)
}

/// Save VM PID to file at an optional custom location.
pub fn save_pid_at(vm_name: &str, pid: i32, custom_path: Option<&str>) -> Result<()> {
    let pid_file = get_pid_file_at(vm_name, custom_path)?;
    if let Some(parent) = pid_file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&pid_file, pid.to_string())?;
    Ok(())
}

/// Read VM PID from file
#[allow(dead_code)]
pub fn read_pid(vm_name: &str) -> Result<Option<i32>> {
    read_pid_at(vm_name, None)
}

/// Read VM PID from file at an optional custom location.
pub fn read_pid_at(vm_name: &str, custom_path: Option<&str>) -> Result<Option<i32>> {
    let pid_file = get_pid_file_at(vm_name, custom_path)?;

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
#[allow(dead_code)]
pub fn delete_pid(vm_name: &str) -> Result<()> {
    delete_pid_at(vm_name, None)
}

/// Delete VM PID file at an optional custom location.
pub fn delete_pid_at(vm_name: &str, custom_path: Option<&str>) -> Result<()> {
    let pid_file = get_pid_file_at(vm_name, custom_path)?;
    if pid_file.exists() {
        fs::remove_file(pid_file)?;
    }
    Ok(())
}

/// Get the log file for a VM
#[allow(dead_code)]
pub fn get_log_file(vm_name: &str, session: &str) -> Result<PathBuf> {
    get_log_file_at(vm_name, session, None)
}

/// Get the log file for a VM, allowing a custom logs directory.
pub fn get_log_file_at(vm_name: &str, session: &str, custom_dir: Option<&str>) -> Result<PathBuf> {
    let logs_dir = get_logs_dir_at(vm_name, custom_dir)?;
    Ok(logs_dir.join(format!("{}.log", session)))
}

/// Create a fresh session log file path using a timestamp-based session id.
pub fn create_session_log_file(vm_name: &str, custom_dir: Option<&str>) -> Result<PathBuf> {
    let session = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow!("Failed to get system time: {}", e))?
        .as_secs()
        .to_string();

    get_log_file_at(vm_name, &session, custom_dir)
}

/// Clean up old log files (keep last N)
#[allow(dead_code)]
pub fn cleanup_old_logs(vm_name: &str) -> Result<()> {
    cleanup_old_logs_at(vm_name, None, None)
}

/// Clean up old log files in an optional custom logs directory.
pub fn cleanup_old_logs_at(
    vm_name: &str,
    custom_dir: Option<&str>,
    keep: Option<usize>,
) -> Result<()> {
    let logs_dir = get_logs_dir_at(vm_name, custom_dir)?;
    let keep = keep.unwrap_or(DEFAULT_LOG_KEEP);

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

    if log_files.len() > keep {
        log_files.sort_by_key(|(_, mtime)| *mtime);
        while log_files.len() > keep {
            let (path, _) = log_files.remove(0);
            let _ = fs::remove_file(path);
        }
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

    #[test]
    fn test_custom_pid_file_operations() {
        let custom_dir = std::env::temp_dir().join("ezkvm-state-tests");
        let custom_path = custom_dir.join("custom.pid");
        let custom_str = custom_path.to_string_lossy().to_string();

        save_pid_at("custom-vm", 54321, Some(&custom_str)).unwrap();
        let pid = read_pid_at("custom-vm", Some(&custom_str)).unwrap();
        assert_eq!(pid, Some(54321));
        delete_pid_at("custom-vm", Some(&custom_str)).unwrap();
        let pid = read_pid_at("custom-vm", Some(&custom_str)).unwrap();
        assert_eq!(pid, None);
        let _ = fs::remove_dir_all(custom_dir);
    }

    #[test]
    fn test_log_rotation_cleanup() {
        let custom_dir = std::env::temp_dir().join("ezkvm-log-tests");
        let custom_str = custom_dir.to_string_lossy().to_string();

        for index in 0..3 {
            let log_file =
                get_log_file_at("log-vm", &format!("session-{}", index), Some(&custom_str))
                    .unwrap();
            fs::write(log_file, format!("log {}", index)).unwrap();
        }

        cleanup_old_logs_at("log-vm", Some(&custom_str), Some(2)).unwrap();
        let logs_dir = get_logs_dir_at("log-vm", Some(&custom_str)).unwrap();
        let remaining = fs::read_dir(logs_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "log")
                    .unwrap_or(false)
            })
            .count();

        assert_eq!(remaining, 2);
        let _ = fs::remove_dir_all(custom_dir);
    }
}
