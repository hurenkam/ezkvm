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
        let explicit_state_path = self
            .config
            .system_tpm()
            .and_then(|tpm| tpm.state_path.as_deref())
            .is_some();

        self.config
            .system_tpm()
            .map(|tpm| tpm.backend == "emulator")
            .unwrap_or(false)
            && !explicit_state_path
            && crate::state::resolve_tpm_placement_mode(
                &self.central_config,
                &self.runtime_overrides,
            ) == crate::state::TpmPlacementMode::Socket
            && crate::state::resolve_swtpm_binary(&self.central_config, &self.runtime_overrides)
                .is_some()
    }

    pub(super) fn tpm_placement_mode(&self) -> crate::state::TpmPlacementMode {
        crate::state::resolve_tpm_placement_mode(&self.central_config, &self.runtime_overrides)
    }

    pub(super) fn has_primary_passthrough_gpu(&self) -> bool {
        self.config
            .host_pci()
            .iter()
            .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
    }

    pub(super) fn resolve_tpm_socket_path(&self) -> String {
        let vm_state_path = self
            .config
            .system_tpm()
            .and_then(|tpm| tpm.state_path.as_deref());

        crate::state::resolve_tpm_socket_path(
            &self.config.name,
            vm_state_path,
            &self.central_config,
            &self.runtime_overrides,
        )
        .unwrap_or_else(|_| format!("/tmp/ezkvm/{}.swtpm", self.config.name))
    }
}
