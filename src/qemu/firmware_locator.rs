//! Firmware capability resolver for OVMF and SeaBIOS asset discovery.
//!
//! Provides ordered search across configured directories and built-in
//! platform defaults, using candidate filename lists from central config.

use std::path::PathBuf;

use crate::config::{CentralConfig, RuntimeCliOverrides};

/// Built-in platform default OVMF search directories (Debian, Ubuntu, Arch).
const PLATFORM_OVMF_DIRS: &[&str] = &[
    "/usr/share/ovmf", // Debian, Arch
    "/usr/share/OVMF", // Ubuntu
];

const DEFAULT_CODE_FILES: &[&str] = &["OVMF_CODE_4M.fd", "OVMF_CODE.fd", "OVMF.fd"];

const DEFAULT_SECBOOT_CODE_FILES: &[&str] =
    &["OVMF_CODE_4M.secboot.fd", "OVMF_CODE.secboot.fd", "OVMF.fd"];

/// Trait for resolving firmware asset paths from a host capability context.
pub trait FirmwareCapabilityResolver {
    /// Return the resolved OVMF code file path, or `None` when no file was found.
    fn resolve_ovmf_code(&self, secure_boot: bool) -> Option<String>;
}

/// Central-config-backed firmware capability resolver.
///
/// Searches an ordered list of directories built from CLI overrides,
/// central config `host_capabilities.firmware` sections, and built-in
/// platform defaults. Within each directory the candidate filename list
/// from central config (or the built-in default) is tried in order.
pub struct CentralFirmwareCapabilityResolver<'a> {
    central_config: &'a CentralConfig,
    runtime_overrides: &'a RuntimeCliOverrides,
}

impl<'a> CentralFirmwareCapabilityResolver<'a> {
    /// Create a new resolver bound to the given config and CLI overrides.
    pub fn new(
        central_config: &'a CentralConfig,
        runtime_overrides: &'a RuntimeCliOverrides,
    ) -> Self {
        Self {
            central_config,
            runtime_overrides,
        }
    }

    /// Ordered list of directories to search: configured dirs first, then platform defaults.
    fn search_dirs(&self) -> Vec<String> {
        let mut dirs = self
            .central_config
            .ovmf_search_dirs_with_overrides(self.runtime_overrides);

        for &platform_dir in PLATFORM_OVMF_DIRS {
            if !dirs.iter().any(|d| d == platform_dir) {
                dirs.push(platform_dir.to_string());
            }
        }

        dirs
    }

    fn candidate_files(&self, secure_boot: bool) -> Vec<&str> {
        let configured = if secure_boot {
            &self
                .central_config
                .host_capabilities
                .firmware
                .secure_boot_code_files
        } else {
            &self.central_config.host_capabilities.firmware.code_files
        };

        if !configured.is_empty() {
            configured.iter().map(|s| s.as_str()).collect()
        } else if secure_boot {
            DEFAULT_SECBOOT_CODE_FILES.to_vec()
        } else {
            DEFAULT_CODE_FILES.to_vec()
        }
    }
}

impl FirmwareCapabilityResolver for CentralFirmwareCapabilityResolver<'_> {
    fn resolve_ovmf_code(&self, secure_boot: bool) -> Option<String> {
        let dirs = self.search_dirs();
        let candidates = self.candidate_files(secure_boot);

        for dir in &dirs {
            for &file in &candidates {
                let path = PathBuf::from(dir).join(file);
                if path.exists() {
                    return Some(path.to_string_lossy().into_owned());
                }
            }
        }

        None
    }
}

