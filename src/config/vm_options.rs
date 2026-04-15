use serde::{Deserialize, Serialize};

use crate::config::{GuestAgentConfig, LookingGlassOptions, QmpConfig};

fn default_nodefaults() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Additional VM options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmOptions {
    /// Enable KVM acceleration
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enable_kvm: bool,

    /// Run in daemon mode
    #[serde(default, skip_serializing_if = "is_false")]
    pub daemonize: bool,

    /// Disable QEMU default devices
    #[serde(default = "default_nodefaults", skip_serializing_if = "is_true")]
    pub nodefaults: bool,

    /// Raw `-global` options
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub global_options: Vec<String>,

    /// RTC configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtc: Option<RtcConfig>,

    /// Custom PID file location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid_file: Option<String>,

    /// Custom log directory for VM-specific logs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_dir: Option<String>,

    /// Number of log files to retain during rotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_keep: Option<usize>,

    /// Path to UEFI variables file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_vars: Option<String>,

    /// Canonical guest agent configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_agent: Option<GuestAgentConfig>,

    /// Canonical QMP monitoring configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qmp: Option<QmpConfig>,

    /// Looking Glass client launch options (VM/profile-level override).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub looking_glass: Option<LookingGlassOptions>,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            enable_kvm: true,
            daemonize: false,
            nodefaults: true,
            global_options: Vec::new(),
            rtc: None,
            pid_file: None,
            log_dir: None,
            log_keep: None,
            uefi_vars: None,
            guest_agent: None,
            qmp: None,
            looking_glass: None,
        }
    }
}

/// RTC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcConfig {
    /// RTC base, usually `utc` or `localtime`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,

    /// RTC drift fix policy, usually `slew` or `none`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driftfix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::VmOptions;

    #[test]
    fn vm_options_omitted_fields_deserialize_to_defaults() {
        let options: VmOptions = serde_yaml::from_str("{}").expect("deserialize vm options");
        assert!(options.enable_kvm);
        assert!(!options.daemonize);
        assert!(options.nodefaults);
    }
}
