use anyhow::{Result, anyhow};

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

        let cli_program = self.runtime_overrides.looking_glass_program.as_deref();
        let vm_program = vm_options.and_then(|opts| opts.program.as_deref());
        let central_program = self.central_config.looking_glass_program();

        for candidate in [cli_program, vm_program, central_program] {
            let Some(path) = Self::non_empty(candidate) else {
                continue;
            };

            if Self::program_available(path) {
                return Ok(Some(path.to_string()));
            }

            if mode == LookingGlassLaunchMode::Explicit {
                return Err(anyhow!(
                    "Looking Glass explicit mode requested but program '{}' is not available",
                    path
                ));
            }
        }

        if let Some(found) = Self::find_in_path("looking-glass-client") {
            return Ok(Some(found));
        }

        if mode == LookingGlassLaunchMode::Explicit {
            return Err(anyhow!(
                "Looking Glass explicit mode requested but no looking-glass-client binary was found in configured paths or PATH"
            ));
        }

        Ok(None)
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
