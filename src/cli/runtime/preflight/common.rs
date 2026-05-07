use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn ensure_readconfig_files_present(config: &crate::config::VmConfig) -> Result<()> {
    for path in &config.system.readconfig {
        if Path::new(path).exists() {
            continue;
        }

        return Err(anyhow!(
            "preflight failed: system.readconfig path '{}' does not exist on this host",
            path
        ));
    }

    Ok(())
}

pub(super) fn ensure_program_available(label: &str, program: &str) -> Result<()> {
    if program_available(program) {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: required {} '{}' is not available",
        label,
        program.trim()
    ))
}

pub(super) fn ensure_host_capability_policy(
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    if let Some(preferred_backend) = central_config.network_backend_preference() {
        let normalized = preferred_backend.trim();
        let valid = matches!(
            normalized,
            "bridge" | "bridge-helper" | "user" | "user-mode"
        );
        if !valid {
            return Err(anyhow!(
                "preflight failed: host_capabilities.network.preferred_backend '{}' is invalid (expected bridge, bridge-helper, user, or user-mode)",
                preferred_backend
            ));
        }
    }

    if let Some(placement_mode) = central_config.tpm_placement_mode() {
        let normalized = placement_mode.trim();
        if !matches!(normalized, "socket" | "state-file") {
            return Err(anyhow!(
                "preflight failed: host_capabilities.tpm.placement_mode '{}' is invalid (expected socket or state-file)",
                placement_mode
            ));
        }
    }

    Ok(())
}

pub(super) fn ensure_runtime_dir_access(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let run_dir =
        crate::state::resolve_runtime_root_with_source(None, central_config, runtime_overrides)
            .value
            .unwrap_or_else(|| "/tmp/ezkvm".to_string());

    ensure_dir_is_writable_or_creatable(Path::new(&run_dir), "runtime run directory")
}

pub(super) fn ensure_socket_dir_access(config: &crate::config::VmConfig) -> Result<()> {
    if let Some(guest_agent) = config.options_guest_agent()
        && guest_agent.enabled
        && let Some(path) = guest_agent.socket_path.as_deref()
    {
        ensure_parent_dir_is_writable_or_creatable(path, "guest agent socket")?;
    }

    if let Some(qmp) = config.options_qmp()
        && qmp.enabled
        && let crate::config::QmpSocketType::Unix = qmp.socket_type
        && let Some(path) = qmp.socket_path.as_deref()
    {
        ensure_parent_dir_is_writable_or_creatable(path, "QMP socket")?;
    }

    Ok(())
}

pub(super) fn ensure_parent_dir_is_writable_or_creatable(path: &str, label: &str) -> Result<()> {
    let parent = Path::new(path).parent().ok_or_else(|| {
        anyhow!(
            "preflight failed: {} '{}' does not have a parent directory",
            label,
            path
        )
    })?;
    ensure_dir_is_writable_or_creatable(parent, label)
}

pub(super) fn ensure_dir_is_writable_or_creatable(path: &Path, label: &str) -> Result<()> {
    if path.exists() {
        let metadata = std::fs::metadata(path).map_err(|err| {
            anyhow!(
                "preflight failed: {} '{}' metadata could not be read: {}",
                label,
                path.display(),
                err
            )
        })?;
        if !metadata.is_dir() {
            return Err(anyhow!(
                "preflight failed: {} '{}' is not a directory",
                label,
                path.display()
            ));
        }
        if metadata.permissions().readonly() {
            return Err(anyhow!(
                "preflight failed: {} '{}' is not writable",
                label,
                path.display()
            ));
        }
        return Ok(());
    }

    let ancestor = nearest_existing_ancestor(path).ok_or_else(|| {
        anyhow!(
            "preflight failed: {} '{}' has no existing parent directory",
            label,
            path.display()
        )
    })?;
    let metadata = std::fs::metadata(&ancestor).map_err(|err| {
        anyhow!(
            "preflight failed: {} '{}' metadata could not be read: {}",
            label,
            ancestor.display(),
            err
        )
    })?;

    if !metadata.is_dir() {
        return Err(anyhow!(
            "preflight failed: {} parent '{}' is not a directory",
            label,
            ancestor.display()
        ));
    }
    if metadata.permissions().readonly() {
        return Err(anyhow!(
            "preflight failed: {} parent '{}' is not writable",
            label,
            ancestor.display()
        ));
    }

    Ok(())
}

pub(super) fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut cursor = path;
    while !cursor.exists() {
        cursor = cursor.parent()?;
    }
    Some(cursor.to_path_buf())
}

pub(super) fn program_available(program: &str) -> bool {
    let trimmed = program.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.contains('/') {
        return Path::new(trimmed).exists();
    }

    Command::new("which")
        .arg(trimmed)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
