use serde::{Deserialize, Serialize};

/// Runtime CLI overrides for host-capability resolution.
#[derive(Debug, Clone, Default)]
pub struct RuntimeCliOverrides {
    pub run_dir: Option<String>,
    pub tpm_socket_path: Option<String>,
    pub swtpm_binary: Option<String>,
    pub remote_viewer_program: Option<String>,
    pub looking_glass_program: Option<String>,
    pub ovmf_dir: Option<String>,
}

/// Central tool configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct CentralConfig {
    /// Tool paths
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Directory locations
    #[serde(default)]
    pub locations: LocationsConfig,

    /// Looking Glass client options
    #[serde(default)]
    pub looking_glass: LookingGlassOptions,

    /// Host-specific capability defaults and deployment policy.
    #[serde(default)]
    pub host_capabilities: HostCapabilitiesConfig,
}

impl CentralConfig {
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

    /// Resolve the effective runtime directory root, including CLI overrides.
    pub fn runtime_run_dir_with_overrides<'a>(
        &'a self,
        overrides: &'a RuntimeCliOverrides,
    ) -> Option<&'a str> {
        Self::non_empty(overrides.run_dir.as_deref()).or_else(|| {
            Self::non_empty(
                self.host_capabilities
                    .runtime
                    .run_dir
                    .as_deref()
                    .or(self.locations.run_dir.as_deref()),
            )
        })
    }

    /// Resolve the effective runtime directory root.
    #[allow(dead_code)]
    pub fn runtime_run_dir(&self) -> Option<&str> {
        Self::non_empty(
            self.host_capabilities
                .runtime
                .run_dir
                .as_deref()
                .or(self.locations.run_dir.as_deref()),
        )
    }

    /// Resolve the configured profile directory.
    pub fn profile_dir(&self) -> Option<&str> {
        self.locations.profile_dir.as_deref()
    }

    /// Resolve the effective OVMF firmware directory, including CLI overrides.
    pub fn ovmf_dir_with_overrides<'a>(
        &'a self,
        overrides: &'a RuntimeCliOverrides,
    ) -> Option<&'a str> {
        Self::non_empty(overrides.ovmf_dir.as_deref()).or_else(|| {
            Self::non_empty(
                self.host_capabilities
                    .firmware
                    .ovmf_dir
                    .as_deref()
                    .or(self.locations.ovmf_dir.as_deref()),
            )
        })
    }

    /// Resolve ordered OVMF search directories including fallback locations.
    pub fn ovmf_search_dirs_with_overrides(&self, overrides: &RuntimeCliOverrides) -> Vec<String> {
        let mut resolved = Vec::new();

        Self::push_non_empty_unique(&mut resolved, overrides.ovmf_dir.as_deref());
        Self::push_non_empty_unique(
            &mut resolved,
            self.host_capabilities.firmware.ovmf_dir.as_deref(),
        );
        Self::push_non_empty_unique(&mut resolved, self.locations.ovmf_dir.as_deref());

        for path in &self.host_capabilities.firmware.search_paths {
            Self::push_non_empty_unique(&mut resolved, Some(path.as_str()));
        }

        resolved
    }

    fn push_non_empty_unique(values: &mut Vec<String>, candidate: Option<&str>) {
        let Some(trimmed) = Self::non_empty(candidate) else {
            return;
        };

        if !values.iter().any(|existing| existing == trimmed) {
            values.push(trimmed.to_string());
        }
    }

    /// Resolve the effective OVMF firmware directory.
    #[allow(dead_code)]
    pub fn ovmf_dir(&self) -> Option<&str> {
        Self::non_empty(
            self.host_capabilities
                .firmware
                .ovmf_dir
                .as_deref()
                .or(self.locations.ovmf_dir.as_deref()),
        )
    }

    /// Resolve the effective swtpm binary path, including CLI overrides.
    pub fn swtpm_program_with_overrides<'a>(
        &'a self,
        overrides: &'a RuntimeCliOverrides,
    ) -> Option<&'a str> {
        Self::non_empty(overrides.swtpm_binary.as_deref()).or_else(|| {
            Self::non_empty(
                self.host_capabilities
                    .tpm
                    .swtpm_binary
                    .as_deref()
                    .or(self.tools.swtpm.as_deref()),
            )
        })
    }

    /// Resolve the effective swtpm binary path.
    #[allow(dead_code)]
    pub fn swtpm_program(&self) -> Option<&str> {
        Self::non_empty(
            self.host_capabilities
                .tpm
                .swtpm_binary
                .as_deref()
                .or(self.tools.swtpm.as_deref()),
        )
    }

    /// Resolve the effective remote-viewer program path, including CLI overrides.
    pub fn remote_viewer_program_with_overrides<'a>(
        &'a self,
        overrides: &'a RuntimeCliOverrides,
    ) -> Option<&'a str> {
        Self::non_empty(overrides.remote_viewer_program.as_deref()).or_else(|| {
            Self::non_empty(
                self.host_capabilities
                    .integrations
                    .remote_viewer
                    .program
                    .as_deref()
                    .or(self.tools.remote_viewer.as_deref()),
            )
        })
    }

    /// Resolve the effective remote-viewer program path.
    #[allow(dead_code)]
    pub fn remote_viewer_program(&self) -> Option<&str> {
        Self::non_empty(
            self.host_capabilities
                .integrations
                .remote_viewer
                .program
                .as_deref()
                .or(self.tools.remote_viewer.as_deref()),
        )
    }

    /// Resolve the effective Looking Glass program path, including CLI overrides.
    pub fn looking_glass_program_with_overrides<'a>(
        &'a self,
        overrides: &'a RuntimeCliOverrides,
    ) -> Option<&'a str> {
        Self::non_empty(overrides.looking_glass_program.as_deref()).or_else(|| {
            Self::non_empty(
                self.host_capabilities
                    .integrations
                    .looking_glass
                    .program
                    .as_deref()
                    .or(self.looking_glass.program.as_deref())
                    .or(self.tools.looking_glass.as_deref()),
            )
        })
    }

    /// Resolve the effective Looking Glass program path.
    #[allow(dead_code)]
    pub fn looking_glass_program(&self) -> Option<&str> {
        Self::non_empty(
            self.host_capabilities
                .integrations
                .looking_glass
                .program
                .as_deref()
                .or(self.looking_glass.program.as_deref())
                .or(self.tools.looking_glass.as_deref()),
        )
    }

    /// Resolve the configured network backend preference.
    #[allow(dead_code)]
    pub fn network_backend_preference(&self) -> Option<&str> {
        self.host_capabilities.network.preferred_backend.as_deref()
    }

    /// Resolve the configured bridge helper path.
    #[allow(dead_code)]
    pub fn bridge_helper(&self) -> Option<&str> {
        self.host_capabilities.network.bridge_helper.as_deref()
    }

    /// Resolve the configured default bridge name.
    #[allow(dead_code)]
    pub fn bridge_name(&self) -> Option<&str> {
        self.host_capabilities.network.bridge_name.as_deref()
    }

    /// Resolve the configured TPM state directory.
    #[allow(dead_code)]
    pub fn tpm_state_dir(&self) -> Option<&str> {
        self.host_capabilities.tpm.state_dir.as_deref()
    }

    /// Resolve the configured TPM socket directory.
    #[allow(dead_code)]
    pub fn tpm_socket_dir(&self) -> Option<&str> {
        self.host_capabilities.tpm.socket_dir.as_deref()
    }

    /// Resolve TPM placement policy.
    #[allow(dead_code)]
    pub fn tpm_placement_mode(&self) -> Option<&str> {
        self.host_capabilities.tpm.placement_mode.as_deref()
    }

    /// Resolve the configured Looking Glass shared-memory device.
    #[allow(dead_code)]
    pub fn looking_glass_shared_memory_device(&self) -> Option<&str> {
        self.host_capabilities
            .integrations
            .looking_glass
            .shared_memory_device
            .as_deref()
    }
}

