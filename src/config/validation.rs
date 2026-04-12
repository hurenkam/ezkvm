//! Configuration validation module
//!
//! Validates VM configurations for correctness and compatibility.

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
    validate_audio_devices, validate_ballooning_config, validate_guest_agent_config,
    validate_hostpci_config, validate_hyperv_config, validate_input_devices,
    validate_iscsi_disk_config, validate_ivshmem_config, validate_numa_config, validate_qmp_config,
    validate_scsi_controller_config, validate_smbios_config, validate_spice_config,
    validate_tpm_config, validate_usb_device_config, validate_xhci_controller_config,
};
use system::validate_system_config;
use vm_options::validate_vm_options;

/// Validate a complete VM configuration
pub fn validate_config(config: &VmConfig) -> Result<()> {
    if config.backend != "qemu" {
        return Err(anyhow!(
            "Unsupported backend: {}. Only 'qemu' is currently supported.",
            config.backend
        ));
    }

    validate_system_config(&config.system)?;
    validate_boot_config(&config.boot)?;
    validate_device_config(&config.devices)?;

    if let Some(tpm) = &config.tpm {
        validate_tpm_config(tpm)?;
    }

    if let Some(guest_agent) = &config.guest_agent {
        validate_guest_agent_config(guest_agent)?;
    }

    if let Some(ballooning) = &config.ballooning {
        validate_ballooning_config(ballooning)?;
    }

    for hostpci in &config.hostpci {
        validate_hostpci_config(hostpci)?;
    }

    for usb_device in &config.usb_devices {
        validate_usb_device_config(usb_device)?;
    }

    for xhci_controller in &config.xhci_controllers {
        validate_xhci_controller_config(xhci_controller)?;
    }

    if let Some(spice) = &config.spice {
        validate_spice_config(spice)?;
    }

    validate_audio_devices(&config.audio_devices, config.spice.as_ref())?;
    validate_input_devices(&config.input_devices)?;

    if let Some(ivshmem) = &config.ivshmem {
        validate_ivshmem_config(ivshmem)?;
    }

    for scsi_controller in &config.scsi_controllers {
        validate_scsi_controller_config(scsi_controller)?;
    }

    for iscsi_disk in &config.iscsi_disks {
        validate_iscsi_disk_config(iscsi_disk)?;
    }

    if let Some(qmp) = &config.qmp {
        validate_qmp_config(qmp)?;
    }

    if let Some(smbios) = &config.smbios {
        validate_smbios_config(smbios)?;
    }

    for numa in &config.numa {
        validate_numa_config(numa)?;
    }

    if let Some(hyperv) = &config.hyperv {
        validate_hyperv_config(hyperv)?;
    }

    validate_vm_options(&config.options)?;

    Ok(())
}
