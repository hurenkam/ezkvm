use serde::{Deserialize, Serialize};

use crate::config::{GuestAgentConfig, LookingGlassOptions, QmpConfig};

/// Additional VM options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmOptions {
    /// Enable KVM acceleration
    pub enable_kvm: bool,

    /// Run in daemon mode
    pub daemonize: bool,

    /// Disable QEMU default devices
    #[serde(default)]
    pub nodefaults: bool,

    /// Raw `-global` options
    #[serde(default)]
    pub global_options: Vec<String>,

    /// RTC configuration
    #[serde(default)]
    pub rtc: Option<RtcConfig>,

    /// Custom PID file location
    pub pid_file: Option<String>,

    /// Custom log directory for VM-specific logs
    pub log_dir: Option<String>,

    /// Number of log files to retain during rotation
    pub log_keep: Option<usize>,

    /// Path to UEFI variables file
    pub uefi_vars: Option<String>,

    /// Canonical guest agent configuration.
    #[serde(default)]
    pub guest_agent: Option<GuestAgentConfig>,

    /// Canonical QMP monitoring configuration.
    #[serde(default)]
    pub qmp: Option<QmpConfig>,

    /// Looking Glass client launch options (VM/profile-level override).
    #[serde(default)]
    pub looking_glass: Option<LookingGlassOptions>,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            enable_kvm: true,
            daemonize: false,
            nodefaults: false,
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
    pub base: Option<String>,

    /// RTC drift fix policy, usually `slew` or `none`
    pub driftfix: Option<String>,
}
