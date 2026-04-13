use crate::config::{CentralConfig, VmConfig};

/// Main QEMU manager
pub struct QemuManager {
    pub(super) config: VmConfig,
    pub(super) central_config: CentralConfig,
}

impl QemuManager {
    /// Create a new QEMU manager for a VM configuration
    pub fn new(config: VmConfig, central_config: CentralConfig) -> Self {
        Self {
            config,
            central_config,
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
            && self.central_config.tools.swtpm.is_some()
    }

    pub(super) fn has_primary_passthrough_gpu(&self) -> bool {
        self.config
            .host_pci()
            .iter()
            .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
    }

    pub(super) fn resolve_tpm_socket_path(&self) -> String {
        if let Some(tpm) = self.config.system_tpm()
            && let Some(state_path) = &tpm.state_path
        {
            return state_path.clone();
        }

        if let Some(run_dir) = &self.central_config.locations.run_dir {
            return format!("{}/{}.swtpm", run_dir, self.config.name);
        }

        format!("/var/run/qemu-server/{}.swtpm", self.config.name)
    }
}
