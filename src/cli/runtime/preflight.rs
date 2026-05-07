use anyhow::{Result, anyhow};
use std::path::Path;

mod common;
mod display;
mod firmware;
mod network;
mod tpm;

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
    common::ensure_readconfig_files_present(config)?;
    let mode = crate::state::detect_runtime_capability_mode(config);

    if mode == crate::state::RuntimeCapabilityMode::PortableLinux {
        common::ensure_host_capability_policy(central_config)?;
        common::ensure_runtime_dir_access(central_config, runtime_overrides)?;
        tpm::ensure_tpm_capabilities(config, central_config, runtime_overrides)?;
        firmware::ensure_firmware_capabilities(config, central_config, runtime_overrides)?;
        display::ensure_looking_glass_capabilities(config, central_config, runtime_overrides)?;
        ensure_bridge_helper_acl_requirements(config, central_config)?;
    }

    common::ensure_socket_dir_access(config)?;

    let mut report = RuntimePreflightReport::default();
    collect_optional_warnings(config, central_config, runtime_overrides, mode, &mut report);
    Ok(report)
}

fn collect_optional_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    mode: crate::state::RuntimeCapabilityMode,
    report: &mut RuntimePreflightReport,
) {
    if mode == crate::state::RuntimeCapabilityMode::PortableLinux {
        network::collect_network_capability_warnings(config, central_config, report);
        if runtime_overrides.dry_run {
            tpm::collect_tpm_backend_uri_warnings(
                config,
                central_config,
                runtime_overrides,
                report,
            );
            firmware::collect_firmware_warnings(config, central_config, runtime_overrides, report);
        }
    }

    if display::should_launch_remote_viewer(config)
        && let Some(program) =
            central_config.remote_viewer_program_with_overrides(runtime_overrides)
    {
        if !common::program_available(program) {
            report.push_warning(format!(
                "remote-viewer integration disabled because '{}' is not available",
                program
            ));
        }
    } else if display::should_launch_remote_viewer(config) {
        report.push_warning(
            "remote-viewer integration disabled because no remote-viewer program is configured"
                .to_string(),
        );
    }

    if mode != crate::state::RuntimeCapabilityMode::PortableLinux
        || !display::should_launch_looking_glass(config)
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
                && !common::program_available(program)
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

#[cfg(test)]
fn apparmor_glob_matches(pattern: &str, path: &str) -> bool {
    tpm::apparmor_glob_matches(pattern, path)
}

#[cfg(test)]
fn apparmor_rules_allow_path(rules: &str, path: &str) -> bool {
    tpm::apparmor_rules_allow_path(rules, path)
}

#[cfg(test)]
fn swtpm_apparmor_path_is_allowed(
    path_value: &str,
    local_override: Option<&str>,
    allow_builtin_socket_patterns: bool,
) -> bool {
    tpm::swtpm_apparmor_path_is_allowed(path_value, local_override, allow_builtin_socket_patterns)
}

#[cfg(test)]
fn swtpm_apparmor_socket_path_is_allowed(socket_path: &str, local_override: Option<&str>) -> bool {
    tpm::swtpm_apparmor_socket_path_is_allowed(socket_path, local_override)
}

#[cfg(test)]
fn bridge_acl_allows_bridge(rules: &str, bridge_name: &str) -> bool {
    network::bridge_acl_allows_bridge(rules, bridge_name)
}

#[cfg(test)]
fn ensure_bridge_helper_acl_exists(bridge_acl: &Path) -> Result<()> {
    network::ensure_bridge_helper_acl_exists(bridge_acl)
}

#[cfg(test)]
fn ensure_bridge_helper_acl_allows_bridge(
    bridge_acl: &Path,
    bridge_name: &str,
    network_id: &str,
) -> Result<()> {
    network::ensure_bridge_helper_acl_allows_bridge(bridge_acl, bridge_name, network_id)
}

fn ensure_bridge_helper_acl_requirements(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    network::ensure_bridge_helper_acl_requirements(config, central_config)
}

#[cfg(test)]
mod tests;