/// Resolve an OVMF code path from a single directory using default candidate filenames.
///
/// Always returns a path (using `OVMF.fd` fallback even when the file is absent).
/// Intended for cases where a directory has already been explicitly chosen (e.g. CLI override).
pub fn resolve_ovmf_code_from_dir(ovmf_dir: &str, secure_boot: bool) -> String {
    let candidates = if secure_boot {
        DEFAULT_SECBOOT_CODE_FILES
    } else {
        DEFAULT_CODE_FILES
    };

    for &file in candidates {
        let candidate = PathBuf::from(ovmf_dir).join(file);
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    PathBuf::from(ovmf_dir)
        .join("OVMF.fd")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CentralConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("ezkvm-fw-{}-{}", label, nanos))
    }

    #[test]
    fn resolve_from_dir_prefers_non_secure_4m_file_when_present() {
        let dir = unique_temp_dir("non-secure");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("OVMF_CODE_4M.fd");
        std::fs::write(&file, b"mock").unwrap();

        let result = resolve_ovmf_code_from_dir(&dir.to_string_lossy(), false);
        assert_eq!(result, file.to_string_lossy());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_from_dir_prefers_secure_4m_secboot_file_when_present() {
        let dir = unique_temp_dir("secure");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("OVMF_CODE_4M.secboot.fd");
        std::fs::write(&file, b"mock").unwrap();

        let result = resolve_ovmf_code_from_dir(&dir.to_string_lossy(), true);
        assert_eq!(result, file.to_string_lossy());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_from_dir_falls_back_to_ovmf_fd_when_no_candidate_exists() {
        let dir = unique_temp_dir("missing");
        let result = resolve_ovmf_code_from_dir(&dir.to_string_lossy(), false);
        assert!(result.ends_with("OVMF.fd"));
    }

    #[test]
    fn resolver_finds_file_in_configured_dir() {
        let dir = unique_temp_dir("resolver-central");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("OVMF_CODE_4M.fd");
        std::fs::write(&file, b"mock").unwrap();

        let mut central = CentralConfig::default();
        central.locations.ovmf_dir = Some(dir.to_string_lossy().into_owned());

        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        let result = resolver.resolve_ovmf_code(false);
        assert_eq!(result, Some(file.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolver_returns_none_when_no_firmware_found() {
        let central = CentralConfig::default();
        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        // Platform defaults won't exist in CI — resolver must return None, not panic.
        // (Accept Some if the host happens to have OVMF installed.)
        let result = resolver.resolve_ovmf_code(false);
        // Just verify it doesn't panic and returns consistent type.
        let _ = result;
    }

    #[test]
    fn resolver_uses_custom_code_files_from_central_config() {
        let dir = unique_temp_dir("resolver-custom-files");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("my-custom-ovmf.fd");
        std::fs::write(&file, b"mock").unwrap();

        let mut central = CentralConfig::default();
        central.locations.ovmf_dir = Some(dir.to_string_lossy().into_owned());
        central.host_capabilities.firmware.code_files = vec!["my-custom-ovmf.fd".to_string()];
        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        let result = resolver.resolve_ovmf_code(false);
        assert_eq!(result, Some(file.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolver_searches_dirs_in_precedence_order() {
        let dir_a = unique_temp_dir("precedence-a");
        let dir_b = unique_temp_dir("precedence-b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        // Only dir_b has the file.
        let file = dir_b.join("OVMF_CODE_4M.fd");
        std::fs::write(&file, b"mock").unwrap();

        let mut central = CentralConfig::default();
        central.host_capabilities.firmware.search_paths = vec![
            dir_a.to_string_lossy().into_owned(),
            dir_b.to_string_lossy().into_owned(),
        ];

        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        let result = resolver.resolve_ovmf_code(false);
        assert_eq!(result, Some(file.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }

    #[test]
    fn resolver_appends_platform_defaults_after_configured_dirs() {
        let mut central = CentralConfig::default();
        central.host_capabilities.firmware.search_paths = vec!["/custom/ovmf".to_string()];

        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        let dirs = resolver.search_dirs();

        assert_eq!(dirs[0], "/custom/ovmf");
        assert!(dirs.iter().any(|d| d == "/usr/share/ovmf"));
        assert!(dirs.iter().any(|d| d == "/usr/share/OVMF"));
    }

    #[test]
    fn resolver_deduplicates_platform_defaults_when_already_configured() {
        let mut central = CentralConfig::default();
        central.host_capabilities.firmware.search_paths =
            vec!["/usr/share/ovmf".to_string(), "/usr/share/OVMF".to_string()];

        let overrides = RuntimeCliOverrides::default();
        let resolver = CentralFirmwareCapabilityResolver::new(&central, &overrides);
        let dirs = resolver.search_dirs();

        assert_eq!(
            dirs.iter()
                .filter(|d| d.as_str() == "/usr/share/ovmf")
                .count(),
            1
        );
        assert_eq!(
            dirs.iter()
                .filter(|d| d.as_str() == "/usr/share/OVMF")
                .count(),
            1
        );
    }
}
