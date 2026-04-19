use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilitySource {
    CliOverride,
    VmOverride,
    #[allow(dead_code)]
    ProfileDefault,
    CentralConfig,
    PathLookup,
    PlatformDefault,
    BuiltInFallback,
    ParityBypass,
}

impl CapabilitySource {
    pub fn label(self) -> &'static str {
        match self {
            Self::CliOverride => "cli-override",
            Self::VmOverride => "vm-override",
            Self::ProfileDefault => "profile-default",
            Self::CentralConfig => "central-config",
            Self::PathLookup => "path-lookup",
            Self::PlatformDefault => "platform-default",
            Self::BuiltInFallback => "built-in-fallback",
            Self::ParityBypass => "parity-bypass",
        }
    }
}

impl fmt::Display for CapabilitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityResolution<T> {
    pub value: Option<T>,
    pub source: Option<CapabilitySource>,
}

impl<T> CapabilityResolution<T> {
    pub fn none() -> Self {
        Self {
            value: None,
            source: None,
        }
    }

    pub fn with_value(value: T, source: CapabilitySource) -> Self {
        Self {
            value: Some(value),
            source: Some(source),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StringCapabilityCandidate<'a> {
    pub source: CapabilitySource,
    pub value: Option<&'a str>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValueCapabilityCandidate<T> {
    pub source: CapabilitySource,
    pub value: Option<T>,
}

pub trait CapabilityPrecedenceResolver {
    fn resolve_non_empty_string(
        &self,
        candidates: &[StringCapabilityCandidate<'_>],
    ) -> CapabilityResolution<String>;

    #[allow(dead_code)]
    fn resolve_first<T: Clone>(
        &self,
        candidates: &[ValueCapabilityCandidate<T>],
    ) -> CapabilityResolution<T>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CentralCapabilityPrecedenceResolver;

impl CentralCapabilityPrecedenceResolver {
    fn normalize_non_empty(candidate: Option<&str>) -> Option<String> {
        candidate.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }
}

impl CapabilityPrecedenceResolver for CentralCapabilityPrecedenceResolver {
    fn resolve_non_empty_string(
        &self,
        candidates: &[StringCapabilityCandidate<'_>],
    ) -> CapabilityResolution<String> {
        for candidate in candidates {
            if let Some(value) = Self::normalize_non_empty(candidate.value) {
                return CapabilityResolution::with_value(value, candidate.source);
            }
        }

        CapabilityResolution::none()
    }

    fn resolve_first<T: Clone>(
        &self,
        candidates: &[ValueCapabilityCandidate<T>],
    ) -> CapabilityResolution<T> {
        for candidate in candidates {
            if let Some(value) = candidate.value.clone() {
                return CapabilityResolution::with_value(value, candidate.source);
            }
        }

        CapabilityResolution::none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCapabilityMode {
    PortableLinux,
    ProxmoxParity,
}

pub fn detect_runtime_capability_mode(config: &crate::config::VmConfig) -> RuntimeCapabilityMode {
    if config
        .profiles
        .iter()
        .any(|profile| profile == "proxmox-parity-runtime")
    {
        RuntimeCapabilityMode::ProxmoxParity
    } else {
        RuntimeCapabilityMode::PortableLinux
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityPrecedenceResolver, CapabilitySource, CentralCapabilityPrecedenceResolver,
        StringCapabilityCandidate, ValueCapabilityCandidate,
    };

    #[test]
    fn resolve_non_empty_string_prefers_cli_vm_profile_central_platform_order() {
        let resolver = CentralCapabilityPrecedenceResolver;
        let resolution = resolver.resolve_non_empty_string(&[
            StringCapabilityCandidate {
                source: CapabilitySource::CliOverride,
                value: Some(" "),
            },
            StringCapabilityCandidate {
                source: CapabilitySource::VmOverride,
                value: None,
            },
            StringCapabilityCandidate {
                source: CapabilitySource::ProfileDefault,
                value: Some("/profile/run"),
            },
            StringCapabilityCandidate {
                source: CapabilitySource::CentralConfig,
                value: Some("/central/run"),
            },
            StringCapabilityCandidate {
                source: CapabilitySource::PlatformDefault,
                value: Some("/platform/run"),
            },
        ]);

        assert_eq!(resolution.value.as_deref(), Some("/profile/run"));
        assert_eq!(resolution.source, Some(CapabilitySource::ProfileDefault));
    }

    #[test]
    fn resolve_first_uses_first_available_value() {
        let resolver = CentralCapabilityPrecedenceResolver;
        let resolution = resolver.resolve_first(&[
            ValueCapabilityCandidate {
                source: CapabilitySource::CliOverride,
                value: None,
            },
            ValueCapabilityCandidate {
                source: CapabilitySource::CentralConfig,
                value: Some(42u32),
            },
            ValueCapabilityCandidate {
                source: CapabilitySource::BuiltInFallback,
                value: Some(7u32),
            },
        ]);

        assert_eq!(resolution.value, Some(42));
        assert_eq!(resolution.source, Some(CapabilitySource::CentralConfig));
    }
}
