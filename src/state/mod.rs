//! VM state management, runtime path resolution, and capability detection.
//!
//! This module manages VM lifecycle tracking and resolves runtime paths following
//! the portable-runtime precedence contract. It handles:
//! - PID files and process lifecycle
//! - Configuration caching for status/inspection commands
//! - Socket path resolution with precedence (TPM, guest-agent, QMP)
//! - Runtime capability detection (Proxmox parity vs portable Linux)
//! - Network capability negotiation and resolution
//! - Looking Glass program resolution with fallback chains
//!
//! # Precedence Model
//! All path resolution follows: CLI > central config > platform defaults > fallback
//!
//! # Design Patterns
//! - Trait-based capability resolvers for extension boundaries
//! - No global singletons; all resolution is explicit and dependency-injected
//! - Centralized error context with anyhow for actionable error messages

mod cache;
mod capability_precedence;
mod logs;
mod looking_glass_resolver;
mod network_resolver;
mod paths;
mod pid;
mod runtime_resolver;
mod shutdown;
mod tpm_resolver;
mod vm_state;

#[allow(unused_imports)]
pub use cache::{cache_config, delete_cached_config, load_cached_config};
#[allow(unused_imports)]
pub use capability_precedence::{
    CapabilityPrecedenceResolver, CapabilityResolution, CapabilitySource,
    CentralCapabilityPrecedenceResolver, RuntimeCapabilityMode, StringCapabilityCandidate,
    ValueCapabilityCandidate, detect_runtime_capability_mode,
};
#[allow(unused_imports)]
pub use logs::{cleanup_old_logs, cleanup_old_logs_at};
#[allow(unused_imports)]
pub use looking_glass_resolver::{
    CentralLookingGlassCapabilityResolver, LookingGlassCapabilityResolver, LookingGlassLaunchMode,
    LookingGlassProgramResolution, resolve_looking_glass_program,
    resolve_looking_glass_program_with_source,
};
#[allow(unused_imports)]
pub use network_resolver::{
    CentralNetworkCapabilityResolver, NetworkCapabilityResolver, NetworkResolutionMode,
    ResolvedNetworkOutcome, resolve_network_outcome, resolve_networks_for_vm,
};
#[allow(unused_imports)]
pub use paths::{
    create_session_log_file, get_config_cache, get_log_file, get_log_file_at, get_logs_dir,
    get_logs_dir_at, get_pid_file, get_pid_file_at, get_shutdown_marker_file_at, get_state_dir,
};
#[allow(unused_imports)]
pub use pid::{delete_pid, delete_pid_at, read_pid, read_pid_at, save_pid, save_pid_at};
#[allow(unused_imports)]
pub use runtime_resolver::{
    CentralRuntimeCapabilityResolver, RuntimeCapabilityResolver,
    resolve_runtime_guest_agent_socket, resolve_runtime_root, resolve_runtime_root_with_source,
    resolve_runtime_tpm_socket,
};
#[allow(unused_imports)]
pub use shutdown::{delete_shutdown_marker, save_shutdown_marker, shutdown_marker_exists};
#[allow(unused_imports)]
pub use tpm_resolver::{
    CentralTpmCapabilityResolver, TpmCapabilityResolver, TpmPlacementMode, resolve_swtpm_binary,
    resolve_swtpm_binary_with_source, resolve_tpm_placement_mode, resolve_tpm_socket_path,
    resolve_tpm_state_dir,
};
#[allow(unused_imports)]
pub use vm_state::{VmState, VmStateEvent};

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
    fn test_shutdown_marker_path_follows_pid_directory() {
        let custom_dir = std::env::temp_dir().join("ezkvm-shutdown-marker-tests");
        let custom_pid = custom_dir.join("vm.pid");
        let marker =
            get_shutdown_marker_file_at("custom-vm", Some(&custom_pid.to_string_lossy())).unwrap();

        assert_eq!(marker, custom_dir.join("custom-vm.shutdown"));
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
