use anyhow::{Result, anyhow};
use std::path::PathBuf;

/// Resolves host runtime directory paths using the portable-runtime precedence contract.
pub trait RuntimeCapabilityResolver {
    /// Resolve runtime root with precedence:
    /// explicit > CLI/central host capabilities > XDG runtime > HOME fallback > built-in fallback.
    fn resolve_runtime_root(&self, explicit: Option<&str>) -> Result<PathBuf>;
}

/// Central-config-backed runtime capability resolver.
pub struct CentralRuntimeCapabilityResolver<'a> {
    central_config: &'a crate::config::CentralConfig,
    runtime_overrides: &'a crate::config::RuntimeCliOverrides,
}

impl<'a> CentralRuntimeCapabilityResolver<'a> {
    pub fn new(
        central_config: &'a crate::config::CentralConfig,
        runtime_overrides: &'a crate::config::RuntimeCliOverrides,
    ) -> Self {
        Self {
            central_config,
            runtime_overrides,
        }
    }

    fn non_empty(candidate: Option<&str>) -> Option<&str> {
        candidate.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }
}

impl RuntimeCapabilityResolver for CentralRuntimeCapabilityResolver<'_> {
    fn resolve_runtime_root(&self, explicit: Option<&str>) -> Result<PathBuf> {
        if let Some(explicit_path) = Self::non_empty(explicit) {
            return Ok(PathBuf::from(explicit_path));
        }

        if let Some(configured) = self
            .central_config
            .runtime_run_dir_with_overrides(self.runtime_overrides)
        {
            return Ok(PathBuf::from(configured));
        }

        if let Some(xdg_runtime) = Self::non_empty(std::env::var("XDG_RUNTIME_DIR").ok().as_deref())
        {
            return Ok(PathBuf::from(xdg_runtime).join("ezkvm"));
        }

        if let Some(home) = Self::non_empty(std::env::var("HOME").ok().as_deref()) {
            return Ok(PathBuf::from(home).join(".local/run/ezkvm"));
        }

        // Final built-in fallback for constrained environments where HOME is unavailable.
        Ok(PathBuf::from("/tmp/ezkvm"))
    }
}

/// Convenience helper for resolving runtime root paths with central defaults.
pub fn resolve_runtime_root(
    explicit: Option<&str>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<PathBuf> {
    CentralRuntimeCapabilityResolver::new(central_config, runtime_overrides)
        .resolve_runtime_root(explicit)
}

/// Resolve default TPM socket path under runtime root.
pub fn resolve_runtime_tpm_socket(
    vm_name: &str,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<String> {
    let root = resolve_runtime_root(None, central_config, runtime_overrides)?;
    let vm = vm_name.trim();
    if vm.is_empty() {
        return Err(anyhow!(
            "VM name must not be empty when resolving TPM socket path"
        ));
    }
    Ok(root.join(format!("{}.swtpm", vm)).display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{resolve_runtime_root, resolve_runtime_tpm_socket};
    use crate::config::{CentralConfig, RuntimeCliOverrides};

    #[test]
    fn runtime_root_precedence_prefers_explicit_over_all_other_sources() {
        let central = CentralConfig::default();
        let overrides = RuntimeCliOverrides {
            run_dir: Some("/cli/run".to_string()),
            ..Default::default()
        };

        let resolved = resolve_runtime_root(Some("/explicit/run"), &central, &overrides)
            .expect("runtime root should resolve");
        assert_eq!(resolved.to_string_lossy(), "/explicit/run");
    }

    #[test]
    fn runtime_root_precedence_prefers_cli_over_xdg() {
        let _guard = crate::test_support::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", "/xdg/runtime");
        }

        let central = CentralConfig::default();
        let overrides = RuntimeCliOverrides {
            run_dir: Some("/cli/runtime".to_string()),
            ..Default::default()
        };

        let resolved =
            resolve_runtime_root(None, &central, &overrides).expect("runtime root should resolve");
        assert_eq!(resolved.to_string_lossy(), "/cli/runtime");
    }

    #[test]
    fn runtime_root_precedence_prefers_xdg_over_home_fallback() {
        let _guard = crate::test_support::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", "/xdg/runtime");
            std::env::set_var("HOME", "/home/tester");
        }

        let resolved = resolve_runtime_root(
            None,
            &CentralConfig::default(),
            &RuntimeCliOverrides::default(),
        )
        .expect("runtime root should resolve");
        assert_eq!(resolved.to_string_lossy(), "/xdg/runtime/ezkvm");
    }

    #[test]
    fn runtime_tpm_socket_uses_runtime_root_and_vm_name() {
        let central = CentralConfig::default();
        let overrides = RuntimeCliOverrides {
            run_dir: Some("/run/ezkvm".to_string()),
            ..Default::default()
        };

        let socket = resolve_runtime_tpm_socket("vm-a", &central, &overrides)
            .expect("socket path should resolve");
        assert_eq!(socket, "/run/ezkvm/vm-a.swtpm");
    }
}
