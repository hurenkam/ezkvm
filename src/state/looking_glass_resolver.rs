use anyhow::{Result, anyhow};

use super::{
    CapabilityPrecedenceResolver, CapabilityResolution, CapabilitySource,
    CentralCapabilityPrecedenceResolver, StringCapabilityCandidate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookingGlassLaunchMode {
    Disabled,
    Auto,
    Explicit,
}

pub trait LookingGlassCapabilityResolver {
    fn resolve_program(
        &self,
        vm_options: Option<&crate::config::LookingGlassOptions>,
    ) -> Result<Option<String>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookingGlassProgramResolution {
    pub mode: LookingGlassLaunchMode,
    pub resolution: CapabilityResolution<String>,
}

pub struct CentralLookingGlassCapabilityResolver<'a> {
    central_config: &'a crate::config::CentralConfig,
    runtime_overrides: &'a crate::config::RuntimeCliOverrides,
}

impl<'a> CentralLookingGlassCapabilityResolver<'a> {
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
        value.and_then(|candidate| {
            let trimmed = candidate.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
    }

    fn parse_mode(value: Option<&str>) -> Option<LookingGlassLaunchMode> {
        match value.map(str::trim) {
            Some("disabled") => Some(LookingGlassLaunchMode::Disabled),
            Some("auto") => Some(LookingGlassLaunchMode::Auto),
            Some("explicit") | Some("enabled") => Some(LookingGlassLaunchMode::Explicit),
            _ => None,
        }
    }

    fn mode_for(
        &self,
        vm_options: Option<&crate::config::LookingGlassOptions>,
    ) -> LookingGlassLaunchMode {
        if Self::non_empty(self.runtime_overrides.looking_glass_program.as_deref()).is_some() {
            return LookingGlassLaunchMode::Explicit;
        }

        if let Some(mode) = Self::parse_mode(vm_options.and_then(|opts| opts.mode.as_deref())) {
            return mode;
        }

        if Self::non_empty(vm_options.and_then(|opts| opts.program.as_deref())).is_some()
            || Self::non_empty(self.central_config.looking_glass_program()).is_some()
        {
            return LookingGlassLaunchMode::Auto;
        }

        LookingGlassLaunchMode::Disabled
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

    fn program_available(program: &str) -> bool {
        let trimmed = program.trim();
        if trimmed.is_empty() {
            return false;
        }

        if trimmed.contains('/') {
            return std::path::Path::new(trimmed).is_file();
        }

        Self::find_in_path(trimmed).is_some()
    }
}

impl LookingGlassCapabilityResolver for CentralLookingGlassCapabilityResolver<'_> {
    fn resolve_program(
        &self,
        vm_options: Option<&crate::config::LookingGlassOptions>,
    ) -> Result<Option<String>> {
        let mode = self.mode_for(vm_options);
        if mode == LookingGlassLaunchMode::Disabled {
            return Ok(None);
        }

        if mode == LookingGlassLaunchMode::Explicit {
            if let Some(path) = self.runtime_overrides.looking_glass_program.as_deref()
                && path.trim().is_empty()
            {
                return Err(anyhow!(
                    "Looking Glass explicit mode requested but CLI program path is empty"
                ));
            }

            if let Some(path) = vm_options.and_then(|opts| opts.program.as_deref())
                && path.trim().is_empty()
            {
                return Err(anyhow!(
                    "Looking Glass explicit mode requested but program path is empty"
                ));
            }
        }

        let resolved = resolve_looking_glass_program_with_source(
            vm_options,
            self.central_config,
            self.runtime_overrides,
        )?;
        Ok(resolved.resolution.value)
    }
}

pub fn resolve_looking_glass_program(
    vm_options: Option<&crate::config::LookingGlassOptions>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<Option<String>> {
    CentralLookingGlassCapabilityResolver::new(central_config, runtime_overrides)
        .resolve_program(vm_options)
}

pub fn resolve_looking_glass_program_with_source(
    vm_options: Option<&crate::config::LookingGlassOptions>,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<LookingGlassProgramResolution> {
    let resolver = CentralLookingGlassCapabilityResolver::new(central_config, runtime_overrides);
    let mode = resolver.mode_for(vm_options);
    if mode == LookingGlassLaunchMode::Disabled {
        return Ok(LookingGlassProgramResolution {
            mode,
            resolution: CapabilityResolution::none(),
        });
    }

    let precedence = CentralCapabilityPrecedenceResolver;
    let mut resolution = precedence.resolve_non_empty_string(&[
        StringCapabilityCandidate {
            source: CapabilitySource::CliOverride,
            value: runtime_overrides.looking_glass_program.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::VmOverride,
            value: vm_options.and_then(|opts| opts.program.as_deref()),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::CentralConfig,
            value: central_config
                .host_capabilities
                .integrations
                .looking_glass
                .program
                .as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::CentralConfig,
            value: central_config.looking_glass.program.as_deref(),
        },
        StringCapabilityCandidate {
            source: CapabilitySource::CentralConfig,
            value: central_config.tools.looking_glass.as_deref(),
        },
    ]);

    if let Some(path) = resolution.value.as_deref() {
        if CentralLookingGlassCapabilityResolver::program_available(path) {
            return Ok(LookingGlassProgramResolution { mode, resolution });
        }

        if mode == LookingGlassLaunchMode::Explicit {
            return Err(anyhow!(
                "Looking Glass explicit mode requested but program '{}' is not available",
                path
            ));
        }

        resolution = CapabilityResolution::none();
    }

    if let Some(found) = CentralLookingGlassCapabilityResolver::find_in_path("looking-glass-client")
    {
        return Ok(LookingGlassProgramResolution {
            mode,
            resolution: CapabilityResolution::with_value(found, CapabilitySource::PathLookup),
        });
    }

    if mode == LookingGlassLaunchMode::Explicit {
        return Err(anyhow!(
            "Looking Glass explicit mode requested but no looking-glass-client binary was found in configured paths or PATH"
        ));
    }

    Ok(LookingGlassProgramResolution { mode, resolution })
}

#[cfg(test)]
mod tests {
    use super::resolve_looking_glass_program;

    #[test]
    fn disabled_mode_returns_none() {
        let vm_options = crate::config::LookingGlassOptions {
            mode: Some("disabled".to_string()),
            ..Default::default()
        };

        let result = resolve_looking_glass_program(
            Some(&vm_options),
            &crate::config::CentralConfig::default(),
            &crate::config::RuntimeCliOverrides::default(),
        )
        .expect("resolution should succeed");

        assert!(result.is_none());
    }

    #[test]
    fn explicit_mode_errors_when_binary_missing() {
        let vm_options = crate::config::LookingGlassOptions {
            mode: Some("explicit".to_string()),
            program: Some("/definitely/missing/lg".to_string()),
            ..Default::default()
        };

        let err = resolve_looking_glass_program(
            Some(&vm_options),
            &crate::config::CentralConfig::default(),
            &crate::config::RuntimeCliOverrides::default(),
        )
        .expect_err("explicit missing binary should fail");

        assert!(err.to_string().contains("explicit mode"));
    }

    #[test]
    fn auto_mode_silently_disables_when_binary_missing() {
        let _guard = crate::test_support::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            std::env::set_var("PATH", "/tmp/definitely-missing-lg");
        }

        let mut central = crate::config::CentralConfig::default();
        central.host_capabilities.integrations.looking_glass.program =
            Some("/definitely/missing/lg".to_string());

        let result = resolve_looking_glass_program(
            None,
            &central,
            &crate::config::RuntimeCliOverrides::default(),
        )
        .expect("resolution should succeed");

        assert!(result.is_none());
    }
}
