use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmPlacementMode {
    Socket,
    StateFile,
}

pub trait TpmCapabilityResolver {
    fn resolve_swtpm_binary(&self) -> Option<String>;
    fn resolve_placement_mode(&self) -> TpmPlacementMode;
    fn resolve_socket_path(&self, vm_name: &str, vm_state_path: Option<&str>) -> Result<String>;
    fn resolve_state_dir(&self, vm_state_dir: Option<&str>) -> Result<PathBuf>;
}

pub struct CentralTpmCapabilityResolver<'a> {
    central_config: &'a crate::config::CentralConfig,
    runtime_overrides: &'a crate::config::RuntimeCliOverrides,
}

impl<'a> CentralTpmCapabilityResolver<'a> {
    pub fn new(
        central_config: &'a crate::config::CentralConfig,
        runtime_overrides: &'a crate::config::RuntimeCliOverrides,
    ) -> Self {
        Self {
            central_config,
            runtime_overrides,
        }
    }

    fn non_empty(value: Option<&str>) -> Option<&str> {
        value.and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    fn find_in_path(program: &str) -> Option<String> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(program);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        None
    }

    fn distro_fallback_swtpm() -> Option<String> {
        for candidate in [
            "/usr/bin/swtpm",
            "/usr/sbin/swtpm",
            "/bin/swtpm",
            "/sbin/swtpm",
        ] {
            if Path::new(candidate).is_file() {
                return Some(candidate.to_string());
            }
        }
        None
    }
}

impl TpmCapabilityResolver for CentralTpmCapabilityResolver<'_> {
    fn resolve_swtpm_binary(&self) -> Option<String> {
        if let Some(program) = Self::non_empty(self.runtime_overrides.swtpm_binary.as_deref()) {
            return Some(program.to_string());
        }

        if let Some(program) = self.central_config.swtpm_program() {
            return Some(program.to_string());
        }

        Self::find_in_path("swtpm").or_else(Self::distro_fallback_swtpm)
    }

    fn resolve_placement_mode(&self) -> TpmPlacementMode {
        match self
            .central_config
            .tpm_placement_mode()
            .map(str::trim)
            .unwrap_or("socket")
        {
            "state-file" => TpmPlacementMode::StateFile,
            _ => TpmPlacementMode::Socket,
        }
    }

    fn resolve_socket_path(&self, vm_name: &str, vm_state_path: Option<&str>) -> Result<String> {
        if let Some(path) = Self::non_empty(self.runtime_overrides.tpm_socket_path.as_deref()) {
            return Ok(path.to_string());
        }

        if let Some(path) = Self::non_empty(vm_state_path) {
            return Ok(path.to_string());
        }

        if let Some(dir) = self.central_config.tpm_socket_dir() {
            let vm = vm_name.trim();
            if vm.is_empty() {
                return Err(anyhow!(
                    "VM name must not be empty when resolving TPM socket path"
                ));
            }
            return Ok(PathBuf::from(dir)
                .join(format!("{}.swtpm", vm))
                .display()
                .to_string());
        }

        crate::state::resolve_runtime_tpm_socket(
            vm_name,
            self.central_config,
            self.runtime_overrides,
        )
    }

    fn resolve_state_dir(&self, vm_state_dir: Option<&str>) -> Result<PathBuf> {
        if let Some(dir) = Self::non_empty(vm_state_dir) {
            return Ok(PathBuf::from(dir));
        }

        if let Some(dir) = self.central_config.tpm_state_dir() {
            return Ok(PathBuf::from(dir));
        }

        if let Some(xdg_state_home) =
            Self::non_empty(std::env::var("XDG_STATE_HOME").ok().as_deref())
        {
            return Ok(PathBuf::from(xdg_state_home).join("ezkvm/tpm"));
        }

        if let Some(home) = Self::non_empty(std::env::var("HOME").ok().as_deref()) {
            return Ok(PathBuf::from(home).join(".local/state/ezkvm/tpm"));
        }

        Ok(PathBuf::from("/tmp/ezkvm/tpm-state"))
    }
}

pub fn resolve_swtpm_binary(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Option<String> {
    CentralTpmCapabilityResolver::new(central_config, runtime_overrides).resolve_swtpm_binary()
}

pub fn resolve_tpm_placement_mode(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> TpmPlacementMode {
    CentralTpmCapabilityResolver::new(central_config, runtime_overrides).resolve_placement_mode()
}

pub fn resolve_tpm_socket_path(
    vm_name: &str,
    vm_state_path: Option<&str>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<String> {
    CentralTpmCapabilityResolver::new(central_config, runtime_overrides)
        .resolve_socket_path(vm_name, vm_state_path)
}

pub fn resolve_tpm_state_dir(
    vm_state_dir: Option<&str>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<PathBuf> {
    CentralTpmCapabilityResolver::new(central_config, runtime_overrides)
        .resolve_state_dir(vm_state_dir)
}

#[cfg(test)]
mod tests {
    use super::{
        TpmPlacementMode, resolve_tpm_placement_mode, resolve_tpm_socket_path,
        resolve_tpm_state_dir,
    };
    use crate::config::{CentralConfig, RuntimeCliOverrides};

    #[test]
    fn placement_mode_defaults_to_socket() {
        let mode =
            resolve_tpm_placement_mode(&CentralConfig::default(), &RuntimeCliOverrides::default());
        assert_eq!(mode, TpmPlacementMode::Socket);
    }

    #[test]
    fn placement_mode_uses_state_file_when_configured() {
        let mut central = CentralConfig::default();
        central.host_capabilities.tpm.placement_mode = Some("state-file".to_string());

        let mode = resolve_tpm_placement_mode(&central, &RuntimeCliOverrides::default());
        assert_eq!(mode, TpmPlacementMode::StateFile);
    }

    #[test]
    fn socket_path_prefers_cli_override() {
        let central = CentralConfig::default();
        let overrides = RuntimeCliOverrides {
            tpm_socket_path: Some("/cli/socket.swtpm".to_string()),
            ..Default::default()
        };

        let path = resolve_tpm_socket_path("vm1", Some("/vm/socket.swtpm"), &central, &overrides)
            .expect("socket path should resolve");
        assert_eq!(path, "/cli/socket.swtpm");
    }

    #[test]
    fn state_dir_uses_xdg_state_home_when_available() {
        let _guard = crate::test_support::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::set_var("XDG_STATE_HOME", "/tmp/xdg-state");
        }

        let resolved = resolve_tpm_state_dir(
            None,
            &CentralConfig::default(),
            &RuntimeCliOverrides::default(),
        )
        .expect("state dir should resolve");
        assert_eq!(resolved.to_string_lossy(), "/tmp/xdg-state/ezkvm/tpm");
    }
}
