use anyhow::{Result, anyhow};

pub(super) fn ensure_looking_glass_capabilities(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    if !should_launch_looking_glass(config) {
        return Ok(());
    }

    match crate::state::resolve_looking_glass_program(
        config.options.looking_glass.as_ref(),
        central_config,
        runtime_overrides,
    ) {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow!("preflight failed: {}", err)),
    }
}

pub(super) fn should_launch_remote_viewer(config: &crate::config::VmConfig) -> bool {
    if has_primary_passthrough_gpu(config) {
        return false;
    }

    matches!(&config.spice, Some(spice) if spice.enabled)
        || matches!(&config.vnc, Some(vnc) if vnc.enabled)
}

pub(super) fn should_launch_looking_glass(config: &crate::config::VmConfig) -> bool {
    if !has_primary_passthrough_gpu(config) {
        return false;
    }

    matches!(config.system_memory_ivshmem(), Some(ivshmem) if ivshmem.enabled)
}

pub(super) fn has_primary_passthrough_gpu(config: &crate::config::VmConfig) -> bool {
    config
        .host_pci()
        .iter()
        .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
}
