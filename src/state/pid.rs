use anyhow::Result;
use std::fs;

use super::get_pid_file_at;

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
