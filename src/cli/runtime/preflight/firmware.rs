use crate::qemu::firmware_locator::FirmwareCapabilityResolver;
use anyhow::{Result, anyhow};
use std::path::Path;

pub(super) fn ensure_firmware_capabilities(
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
        if runtime_overrides.dry_run {
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

    if runtime_overrides.dry_run {
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

pub(super) fn collect_firmware_warnings(
    config: &crate::config::VmConfig,
    central_config: &crate::config::CentralConfig,
    runtime_overrides: &crate::config::RuntimeCliOverrides,
    report: &mut super::RuntimePreflightReport,
) {
    let firmware = config
        .system_boot()
        .firmware
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if firmware != "uefi" && firmware != "ovmf" {
        return;
    }

    if let Some(explicit_code) = config.system_boot().uefi_code.as_deref() {
        if !Path::new(explicit_code).exists() {
            report.push_warning(format!(
                "UEFI firmware code '{}' does not exist",
                explicit_code
            ));
        }
        return;
    }

    let secure_boot = config.system_boot().secure_boot;
    let resolver =
        crate::qemu::CentralFirmwareCapabilityResolver::new(central_config, runtime_overrides);
    if resolver.resolve_ovmf_code(secure_boot).is_none() {
        let search_dirs = central_config.ovmf_search_dirs_with_overrides(runtime_overrides);
        let searched = if search_dirs.is_empty() {
            "/usr/share/ovmf, /usr/share/OVMF".to_string()
        } else {
            search_dirs.join(", ")
        };
        report.push_warning(format!(
            "UEFI firmware requested but no OVMF file found in: {}",
            searched
        ));
    }
}