/// Tool paths configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ToolsConfig {
    /// Path to qemu-system executable.
    pub qemu: Option<String>,

    /// Path to swtpm executable
    pub swtpm: Option<String>,

    /// Path to remote-viewer executable
    pub remote_viewer: Option<String>,

    /// Path to looking-glass-client executable
    pub looking_glass: Option<String>,
}

/// Directory locations configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LocationsConfig {
    /// Runtime directory for PID files, sockets, etc.
    pub run_dir: Option<String>,

    /// Runtime directory for PID files.
    pub pid_dir: Option<String>,

    /// Runtime directory for sockets.
    pub socket_dir: Option<String>,

    /// Runtime directory for logs.
    pub log_dir: Option<String>,

    /// Directory containing OVMF firmware files
    pub ovmf_dir: Option<String>,

    /// Default directory for VM configuration files
    #[serde(alias = "vms_dir")]
    pub vm_dir: Option<String>,

    /// Directory for VM templates
    pub template_dir: Option<String>,

    /// Directory containing reusable VM profile files
    pub profile_dir: Option<String>,
}

/// Looking Glass client options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LookingGlassOptions {
    /// Path to looking-glass-client executable (VM/profile-level preferred).
    pub program: Option<String>,

    /// Launch the client in fullscreen mode.
    pub full_screen: Option<bool>,

    /// Initial window size in WIDTHxHEIGHT format.
    pub size: Option<String>,

    /// Grab the keyboard when focused.
    pub grab_keyboard: Option<bool>,

    /// Escape key name used to release keyboard grab.
    pub escape_key: Option<String>,
}

