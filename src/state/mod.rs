//! VM state management
//!
//! Handles PID files, configuration caching, and VM state persistence.

mod cache;
mod logs;
mod paths;
mod pid;
mod runtime_resolver;
mod tpm_resolver;

#[allow(unused_imports)]
pub use cache::{cache_config, delete_cached_config, load_cached_config};
#[allow(unused_imports)]
pub use logs::{cleanup_old_logs, cleanup_old_logs_at};
#[allow(unused_imports)]
pub use paths::{
    create_session_log_file, get_config_cache, get_log_file, get_log_file_at, get_logs_dir,
    get_logs_dir_at, get_pid_file, get_pid_file_at, get_state_dir,
};
#[allow(unused_imports)]
pub use pid::{delete_pid, delete_pid_at, read_pid, read_pid_at, save_pid, save_pid_at};
#[allow(unused_imports)]
pub use runtime_resolver::{
    CentralRuntimeCapabilityResolver, RuntimeCapabilityResolver, resolve_runtime_root,
    resolve_runtime_tpm_socket,
};
#[allow(unused_imports)]
pub use tpm_resolver::{
    CentralTpmCapabilityResolver, TpmCapabilityResolver, TpmPlacementMode, resolve_swtpm_binary,
    resolve_tpm_placement_mode, resolve_tpm_socket_path, resolve_tpm_state_dir,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
