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
    ensure_host_capability_policy(central_config)?;
    ensure_runtime_dir_access(central_config, runtime_overrides)?;
    ensure_socket_dir_access(config)?;
    ensure_tpm_capabilities(config, central_config, runtime_overrides)?;
    ensure_firmware_capabilities(config, central_config, runtime_overrides)?;
    ensure_network_helper_capabilities(config, central_config)?;

    let mut report = RuntimePreflightReport::default();
    collect_optional_warnings(config, central_config, runtime_overrides, &mut report);
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

        if normalized == "bridge-helper" && central_config.bridge_helper().is_none() {
            return Err(anyhow!(
                "preflight failed: host_capabilities.network.preferred_backend=bridge-helper requires host_capabilities.network.bridge_helper"
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
    let run_dir = central_config
        .runtime_run_dir_with_overrides(runtime_overrides)
        .unwrap_or("/var/run/ezkvm");

    ensure_dir_is_writable_or_creatable(Path::new(run_dir), "runtime run directory")
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
    let explicit_socket_managed = tpm.state_path.is_some();

    if placement_mode == crate::state::TpmPlacementMode::Socket && !explicit_socket_managed {
        let swtpm_binary = crate::state::resolve_swtpm_binary(central_config, runtime_overrides)
            .ok_or_else(|| {
                anyhow!(
                    "preflight failed: TPM emulator backend in socket mode requires --swtpm-binary, host_capabilities.tpm.swtpm_binary, PATH swtpm, or legacy tools.swtpm"
                )
            })?;
        ensure_program_available("swtpm binary", &swtpm_binary)?;
    }

    if placement_mode == crate::state::TpmPlacementMode::Socket {
        let tpm_socket = resolve_tpm_socket_path(config, central_config, runtime_overrides);
        ensure_parent_dir_is_writable_or_creatable(&tpm_socket, "TPM socket")?;
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

fn ensure_network_helper_capabilities(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    for network in &config.devices.networks {
        let Some(backend) = network.backend.as_ref() else {
            continue;
        };
        if backend.backend_type != "bridge" {
            continue;
        }

        if let Some(helper_path) = backend.helper.as_deref().or(central_config.bridge_helper()) {
            ensure_program_available("network bridge helper", helper_path)?;
        }
    }

    Ok(())
}

fn collect_optional_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    report: &mut RuntimePreflightReport,
) {
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

    if !should_launch_looking_glass(config) {
        return;
    }

    match crate::cli::runtime::build_looking_glass_launch(config, central_config, runtime_overrides)
    {
        Ok(Some(launch)) => {
            if !program_available(&launch.program) {
                report.push_warning(format!(
                    "Looking Glass integration disabled because '{}' is not available",
                    launch.program
                ));
            }
        }
        Ok(None) => {
            report.push_warning(
                "Looking Glass integration disabled because no client program is configured"
                    .to_string(),
            );
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
    use super::run_runtime_preflight;
    use crate::config::{CentralConfig, RuntimeCliOverrides, VmConfig};

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
    fn preflight_reports_optional_capability_downgrade_for_missing_looking_glass() {
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
                .any(|warning| warning.contains("Looking Glass integration disabled"))
        );
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
}