/// Host-specific capability defaults and deployment policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct HostCapabilitiesConfig {
    /// Runtime directory layout and file-placement policy.
    pub runtime: RuntimeHostCapabilities,

    /// Firmware discovery policy.
    pub firmware: FirmwareHostCapabilities,

    /// Network backend and helper policy.
    pub network: NetworkHostCapabilities,

    /// TPM backend policy.
    pub tpm: TpmHostCapabilities,

    /// Optional host integration programs and devices.
    pub integrations: IntegrationHostCapabilities,
}

/// Runtime directory and file-placement defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeHostCapabilities {
    /// Root runtime directory used for sockets, pid files, and logs.
    ///
    /// Migration note: `root_directory` is accepted as an alias to ease
    /// rollout from the Phase-2 backlog wording.
    #[serde(alias = "root_directory")]
    pub run_dir: Option<String>,

    /// Directory used for pid files.
    pub pid_dir: Option<String>,

    /// Directory used for unix sockets.
    pub socket_dir: Option<String>,

    /// Directory used for log files.
    pub log_dir: Option<String>,
}

/// Firmware discovery defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct FirmwareHostCapabilities {
    /// Directory containing OVMF firmware files.
    pub ovmf_dir: Option<String>,

    /// Additional fallback directories used for OVMF discovery.
    pub search_paths: Vec<String>,

    /// Optional preferred OVMF file candidates for secure-boot mode.
    pub secure_boot_code_files: Vec<String>,

    /// Optional preferred OVMF file candidates for non-secure mode.
    pub code_files: Vec<String>,
}

/// Network backend and helper defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkHostCapabilities {
    /// Preferred host-side backend strategy for portable runtime.
    pub preferred_backend: Option<String>,

    /// Path to qemu-bridge-helper or equivalent helper.
    pub bridge_helper: Option<String>,

    /// Default bridge device name when bridge mode is used.
    pub bridge_name: Option<String>,
}

/// TPM backend defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct TpmHostCapabilities {
    /// Path to the swtpm executable.
    pub swtpm_binary: Option<String>,

    /// TPM state placement mode (`socket` or `state-file`).
    pub placement_mode: Option<String>,

    /// Default directory for TPM state.
    pub state_dir: Option<String>,

    /// Default directory for TPM control sockets.
    pub socket_dir: Option<String>,
}

/// Optional host integrations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationHostCapabilities {
    /// remote-viewer program policy.
    pub remote_viewer: ProgramCapability,

    /// Looking Glass program and shared-memory policy.
    pub looking_glass: LookingGlassCapability,
}

/// Generic external program capability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ProgramCapability {
    /// Program path.
    pub program: Option<String>,
}

/// Looking Glass host integration policy.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LookingGlassCapability {
    /// Program path.
    pub program: Option<String>,

    /// Shared-memory device path exposed by the host.
    pub shared_memory_device: Option<String>,
}
