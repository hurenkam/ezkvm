use crate::config::{CentralConfig, VmConfig};

/// Main QEMU manager
pub struct QemuManager {
    pub(super) config: VmConfig,
    pub(super) central_config: CentralConfig,
    pub(super) runtime_overrides: crate::config::RuntimeCliOverrides,
}

impl QemuManager {
    /// Create a new QEMU manager for a VM configuration
    #[allow(dead_code)]
    pub fn new(config: VmConfig, central_config: CentralConfig) -> Self {
        Self::new_with_overrides(
            config,
            central_config,
            crate::config::RuntimeCliOverrides::default(),
        )
    }

    /// Create a new QEMU manager with runtime CLI overrides.
    pub fn new_with_overrides(
        config: VmConfig,
        central_config: CentralConfig,
        runtime_overrides: crate::config::RuntimeCliOverrides,
    ) -> Self {
        Self {
            config,
            central_config,
            runtime_overrides,
        }
    }

    /// Get a reference to the VM configuration
    pub fn config(&self) -> &VmConfig {
        &self.config
    }

    /// Get the QEMU binary name
    pub fn binary_name(&self) -> String {
        self.config.system.qemu_binary()
    }

    pub(super) fn uses_external_swtpm(&self) -> bool {
        self.config
            .system_tpm()
            .map(|tpm| tpm.backend == "emulator")
            .unwrap_or(false)
            && self
                .central_config
                .swtpm_program_with_overrides(&self.runtime_overrides)
                .is_some()
    }

    pub(super) fn has_primary_passthrough_gpu(&self) -> bool {
        self.config
            .host_pci()
            .iter()
            .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
    }

    pub(super) fn resolve_tpm_socket_path(&self) -> String {
        if let Some(socket_path) = self
            .runtime_overrides
            .tpm_socket_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            return socket_path.to_string();
        }

        if let Some(tpm) = self.config.system_tpm()
            && let Some(state_path) = &tpm.state_path
        {
            return state_path.clone();
        }

        if let Some(run_dir) = self
            .central_config
            .runtime_run_dir_with_overrides(&self.runtime_overrides)
        {
            return format!("{}/{}.swtpm", run_dir, self.config.name);
        }

        format!("/var/run/qemu-server/{}.swtpm", self.config.name)
    }
}
