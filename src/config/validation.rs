//! Configuration validation module with multi-stage checks.
//!
//! Performs comprehensive validation of parsed VM configurations following
//! the fail-fast principle: catches errors early with actionable messages.
//! Validation stages: schema > business logic > cross-device constraints.
//!
//! # Design
//! - Validates after deserialization to catch YAML/type issues
//! - Provides descriptive errors using anyhow context
//! - Modular validators per subsystem (system, boot, devices, platform)
//! - Can be extended for custom validation rules in future phases

mod boot;
mod devices;
mod platform;
mod system;
mod vm_options;

use anyhow::{Result, anyhow};

use super::VmConfig;
use boot::validate_boot_config;
use devices::validate_device_config;
use platform::{
    validate_applesmc_config, validate_audio_devices, validate_ballooning_config,
    validate_guest_agent_config, validate_hostpci_config, validate_hugepages_config,
    validate_hyperv_config, validate_input_devices, validate_iommu_config,
    validate_iscsi_disk_config, validate_ivshmem_config, validate_numa_config, validate_qmp_config,
    validate_sata_controller_config, validate_scsi_controller_config, validate_smbios_config,
    validate_spice_config, validate_tpm_config, validate_usb_device_config, validate_vnc_config,
    validate_xhci_controller_config,
};
use system::validate_system_config;
use vm_options::validate_vm_options;

/// Validate a complete VM configuration
/// Validate a complete VM configuration.
///
/// Performs multi-stage validation:
/// 1. Backend support check (currently qemu only)
/// 2. Core sections (system, boot, devices)
/// 3. Optional platform sections (TPM, guest-agent, SPICE, VNC, etc.)
/// 4. Device collections (hostpci, USB, XHCI, audio, input)
/// 5. VM options (RTC, guest-agent, balloon)
///
/// # Arguments
/// * `config` - The VmConfig to validate
///
/// # Returns
/// Ok if config passes all checks, Err with actionable message otherwise
///
/// # Examples
/// ```ignore
/// use ezkvm::config::validation::validate_config;
/// match validate_config(&config) {
///     Ok(()) => println!("Configuration valid"),
///     Err(e) => eprintln!("Validation error: {}", e),
/// }
/// ```
pub fn validate_config(config: &VmConfig) -> Result<()> {
    validate_backend(config)?;
    validate_core_sections(config)?;
    validate_optional_platform_sections(config)?;
    validate_collections(config)?;
    validate_vm_options(&config.options)?;

    Ok(())
}

fn validate_backend(config: &VmConfig) -> Result<()> {
    if config.backend == "qemu" {
        return Ok(());
    }

    Err(anyhow!(
        "Unsupported backend: {}. Only 'qemu' is currently supported.",
        config.backend
    ))
}

fn validate_core_sections(config: &VmConfig) -> Result<()> {
    validate_system_config(&config.system)?;
    validate_boot_config(&config.system.boot)?;
    validate_device_config(&config.devices)?;
    Ok(())
}

fn validate_optional_platform_sections(config: &VmConfig) -> Result<()> {
    if let Some(tpm) = &config.system.tpm {
        validate_tpm_config(tpm)?;
    }
    if let Some(guest_agent) = &config.options.guest_agent {
        validate_guest_agent_config(guest_agent)?;
    }
    if let Some(ballooning) = &config.system.memory.ballooning {
        validate_ballooning_config(ballooning)?;
    }
    if let Some(spice) = &config.spice {
        validate_spice_config(spice)?;
    }
    if let Some(vnc) = &config.vnc {
        validate_vnc_config(vnc)?;
    }
    if let Some(ivshmem) = &config.system.memory.ivshmem {
        validate_ivshmem_config(ivshmem)?;
    }
    if let Some(hugepages) = &config.system.memory.hugepages {
        validate_hugepages_config(hugepages)?;
    }
    if let Some(qmp) = &config.options.qmp {
        validate_qmp_config(qmp)?;
    }
    if let Some(smbios) = &config.system.smbios {
        validate_smbios_config(smbios)?;
    }
    if let Some(applesmc) = &config.system.applesmc {
        validate_applesmc_config(applesmc)?;
    }
    if let Some(hyperv) = &config.hyperv {
        validate_hyperv_config(hyperv)?;
    }
    if let Some(iommu) = &config.iommu {
        validate_iommu_config(iommu)?;
    }
    Ok(())
}

fn validate_collections(config: &VmConfig) -> Result<()> {
    for hostpci in &config.host.pci {
        validate_hostpci_config(hostpci)?;
    }
    for usb_device in &config.host.usb {
        validate_usb_device_config(usb_device)?;
    }
    for xhci_controller in &config.controllers.xhci {
        validate_xhci_controller_config(xhci_controller)?;
    }
    validate_audio_devices(&config.devices.audio, config.spice.as_ref())?;
    validate_input_devices(&config.devices.input)?;
    for scsi_controller in &config.controllers.scsi {
        validate_scsi_controller_config(scsi_controller)?;
    }
    for sata_controller in &config.controllers.sata {
        validate_sata_controller_config(sata_controller)?;
    }
    for iscsi_disk in &config.iscsi_disks {
        validate_iscsi_disk_config(iscsi_disk)?;
    }
    for numa in &config.system.cpu.numa {
        validate_numa_config(numa)?;
    }
    Ok(())
}
