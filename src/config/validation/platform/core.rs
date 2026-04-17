use anyhow::{Result, anyhow};

use crate::config::{
    AppleSmcConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig, QmpConfig,
    SmbiosConfig, SpiceConfig, TpmConfig, VncConfig,
};

use super::helpers::is_valid_pci_address;

pub(crate) fn validate_tpm_config(tpm: &TpmConfig) -> Result<()> {
    let valid_versions = ["1.2", "2.0"];
    if !valid_versions.contains(&tpm.version.as_str()) {
        return Err(anyhow!(
            "Unsupported TPM version: {}. Supported: {:?}",
            tpm.version,
            valid_versions
        ));
    }

    let valid_backends = ["emulator", "passthrough"];
    if !valid_backends.contains(&tpm.backend.as_str()) {
        return Err(anyhow!(
            "Unsupported TPM backend: {}. Supported: {:?}",
            tpm.backend,
            valid_backends
        ));
    }

    let valid_models = ["tpm-tis", "tpm-crb"];
    if !valid_models.contains(&tpm.model.as_str()) {
        return Err(anyhow!(
            "Unsupported TPM model: {}. Supported: {:?}",
            tpm.model,
            valid_models
        ));
    }

    Ok(())
}

pub(crate) fn validate_guest_agent_config(guest_agent: &GuestAgentConfig) -> Result<()> {
    if let Some(bus) = &guest_agent.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Guest agent bus cannot be empty"));
    }

    if let Some(addr) = &guest_agent.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("Guest agent address cannot be empty"));
    }

    Ok(())
}

pub(crate) fn validate_ballooning_config(ballooning: &BallooningConfig) -> Result<()> {
    let valid_models = ["virtio-balloon-pci", "virtio-balloon-ccw"];
    if !valid_models.contains(&ballooning.model.as_str()) {
        return Err(anyhow!(
            "Unsupported balloon model: {}. Supported: {:?}",
            ballooning.model,
            valid_models
        ));
    }

    if let Some(id) = &ballooning.id
        && id.trim().is_empty()
    {
        return Err(anyhow!("Balloon device id cannot be empty"));
    }

    if let Some(bus) = &ballooning.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Balloon device bus cannot be empty"));
    }

    if let Some(addr) = &ballooning.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("Balloon device address cannot be empty"));
    }

    Ok(())
}

pub(crate) fn validate_hostpci_config(hostpci: &HostPciConfig) -> Result<()> {
    if !is_valid_pci_address(&hostpci.device) {
        return Err(anyhow!(
            "Invalid PCI device address format: {}",
            hostpci.device
        ));
    }

    if let Some(bus) = &hostpci.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("Host PCI guest bus cannot be empty"));
    }

    if let Some(addr) = &hostpci.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("Host PCI guest address cannot be empty"));
    }

    if let Some(romfile) = &hostpci.romfile
        && !std::path::Path::new(romfile).exists()
    {
        eprintln!("Warning: ROM file '{}' does not exist", romfile);
    }

    Ok(())
}

pub(crate) fn validate_spice_config(spice: &SpiceConfig) -> Result<()> {
    if spice.port == 0 {
        return Err(anyhow!(
            "Invalid SPICE port: {}. Must be between 1 and 65535",
            spice.port
        ));
    }

    if spice.addr.is_empty() {
        return Err(anyhow!("SPICE address cannot be empty"));
    }

    Ok(())
}

pub(crate) fn validate_vnc_config(vnc: &VncConfig) -> Result<()> {
    if !vnc.enabled {
        return Ok(());
    }

    if vnc.display.trim().is_empty() {
        return Err(anyhow!("VNC display cannot be empty when enabled"));
    }

    Ok(())
}

pub(crate) fn validate_qmp_config(qmp: &QmpConfig) -> Result<()> {
    if let Some(socket_path) = &qmp.socket_path {
        if socket_path.is_empty() {
            return Err(anyhow!("QMP socket path cannot be empty"));
        }
        if !socket_path.starts_with('/') {
            return Err(anyhow!("QMP socket path must be an absolute path"));
        }
    }

    Ok(())
}

pub(crate) fn validate_smbios_config(smbios: &SmbiosConfig) -> Result<()> {
    if smbios.smbios_type != 1 && smbios.smbios_type != 2 {
        return Err(anyhow!(
            "SMBIOS type must be either 1 or 2, got {}",
            smbios.smbios_type
        ));
    }

    if let Some(uuid) = &smbios.uuid
        && (uuid.len() != 36 || !uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
    {
        return Err(anyhow!(
            "SMBIOS UUID must be in format XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
        ));
    }

    if let Some(vm_gen_id) = &smbios.vm_generation_id
        && (vm_gen_id.len() != 36 || !vm_gen_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
    {
        return Err(anyhow!(
            "VM generation ID must be in format XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
        ));
    }

    Ok(())
}

pub(crate) fn validate_applesmc_config(applesmc: &AppleSmcConfig) -> Result<()> {
    if applesmc.enabled && applesmc.osk.trim().is_empty() {
        return Err(anyhow!("AppleSMC OSK cannot be empty when enabled"));
    }

    Ok(())
}

pub(crate) fn validate_hyperv_config(hyperv: &HypervConfig) -> Result<()> {
    if let Some(vendor_id) = &hyperv.vendor_id
        && vendor_id.len() > 12
    {
        return Err(anyhow!("Hyper-V vendor_id cannot exceed 12 characters"));
    }

    if let Some(retry_count) = hyperv.spinlock_retry
        && retry_count == 0
    {
        return Err(anyhow!(
            "Hyper-V spinlock_retry must be between 1 and 4294967295"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_applesmc_config, validate_smbios_config};
    use crate::config::{AppleSmcConfig, SmbiosConfig};

    #[test]
    fn smbios_type_rejects_unsupported_values() {
        let smbios = SmbiosConfig {
            smbios_type: 3,
            manufacturer: None,
            product: None,
            version: None,
            serial: None,
            uuid: None,
            sku: None,
            family: None,
            vm_generation_id: None,
        };

        let err = validate_smbios_config(&smbios).expect_err("must fail");
        assert!(
            err.to_string()
                .contains("SMBIOS type must be either 1 or 2")
        );
    }

    #[test]
    fn smbios_type_accepts_type_2() {
        let smbios = SmbiosConfig {
            smbios_type: 2,
            manufacturer: None,
            product: None,
            version: None,
            serial: None,
            uuid: None,
            sku: None,
            family: None,
            vm_generation_id: None,
        };

        validate_smbios_config(&smbios).expect("must succeed");
    }

    #[test]
    fn applesmc_requires_non_empty_osk() {
        let applesmc = AppleSmcConfig {
            enabled: true,
            osk: "   ".to_string(),
        };

        let err = validate_applesmc_config(&applesmc).expect_err("must fail");
        assert!(
            err.to_string()
                .contains("AppleSMC OSK cannot be empty when enabled")
        );
    }
}
