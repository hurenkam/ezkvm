use anyhow::{Result, anyhow};
use std::path::Path;

pub(super) fn collect_network_capability_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    report: &mut super::RuntimePreflightReport,
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

pub(super) fn warn_if_bridge_socket_unavailable(report: &mut super::RuntimePreflightReport) {
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

pub(super) fn ensure_bridge_helper_acl_requirements(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
) -> Result<()> {
    let bridge_acl = Path::new("/etc/qemu/bridge.conf");

    for network in &config.devices.networks {
        let outcome = crate::state::resolve_network_outcome(&config.name, network, central_config);
        if outcome.mode == crate::state::NetworkResolutionMode::BridgeHelper {
            ensure_bridge_helper_acl_exists(bridge_acl)?;

            if let Some(bridge_name) = outcome
                .network
                .backend
                .as_ref()
                .and_then(|backend| backend.bridge.as_deref())
            {
                ensure_bridge_helper_acl_allows_bridge(bridge_acl, bridge_name, &network.id)?;
            }
        }
    }

    Ok(())
}

pub(super) fn ensure_bridge_helper_acl_exists(bridge_acl: &Path) -> Result<()> {
    if bridge_acl.exists() {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: bridge backend resolved to qemu-bridge-helper, but '{}' does not exist. qemu-bridge-helper requires this ACL file; create it and add allowed bridges (for example: allow vmbr0)",
        bridge_acl.display()
    ))
}

pub(super) fn ensure_bridge_helper_acl_allows_bridge(
    bridge_acl: &Path,
    bridge_name: &str,
    network_id: &str,
) -> Result<()> {
    let rules = std::fs::read_to_string(bridge_acl).map_err(|err| {
        anyhow!(
            "preflight failed: bridge-helper ACL file '{}' could not be read: {}",
            bridge_acl.display(),
            err
        )
    })?;

    if bridge_acl_allows_bridge(&rules, bridge_name) {
        return Ok(());
    }

    Err(anyhow!(
        "preflight failed: network '{}' resolves to qemu-bridge-helper bridge '{}' but '{}' does not allow it. Add 'allow {}' (or 'allow all') to the ACL file",
        network_id,
        bridge_name,
        bridge_acl.display(),
        bridge_name
    ))
}

pub(super) fn bridge_acl_allows_bridge(rules: &str, bridge_name: &str) -> bool {
    for raw_line in rules.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(action) = parts.next() else {
            continue;
        };
        let Some(target) = parts.next() else {
            continue;
        };

        if parts.next().is_some() {
            continue;
        }

        let matches_bridge = target == "all" || target == bridge_name;
        if !matches_bridge {
            continue;
        }

        match action {
            "allow" => return true,
            "deny" => return false,
            _ => continue,
        }
    }

    false
}
