use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
