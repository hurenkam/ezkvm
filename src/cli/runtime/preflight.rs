use crate::qemu::firmware_locator::FirmwareCapabilityResolver;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default)]
pub(crate) struct RuntimePreflightReport {
    optional_warnings: Vec<String>,
}

impl RuntimePreflightReport {
    pub(crate) fn optional_warnings(&self) -> &[String] {
        &self.optional_warnings
    }

    fn push_warning(&mut self, warning: String) {
        self.optional_warnings.push(warning);
    }
}

pub(crate) fn run_runtime_preflight(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    qemu_binary: &str,
) -> Result<RuntimePreflightReport> {
    crate::qemu::executor::check_qemu_available(qemu_binary)
        .map_err(|err| anyhow!("preflight failed: {}", err))?;
    let mode = crate::state::detect_runtime_capability_mode(config);

    if mode == crate::state::RuntimeCapabilityMode::PortableLinux {
        ensure_host_capability_policy(central_config)?;
        ensure_runtime_dir_access(central_config, runtime_overrides)?;
        ensure_tpm_capabilities(config, central_config, runtime_overrides)?;
        ensure_firmware_capabilities(config, central_config, runtime_overrides)?;
        ensure_looking_glass_capabilities(config, central_config, runtime_overrides)?;
        ensure_bridge_helper_acl_requirements(config, central_config)?;
    }

    ensure_socket_dir_access(config)?;

    let mut report = RuntimePreflightReport::default();
    collect_optional_warnings(config, central_config, runtime_overrides, mode, &mut report);
    Ok(report)
}

fn ensure_program_available(label: &str, program: &str) -> Result<()> {
    if program_available(program) {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: required {} '{}' is not available",
        label,
        program.trim()
    ))
}

