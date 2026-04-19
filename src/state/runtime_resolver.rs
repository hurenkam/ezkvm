use anyhow::{Result, anyhow};
use std::path::PathBuf;

use super::{
    CapabilityPrecedenceResolver, CapabilityResolution, CapabilitySource,
    CentralCapabilityPrecedenceResolver, StringCapabilityCandidate,
};

/// Resolves host runtime directory paths using the portable-runtime precedence contract.
#[allow(dead_code)]
pub trait RuntimeCapabilityResolver {
    /// Resolve runtime root with precedence:
    /// explicit > CLI/central host capabilities > XDG runtime > HOME fallback > built-in fallback.
    fn resolve_runtime_root(&self, explicit: Option<&str>) -> Result<PathBuf>;
}

/// Central-config-backed runtime capability resolver.
#[allow(dead_code)]
pub struct CentralRuntimeCapabilityResolver<'a> {
    central_config: &'a crate::config::CentralConfig,
    runtime_overrides: &'a crate::config::RuntimeCliOverrides,
}

impl<'a> CentralRuntimeCapabilityResolver<'a> {
    #[allow(dead_code)]
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
        Ok(
            resolve_runtime_root_with_source(explicit, self.central_config, self.runtime_overrides)
                .value
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/ezkvm")),
        )
    }
}

pub fn resolve_runtime_root_with_source(
    explicit: Option<&str>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> CapabilityResolution<String> {
    let resolver = CentralCapabilityPrecedenceResolver;
    let env_xdg = CentralRuntimeCapabilityResolver::non_empty(
        std::env::var("XDG_RUNTIME_DIR").ok().as_deref(),
    )
    .map(|path| format!("{}/ezkvm", path));
    let env_home =
        CentralRuntimeCapabilityResolver::non_empty(std::env::var("HOME").ok().as_deref())
            .map(|path| format!("{}/.local/run/ezkvm", path));

    let mut resolution = resolver.resolve_non_empty_string(&[
        StringCapabilityCandidate {
            source: CapabilitySource::CliOverride,
            value: runtime_overrides.run_dir.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::VmOverride,
            value: explicit,
        },
        StringCapabilityCandidate {
            source: CapabilitySource::CentralConfig,
            value: central_config.host_capabilities.runtime.run_dir.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::CentralConfig,
            value: central_config.locations.run_dir.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::PlatformDefault,
            value: env_xdg.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::PlatformDefault,
            value: env_home.as_deref(),
        },
    ]);

    if resolution.value.is_none() {
        resolution = CapabilityResolution::with_value(
            "/tmp/ezkvm".to_string(),
            CapabilitySource::BuiltInFallback,
        );
    }

    resolution
}

/// Convenience helper for resolving runtime root paths with central defaults.
pub fn resolve_runtime_root(
    explicit: Option<&str>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<PathBuf> {
    let resolved = resolve_runtime_root_with_source(explicit, central_config, runtime_overrides)
        .value
        .ok_or_else(|| anyhow!("failed to resolve runtime root"))?;
    Ok(PathBuf::from(resolved))
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
        let overrides = RuntimeCliOverrides::default();

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
