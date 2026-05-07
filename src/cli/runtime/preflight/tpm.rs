use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use super::common::{
    ensure_dir_is_writable_or_creatable, ensure_parent_dir_is_writable_or_creatable,
    ensure_program_available,
};

pub(super) fn ensure_tpm_capabilities(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let Some(tpm) = config.system_tpm().filter(|tpm| tpm.backend == "emulator") else {
        return Ok(());
    };

    let placement_mode =
        crate::state::resolve_tpm_placement_mode(central_config, runtime_overrides);
    if placement_mode == crate::state::TpmPlacementMode::Socket {
        let swtpm_binary = crate::state::resolve_swtpm_binary(central_config, runtime_overrides)
            .ok_or_else(|| {
                anyhow!(
                    "preflight failed: TPM emulator backend in socket mode requires --swtpm-binary, host_capabilities.tpm.swtpm_binary, PATH swtpm, or legacy tools.swtpm"
                )
            })?;
        ensure_program_available("swtpm binary", &swtpm_binary)?;

        let tpm_socket = resolve_tpm_socket_path(config, central_config, runtime_overrides);
        ensure_parent_dir_is_writable_or_creatable(&tpm_socket, "TPM socket")?;
        ensure_swtpm_apparmor_socket_policy(&tpm_socket)?;

        if !runtime_overrides.dry_run {
            ensure_tpm_backend_uri_local_path_exists(tpm)?;
        }
        ensure_tpm_backend_uri_apparmor_policy(tpm)?;

        if let Some(swtpm_log_path) = resolve_configured_swtpm_log_path(config, central_config) {
            ensure_parent_dir_is_writable_or_creatable(&swtpm_log_path, "swtpm log file")?;
            ensure_swtpm_apparmor_log_policy(&swtpm_log_path)?;
        }
    }

    if placement_mode == crate::state::TpmPlacementMode::StateFile {
        let state_dir = crate::state::resolve_tpm_state_dir(
            tpm.state_dir.as_deref(),
            central_config,
            runtime_overrides,
        )?;
        ensure_dir_is_writable_or_creatable(&state_dir, "TPM state directory")?;
    } else if let Some(state_dir) = tpm.state_dir.as_deref() {
        ensure_dir_is_writable_or_creatable(Path::new(state_dir), "TPM state directory")?;
    }

    Ok(())
}

pub(super) fn ensure_tpm_backend_uri_local_path_exists(
    tpm: &crate::config::TpmConfig,
) -> Result<()> {
    let Some(path) = tpm_backend_uri_local_path(tpm.state_backend_uri.as_deref()) else {
        return Ok(());
    };

    if Path::new(&path).exists() {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: system.tpm.state_backend_uri resolves to local path '{}' but it does not exist on this host",
        path
    ))
}

pub(super) fn ensure_tpm_backend_uri_apparmor_policy(tpm: &crate::config::TpmConfig) -> Result<()> {
    let Some(path) = tpm_backend_uri_local_path(tpm.state_backend_uri.as_deref()) else {
        return Ok(());
    };

    ensure_swtpm_apparmor_path_policy(&path, false, "TPM backend URI path")
}

pub(super) fn ensure_swtpm_apparmor_socket_policy(socket_path: &str) -> Result<()> {
    ensure_swtpm_apparmor_path_policy(socket_path, true, "TPM socket")
}

#[cfg(test)]
pub(super) fn swtpm_apparmor_socket_path_is_allowed(
    socket_path: &str,
    local_override: Option<&str>,
) -> bool {
    swtpm_apparmor_path_is_allowed(socket_path, local_override, true)
}

pub(super) fn ensure_swtpm_apparmor_log_policy(log_path: &str) -> Result<()> {
    ensure_swtpm_apparmor_path_policy(log_path, false, "swtpm log file")
}

pub(super) fn ensure_swtpm_apparmor_path_policy(
    path: &str,
    allow_builtin_socket_patterns: bool,
    label: &str,
) -> Result<()> {
    if !(path.starts_with("/run/")
        || path.starts_with("/var/run/")
        || path.starts_with("/var/log/")
        || path.starts_with("/dev/"))
    {
        return Ok(());
    }

    let profile_path = Path::new("/etc/apparmor.d/usr.bin.swtpm");
    if !profile_path.exists() {
        return Ok(());
    }

    let profile = match std::fs::read_to_string(profile_path) {
        Ok(contents) => contents,
        Err(_) => return Ok(()),
    };
    let local_override = std::fs::read_to_string("/etc/apparmor.d/local/usr.bin.swtpm").ok();

    let has_restrictive_rules =
        profile.contains("/run/libvirt/qemu/swtpm/*.sock") || profile.contains("/run/swtpm/sock");
    if !has_restrictive_rules {
        return Ok(());
    }

    if swtpm_apparmor_path_is_allowed(
        path,
        local_override.as_deref(),
        allow_builtin_socket_patterns,
    ) {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: swtpm AppArmor profile '/etc/apparmor.d/usr.bin.swtpm' is present and {} '{}' is not allowed by policy. Add a local AppArmor override for this path{}",
        label,
        path,
        if allow_builtin_socket_patterns {
            "; built-in socket paths include '/run/libvirt/qemu/swtpm/<name>.sock' and '/run/swtpm/sock'"
        } else {
            ""
        }
    ))
}