fn ensure_host_capability_policy(central_config: &crate::config::CentralConfig) -> Result<()> {
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

fn ensure_runtime_dir_access(
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let run_dir =
        crate::state::resolve_runtime_root_with_source(None, central_config, runtime_overrides)
            .value
            .unwrap_or_else(|| "/tmp/ezkvm".to_string());

    ensure_dir_is_writable_or_creatable(Path::new(&run_dir), "runtime run directory")
}

fn ensure_socket_dir_access(config: &crate::config::VmConfig) -> Result<()> {
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

fn ensure_tpm_capabilities(
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

        ensure_tpm_backend_uri_local_path_exists(tpm)?;

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

fn ensure_tpm_backend_uri_local_path_exists(tpm: &crate::config::TpmConfig) -> Result<()> {
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

fn ensure_swtpm_apparmor_socket_policy(socket_path: &str) -> Result<()> {
    ensure_swtpm_apparmor_path_policy(socket_path, true, "TPM socket")
}

#[cfg(test)]
fn swtpm_apparmor_socket_path_is_allowed(socket_path: &str, local_override: Option<&str>) -> bool {
    swtpm_apparmor_path_is_allowed(socket_path, local_override, true)
}

fn ensure_swtpm_apparmor_log_policy(log_path: &str) -> Result<()> {
    ensure_swtpm_apparmor_path_policy(log_path, false, "swtpm log file")
}

fn ensure_swtpm_apparmor_path_policy(
    path: &str,
    allow_builtin_socket_patterns: bool,
    label: &str,
) -> Result<()> {
    if !(path.starts_with("/run/") || path.starts_with("/var/run/") || path.starts_with("/var/log/")) {
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

    let has_restrictive_rules = profile.contains("/run/libvirt/qemu/swtpm/*.sock")
        || profile.contains("/run/swtpm/sock");
    if !has_restrictive_rules {
        return Ok(());
    }

    if swtpm_apparmor_path_is_allowed(path, local_override.as_deref(), allow_builtin_socket_patterns)
    {
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

fn swtpm_apparmor_path_is_allowed(
    path_value: &str,
    local_override: Option<&str>,
    allow_builtin_socket_patterns: bool,
) -> bool {
    let path = Path::new(path_value);

    let file_name = path.file_name().and_then(|name| name.to_str());
    let parent = path.parent();

    let is_libvirt_pattern = allow_builtin_socket_patterns
        && matches!(parent, Some(p) if p == Path::new("/run/libvirt/qemu/swtpm") || p == Path::new("/var/run/libvirt/qemu/swtpm"))
        && file_name.map(|name| name.ends_with(".sock")).unwrap_or(false);

    let is_single_socket_pattern = allow_builtin_socket_patterns
        && matches!(parent, Some(p) if p == Path::new("/run/swtpm") || p == Path::new("/var/run/swtpm"))
        && file_name == Some("sock");

    is_libvirt_pattern
        || is_single_socket_pattern
        || local_override
            .map(|rules| apparmor_rules_allow_path(rules, path_value))
            .unwrap_or(false)
}

fn resolve_configured_swtpm_log_path(
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
        })
        ?;

    Some(
        base_dir
            .join(format!("{}-swtpm.log", config.name))
            .display()
            .to_string(),
    )
}

fn apparmor_rules_allow_path(rules: &str, path: &str) -> bool {
    for raw_line in rules.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || !line.starts_with('/') {
            continue;
        }

        // AppArmor path rules terminate with a comma and may have trailing perms.
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

fn apparmor_glob_matches(pattern: &str, path: &str) -> bool {
    if let Some(star) = pattern.find('*') {
        let prefix = &pattern[..star];
        let suffix = &pattern[star + 1..];
        return path.starts_with(prefix) && path.ends_with(suffix);
    }

    pattern == path
}

fn tpm_backend_uri_local_path(uri: Option<&str>) -> Option<String> {
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

fn ensure_firmware_capabilities(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
) -> Result<()> {
    let firmware = config
        .system_boot()
        .firmware
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if firmware != "uefi" && firmware != "ovmf" {
        return Ok(());
    }

    if let Some(explicit_code) = config.system_boot().uefi_code.as_deref() {
        if Path::new(explicit_code).exists() {
            return Ok(());
        }
        return Err(anyhow!(
            "preflight failed: UEFI firmware code '{}' does not exist",
            explicit_code
        ));
    }

    let secure_boot = config.system_boot().secure_boot;
    let resolver =
        crate::qemu::CentralFirmwareCapabilityResolver::new(central_config, runtime_overrides);

    if resolver.resolve_ovmf_code(secure_boot).is_some() {
        return Ok(());
    }

    let search_dirs = central_config.ovmf_search_dirs_with_overrides(runtime_overrides);
    let searched = if search_dirs.is_empty() {
        "/usr/share/ovmf, /usr/share/OVMF".to_string()
    } else {
        search_dirs.join(", ")
    };

    Err(anyhow!(
        "preflight failed: UEFI firmware requested but no OVMF file found in: {}",
        searched
    ))
}

fn collect_optional_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    mode: crate::state::RuntimeCapabilityMode,
    report: &mut RuntimePreflightReport,
) {
    if mode == crate::state::RuntimeCapabilityMode::PortableLinux {
        collect_network_capability_warnings(config, central_config, report);
    }

    if should_launch_remote_viewer(config)
        && let Some(program) =
            central_config.remote_viewer_program_with_overrides(runtime_overrides)
    {
        if !program_available(program) {
            report.push_warning(format!(
                "remote-viewer integration disabled because '{}' is not available",
                program
            ));
        }
    } else if should_launch_remote_viewer(config) {
        report.push_warning(
            "remote-viewer integration disabled because no remote-viewer program is configured"
                .to_string(),
        );
    }

    if mode != crate::state::RuntimeCapabilityMode::PortableLinux
        || !should_launch_looking_glass(config)
    {
        return;
    }

    match crate::state::resolve_looking_glass_program_with_source(
        config.options.looking_glass.as_ref(),
        central_config,
        runtime_overrides,
    ) {
        Ok(resolved) => {
            if let Some(program) = resolved.resolution.value.as_deref()
                && !program_available(program)
            {
                report.push_warning(format!(
                    "Looking Glass integration disabled because '{}' is not available",
                    program
                ));
            }
        }
        Err(err) => {
            report.push_warning(format!(
                "Looking Glass integration disabled due to configuration error: {}",
                err
            ));
        }
    }

    if let Some(ivshmem) = config.system_memory_ivshmem()
        && !Path::new(&ivshmem.mem_path).exists()
    {
        report.push_warning(format!(
            "Looking Glass shared memory path '{}' does not exist on this host",
            ivshmem.mem_path
        ));
    }
}

fn ensure_looking_glass_capabilities(
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

fn collect_network_capability_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    report: &mut RuntimePreflightReport,
) {
    for network in &config.devices.networks {
        let outcome = crate::state::resolve_network_outcome(&config.name, network, central_config);
        if let Some(warning) = outcome.warning {
            report.push_warning(warning);
        }

        if outcome.mode == crate::state::NetworkResolutionMode::BridgeHelper {
            warn_if_bridge_socket_unavailable(report);
        }
    }
}

fn warn_if_bridge_socket_unavailable(report: &mut RuntimePreflightReport) {
    let tun = Path::new("/dev/net/tun");
    if !tun.exists() {
        report.push_warning(
            "bridge backend requested but /dev/net/tun is missing; install or enable tuntap support"
                .to_string(),
        );
        return;
    }

    if std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(tun)
        .is_err()
    {
        report.push_warning(
            "bridge backend requested but /dev/net/tun is not writable; configure permissions or run with required capabilities"
                .to_string(),
        );
    }
}

fn ensure_bridge_helper_acl_requirements(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    let bridge_acl = Path::new("/etc/qemu/bridge.conf");

    for network in &config.devices.networks {
        let outcome = crate::state::resolve_network_outcome(&config.name, network, central_config);
        if outcome.mode == crate::state::NetworkResolutionMode::BridgeHelper {
            return ensure_bridge_helper_acl_exists(bridge_acl);
        }
    }

    Ok(())
}

fn ensure_bridge_helper_acl_exists(bridge_acl: &Path) -> Result<()> {
    if bridge_acl.exists() {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: bridge backend resolved to qemu-bridge-helper, but '{}' does not exist. qemu-bridge-helper requires this ACL file; create it and add allowed bridges (for example: allow vmbr0)",
        bridge_acl.display()
    ))
}

fn should_launch_remote_viewer(config: &crate::config::VmConfig) -> bool {
    if has_primary_passthrough_gpu(config) {
        return false;
    }

    matches!(&config.spice, Some(spice) if spice.enabled)
}

fn should_launch_looking_glass(config: &crate::config::VmConfig) -> bool {
    if !has_primary_passthrough_gpu(config) {
        return false;
    }

    matches!(config.system_memory_ivshmem(), Some(ivshmem) if ivshmem.enabled)
}

fn has_primary_passthrough_gpu(config: &crate::config::VmConfig) -> bool {
    config
        .host_pci()
        .iter()
        .any(|device| device.x_vga || device.id.starts_with("hostpci0"))
}

fn resolve_tpm_socket_path(
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

fn ensure_parent_dir_is_writable_or_creatable(path: &str, label: &str) -> Result<()> {
    let parent = Path::new(path).parent().ok_or_else(|| {
        anyhow!(
            "preflight failed: {} '{}' does not have a parent directory",
            label,
            path
        )
    })?;
    ensure_dir_is_writable_or_creatable(parent, label)
}

fn ensure_dir_is_writable_or_creatable(path: &Path, label: &str) -> Result<()> {
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

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    let mut cursor = path;
    while !cursor.exists() {
        cursor = cursor.parent()?;
    }
    Some(cursor.to_path_buf())
}

fn program_available(program: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{
        ensure_bridge_helper_acl_exists, ensure_bridge_helper_acl_requirements,
        apparmor_glob_matches, apparmor_rules_allow_path, run_runtime_preflight,
        swtpm_apparmor_socket_path_is_allowed,
    };
    use crate::config::{CentralConfig, RuntimeCliOverrides, VmConfig};
    use std::path::Path;

    #[test]
    fn preflight_succeeds_for_minimal_portable_vm() {
        let config = VmConfig::from_str(
            r#"
name: preflight-ok
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
        )
        .expect("vm config should parse");

        let result = run_runtime_preflight(
            &config,
            &CentralConfig::default(),
            &RuntimeCliOverrides::default(),
            "/bin/sh",
        )
        .expect("preflight should pass");

        assert!(result.optional_warnings().is_empty());
    }

    #[test]
    fn preflight_fails_when_tpm_emulator_has_no_swtpm_binary() {
        let config = VmConfig::from_str(
            r#"
name: preflight-tpm
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
  tpm:
    version: "2.0"
    backend: emulator
    model: tpm-tis
devices: {}
"#,
        )
        .expect("vm config should parse");

        let err = run_runtime_preflight(
            &config,
            &CentralConfig::default(),
            &RuntimeCliOverrides {
                swtpm_binary: Some("/definitely/missing/swtpm".to_string()),
                ..Default::default()
            },
            "/bin/sh",
        )
        .expect_err("preflight should fail");

        assert!(
            err.to_string()
                .contains("required swtpm binary '/definitely/missing/swtpm' is not available")
        );
    }

        #[test]
        fn preflight_fails_when_tpm_backend_uri_local_path_missing() {
                let config = VmConfig::from_str(
                        r#"
name: preflight-tpm-uri-missing
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    tpm:
        version: "2.0"
        backend: emulator
        state_backend_uri: file:///definitely/missing/vm-tpm-state
devices: {}
"#,
                )
                .expect("vm config should parse");

                let err = run_runtime_preflight(
                        &config,
                        &CentralConfig::default(),
                        &RuntimeCliOverrides {
                                swtpm_binary: Some("/bin/sh".to_string()),
                        tpm_socket_path: Some("/run/libvirt/qemu/swtpm/preflight.sock".to_string()),
                                ..Default::default()
                        },
                        "/bin/sh",
                )
                .expect_err("preflight should fail");

                assert!(
                        err.to_string()
                                .contains("system.tpm.state_backend_uri resolves to local path '/definitely/missing/vm-tpm-state'")
                );
        }

        #[test]
        fn preflight_accepts_existing_tpm_backend_uri_local_path() {
                let config = VmConfig::from_str(
                        r#"
name: preflight-tpm-uri-existing
backend: qemu
system:
    architecture: x86_64
    machine: q35
    memory:
        size: 1024
    cpu:
        model: host
        vcpus: 2
    tpm:
        version: "2.0"
        backend: emulator
        state_backend_uri: file:///etc/hosts
devices: {}
"#,
                )
                .expect("vm config should parse");

                let result = run_runtime_preflight(
                        &config,
                        &CentralConfig::default(),
                        &RuntimeCliOverrides {
                                swtpm_binary: Some("/bin/sh".to_string()),
                        tpm_socket_path: Some("/run/libvirt/qemu/swtpm/preflight.sock".to_string()),
                                ..Default::default()
                        },
                        "/bin/sh",
                );

                assert!(result.is_ok());
        }

    #[test]
    fn preflight_keeps_shared_memory_warning_when_looking_glass_auto_mode_skips_client() {
        let config = VmConfig::from_str(
            r#"
name: preflight-lg
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
    ivshmem:
      enabled: true
      mem_path: /definitely/missing/kvmfr0
  cpu:
    model: host
    vcpus: 2
devices: {}
host:
  pci:
    - id: hostpci0
      device: "0000:03:00.0"
"#,
        )
        .expect("vm config should parse");

        let result = run_runtime_preflight(
            &config,
            &CentralConfig::default(),
            &RuntimeCliOverrides::default(),
            "/bin/sh",
        )
        .expect("preflight should succeed with optional warnings");

        assert!(
            result
                .optional_warnings()
                .iter()
                .any(|warning| warning.contains("shared memory path"))
        );
    }

    #[test]
    fn preflight_fails_for_invalid_network_backend_policy() {
        let config = VmConfig::from_str(
            r#"
name: preflight-network-policy
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
        )
        .expect("vm config should parse");

        let central: CentralConfig = serde_yaml::from_str(
            r#"
host_capabilities:
  network:
    preferred_backend: invalid-backend
"#,
        )
        .expect("central config should parse");

        let err = run_runtime_preflight(
            &config,
            &central,
            &RuntimeCliOverrides::default(),
            "/bin/sh",
        )
        .expect_err("preflight should fail");

        assert!(
            err.to_string()
                .contains("host_capabilities.network.preferred_backend")
        );
    }

    #[test]
    fn preflight_fails_for_invalid_tpm_placement_mode() {
        let config = VmConfig::from_str(
            r#"
name: preflight-tpm-policy
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices: {}
"#,
        )
        .expect("vm config should parse");

        let central: CentralConfig = serde_yaml::from_str(
            r#"
host_capabilities:
  tpm:
    placement_mode: invalid
"#,
        )
        .expect("central config should parse");

        let err = run_runtime_preflight(
            &config,
            &central,
            &RuntimeCliOverrides::default(),
            "/bin/sh",
        )
        .expect_err("preflight should fail");

        assert!(
            err.to_string()
                .contains("host_capabilities.tpm.placement_mode")
        );
    }

    #[test]
    fn bridge_acl_helper_accepts_existing_file() {
        let existing = Path::new("/etc/hosts");
        let result = ensure_bridge_helper_acl_exists(existing);
        assert!(result.is_ok());
    }

    #[test]
    fn bridge_acl_helper_rejects_missing_file() {
        let missing = Path::new("/definitely/missing/bridge.conf");
        let err = ensure_bridge_helper_acl_exists(missing).expect_err("missing ACL should fail");
        assert!(err.to_string().contains("/definitely/missing/bridge.conf"));
        assert!(err.to_string().contains("qemu-bridge-helper requires this ACL file"));
    }

    #[test]
    fn bridge_acl_requirement_skips_non_bridge_helper_networks() {
        let config = VmConfig::from_str(
            r#"
name: preflight-bridge-acl-skip
backend: qemu
system:
  architecture: x86_64
  machine: q35
  memory:
    size: 1024
  cpu:
    model: host
    vcpus: 2
devices:
  networks:
    - id: net0
      model: virtio-net-pci
      backend:
        type: user
"#,
        )
        .expect("vm config should parse");

        let result = ensure_bridge_helper_acl_requirements(&config, &CentralConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn swtpm_apparmor_path_allows_libvirt_sockets() {
        assert!(swtpm_apparmor_socket_path_is_allowed(
            "/run/libvirt/qemu/swtpm/wakiza.sock",
            None,
        ));
        assert!(swtpm_apparmor_socket_path_is_allowed(
            "/var/run/libvirt/qemu/swtpm/wakiza.sock",
            None,
        ));
    }

    #[test]
    fn swtpm_apparmor_path_allows_single_run_socket() {
        assert!(swtpm_apparmor_socket_path_is_allowed("/run/swtpm/sock", None));
        assert!(swtpm_apparmor_socket_path_is_allowed(
            "/var/run/swtpm/sock",
            None,
        ));
    }

    #[test]
    fn swtpm_apparmor_path_rejects_unlisted_locations() {
        assert!(!swtpm_apparmor_socket_path_is_allowed(
            "/var/run/ezkvm/tpmstate0-tpm.socket",
            None,
        ));
        assert!(!swtpm_apparmor_socket_path_is_allowed(
            "/tmp/ezkvm/wakiza.swtpm",
            None,
        ));
    }

    #[test]
    fn swtpm_apparmor_path_accepts_local_override_glob() {
        let local = "/var/run/ezkvm/*.socket rwk,\n/var/run/ezkvm/*.pid rwk,";
        assert!(swtpm_apparmor_socket_path_is_allowed(
            "/var/run/ezkvm/tpmstate0-tpm.socket",
            Some(local),
        ));
    }

    #[test]
    fn apparmor_glob_matches_simple_single_star() {
        assert!(apparmor_glob_matches(
            "/var/run/ezkvm/*.socket",
            "/var/run/ezkvm/tpmstate0-tpm.socket",
        ));
        assert!(!apparmor_glob_matches(
            "/var/run/ezkvm/*.sock",
            "/var/run/ezkvm/tpmstate0-tpm.socket",
        ));
    }

    #[test]
    fn apparmor_rules_allow_path_ignores_comments_and_permissions() {
        let rules = r#"
# comment
/var/run/ezkvm/*.socket rwk,
/var/log/ezkvm/*.log rwk,
"#;
        assert!(apparmor_rules_allow_path(
            rules,
            "/var/run/ezkvm/tpmstate0-tpm.socket",
        ));
    }
}