pub(super) fn swtpm_apparmor_path_is_allowed(
    path_value: &str,
    local_override: Option<&str>,
    allow_builtin_socket_patterns: bool,
) -> bool {
    let path = Path::new(path_value);

    let file_name = path.file_name().and_then(|name| name.to_str());
    let parent = path.parent();

    let is_libvirt_pattern = allow_builtin_socket_patterns
        && matches!(parent, Some(p) if p == Path::new("/run/libvirt/qemu/swtpm") || p == Path::new("/var/run/libvirt/qemu/swtpm"))
        && file_name
            .map(|name| name.ends_with(".sock"))
            .unwrap_or(false);

    let is_single_socket_pattern = allow_builtin_socket_patterns
        && matches!(parent, Some(p) if p == Path::new("/run/swtpm") || p == Path::new("/var/run/swtpm"))
        && file_name == Some("sock");

    is_libvirt_pattern
        || is_single_socket_pattern
        || local_override
            .map(|rules| apparmor_rules_allow_path(rules, path_value))
            .unwrap_or(false)
}

pub(super) fn resolve_configured_swtpm_log_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Option<String> {
    let base_dir = config
        .options
        .log_dir
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            central_config
                .host_capabilities
                .runtime
                .log_dir
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| {
            central_config
                .locations
                .log_dir
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
        })?;

    Some(
        base_dir
            .join(format!("{}-swtpm.log", config.name))
            .display()
            .to_string(),
    )
}

pub(super) fn apparmor_rules_allow_path(rules: &str, path: &str) -> bool {
    for raw_line in rules.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || !line.starts_with('/') {
            continue;
        }

        let mut tokens = line.split_whitespace();
        let Some(path_token) = tokens.next() else {
            continue;
        };
        let pattern = path_token.trim_end_matches(',');

        if apparmor_glob_matches(pattern, path) {
            return true;
        }
    }
    false
}

pub(super) fn apparmor_glob_matches(pattern: &str, path: &str) -> bool {
    if let Some(star) = pattern.find('*') {
        let prefix = &pattern[..star];
        let suffix = &pattern[star + 1..];
        return path.starts_with(prefix) && path.ends_with(suffix);
    }

    pattern == path
}

pub(super) fn tpm_backend_uri_local_path(uri: Option<&str>) -> Option<String> {
    let value = uri?.trim();
    if value.is_empty() {
        return None;
    }

    let without_prefix = value.strip_prefix("backend-uri=").unwrap_or(value);
    let no_options = without_prefix.split(',').next().unwrap_or(without_prefix);

    if no_options.starts_with('/') {
        return Some(no_options.to_string());
    }

    if let Some(stripped) = no_options.strip_prefix("file://dev/") {
        return Some(format!("/dev/{}", stripped));
    }

    if let Some(stripped) = no_options.strip_prefix("file://") {
        if stripped.starts_with('/') {
            return Some(stripped.to_string());
        }
        return Some(format!("/{}", stripped));
    }

    None
}

pub(super) fn collect_tpm_backend_uri_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    report: &mut super::RuntimePreflightReport,
) {
    let Some(tpm) = config.system_tpm().filter(|t| t.backend == "emulator") else {
        return;
    };
    let placement_mode =
        crate::state::resolve_tpm_placement_mode(central_config, runtime_overrides);
    if placement_mode != crate::state::TpmPlacementMode::Socket {
        return;
    }
    let Some(path) = tpm_backend_uri_local_path(tpm.state_backend_uri.as_deref()) else {
        return;
    };
    if !Path::new(&path).exists() {
        report.push_warning(format!(
            "system.tpm.state_backend_uri resolves to local path '{}' but it does not exist on this host",
            path
        ));
    }
}

pub(super) fn resolve_tpm_socket_path(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> String {
    if let Some(socket_path) = runtime_overrides
        .tpm_socket_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        return socket_path.to_string();
    }

    if let Some(tpm) = config.system_tpm()
        && let Some(state_path) = &tpm.state_path
    {
        return state_path.clone();
    }

    crate::state::resolve_runtime_tpm_socket(&config.name, central_config, runtime_overrides)
        .unwrap_or_else(|_| format!("/tmp/ezkvm/{}.swtpm", config.name))
}
