use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::config::{
    HugepagesConfig, InputDeviceConfig, IommuConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig,
    SataControllerConfig, ScsiControllerConfig,
};

pub(crate) fn validate_input_devices(input_devices: &[InputDeviceConfig]) -> Result<()> {
    let valid_types = ["virtio-mouse", "virtio-keyboard"];
    let mut seen_types = HashSet::new();

    for input_device in input_devices {
        if !valid_types.contains(&input_device.r#type.as_str()) {
            return Err(anyhow!(
                "Unsupported input device type: {}. Supported: {:?}",
                input_device.r#type,
                valid_types
            ));
        }

        if !seen_types.insert(input_device.r#type.as_str()) {
            return Err(anyhow!(
                "Duplicate input device type configured: {}",
                input_device.r#type
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_ivshmem_config(ivshmem: &IvshmemConfig) -> Result<()> {
    if ivshmem.size == 0 {
        return Err(anyhow!("ivshmem size must be greater than 0"));
    }
    if ivshmem.size > 1024 {
        return Err(anyhow!("ivshmem size cannot exceed 1024 MiB"));
    }

    if ivshmem.vectors == 0 || ivshmem.vectors > 32 {
        return Err(anyhow!("ivshmem vectors must be between 1 and 32"));
    }

    if ivshmem.id.trim().is_empty() {
        return Err(anyhow!("ivshmem id cannot be empty"));
    }

    if let Some(bus) = &ivshmem.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("ivshmem bus cannot be empty"));
    }

    if ivshmem.mem_path.trim().is_empty() {
        return Err(anyhow!("ivshmem mem_path cannot be empty"));
    }

    if !ivshmem.mem_path.starts_with('/') {
        return Err(anyhow!("ivshmem mem_path must be an absolute path"));
    }

    Ok(())
}

pub(crate) fn validate_scsi_controller_config(
    scsi_controller: &ScsiControllerConfig,
) -> Result<()> {
    let valid_types = [
        "virtio-scsi-single",
        "virtio-scsi-pci",
        "pvscsi",
        "lsi",
        "lsi53c895a",
        "megasas",
        "megasas-gen2",
    ];
    if !valid_types.contains(&scsi_controller.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported SCSI controller type: {}. Supported: {:?}",
            scsi_controller.r#type,
            valid_types
        ));
    }

    if let Some(max_targets) = scsi_controller.max_targets
        && (max_targets == 0 || max_targets > 256)
    {
        return Err(anyhow!("max_targets must be between 1 and 256"));
    }

    if let Some(bus) = &scsi_controller.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("SCSI controller bus cannot be empty"));
    }

    if let Some(addr) = &scsi_controller.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("SCSI controller addr cannot be empty"));
    }

    Ok(())
}

pub(crate) fn validate_sata_controller_config(
    sata_controller: &SataControllerConfig,
) -> Result<()> {
    let valid_types = ["ahci"];
    if !valid_types.contains(&sata_controller.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported SATA controller type: {}. Supported: {:?}",
            sata_controller.r#type,
            valid_types
        ));
    }

    // Allow empty IDs: they are auto-generated during runtime deserialization (VmConfig::from_str).
    // Empty IDs are intentionally preserved in import output YAML to match the original mapper output.

    if let Some(bus) = &sata_controller.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("SATA controller bus cannot be empty"));
    }

    if let Some(addr) = &sata_controller.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("SATA controller addr cannot be empty"));
    }

    Ok(())
}

pub(crate) fn validate_iscsi_disk_config(iscsi_disk: &IscsiDiskConfig) -> Result<()> {
    if !iscsi_disk.portal.contains(':') {
        return Err(anyhow!("iSCSI portal must be in format 'host:port'"));
    }

    if !iscsi_disk.target.starts_with("iqn.") {
        return Err(anyhow!(
            "iSCSI target must be a valid IQN starting with 'iqn.'"
        ));
    }

    if iscsi_disk.lun > 255 {
        return Err(anyhow!("iSCSI LUN cannot exceed 255"));
    }

    if let Some(initiator) = &iscsi_disk.initiator
        && !initiator.starts_with("iqn.")
    {
        return Err(anyhow!(
            "iSCSI initiator must be a valid IQN starting with 'iqn.'"
        ));
    }

    match (&iscsi_disk.username, &iscsi_disk.password) {
        (Some(username), Some(password)) => {
            if username.trim().is_empty() {
                return Err(anyhow!("iSCSI username cannot be empty"));
            }
            if password.trim().is_empty() {
                return Err(anyhow!("iSCSI password cannot be empty"));
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(anyhow!(
                "iSCSI authentication requires both username and password"
            ));
        }
        (None, None) => {}
    }

    Ok(())
}

pub(crate) fn validate_numa_config(numa: &NumaConfig) -> Result<()> {
    if numa.memory == 0 {
        return Err(anyhow!("NUMA node memory must be greater than 0"));
    }

    if numa.cpus.is_empty() {
        return Err(anyhow!("NUMA node must have at least one CPU"));
    }

    for cpu in &numa.cpus {
        if *cpu > 1023 {
            return Err(anyhow!("CPU ID {} exceeds maximum of 1023", cpu));
        }
    }

    let mut seen_cpus = HashSet::new();
    for cpu in &numa.cpus {
        if !seen_cpus.insert(cpu) {
            return Err(anyhow!("Duplicate CPU ID {} in NUMA node {}", cpu, numa.id));
        }
    }

    Ok(())
}

pub(crate) fn validate_hugepages_config(hp: &HugepagesConfig) -> Result<()> {
    if let Some(path) = &hp.mem_path
        && path.trim().is_empty()
    {
        return Err(anyhow!("hugepages mem_path cannot be empty"));
    }

    if let Some(size) = hp.size_kib
        && size == 0
    {
        return Err(anyhow!("hugepages size_kib must be greater than 0"));
    }

    Ok(())
}

pub(crate) fn validate_iommu_config(iommu: &IommuConfig) -> Result<()> {
    let valid_types = ["intel", "amd"];
    if !valid_types.contains(&iommu.r#type.as_str()) {
        return Err(anyhow!(
            "Unsupported IOMMU type: {}. Supported: {:?}",
            iommu.r#type,
            valid_types
        ));
    }

    if iommu.id.trim().is_empty() {
        return Err(anyhow!("IOMMU id cannot be empty"));
    }

    if iommu.r#type == "amd" && iommu.eim {
        return Err(anyhow!("EIM is only supported for Intel IOMMU"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_hugepages_config, validate_iommu_config, validate_sata_controller_config,
    };
    use crate::config::{HugepagesConfig, IommuConfig, SataControllerConfig};

    #[test]
    fn sata_controller_rejects_unsupported_type() {
        let controller = SataControllerConfig {
            id: "sata0".to_string(),
            r#type: "ich9-ahci".to_string(),
            bus: None,
            addr: None,
        };

        let err = validate_sata_controller_config(&controller).expect_err("must fail");
        assert!(err.to_string().contains("Unsupported SATA controller type"));
    }

    #[test]
    fn sata_controller_rejects_empty_bus_and_addr() {
        let controller = SataControllerConfig {
            id: "sata0".to_string(),
            r#type: "ahci".to_string(),
            bus: Some(" ".to_string()),
            addr: Some(" ".to_string()),
        };

        let err = validate_sata_controller_config(&controller).expect_err("must fail");
        assert!(
            err.to_string()
                .contains("SATA controller bus cannot be empty")
        );
    }

    #[test]
    fn iommu_rejects_unsupported_type() {
        let iommu = IommuConfig {
            r#type: "arm-smmu".to_string(),
            id: "iommu0".to_string(),
            intremap: true,
            caching_mode: false,
            eim: false,
        };
        let err = validate_iommu_config(&iommu).expect_err("must fail");
        assert!(err.to_string().contains("Unsupported IOMMU type"));
    }

    #[test]
    fn iommu_rejects_empty_id() {
        let iommu = IommuConfig {
            r#type: "intel".to_string(),
            id: "  ".to_string(),
            intremap: true,
            caching_mode: false,
            eim: false,
        };
        let err = validate_iommu_config(&iommu).expect_err("must fail");
        assert!(err.to_string().contains("IOMMU id cannot be empty"));
    }

    #[test]
    fn iommu_rejects_eim_on_amd() {
        let iommu = IommuConfig {
            r#type: "amd".to_string(),
            id: "iommu0".to_string(),
            intremap: false,
            caching_mode: false,
            eim: true,
        };
        let err = validate_iommu_config(&iommu).expect_err("must fail");
        assert!(err.to_string().contains("EIM is only supported for Intel"));
    }

    #[test]
    fn hugepages_rejects_empty_mem_path() {
        let hp = HugepagesConfig {
            enabled: true,
            size_kib: None,
            mem_path: Some("  ".to_string()),
            prealloc: true,
        };
        let err = validate_hugepages_config(&hp).expect_err("must fail");
        assert!(err.to_string().contains("mem_path cannot be empty"));
    }

    #[test]
    fn hugepages_rejects_zero_size_kib() {
        let hp = HugepagesConfig {
            enabled: true,
            size_kib: Some(0),
            mem_path: None,
            prealloc: true,
        };
        let err = validate_hugepages_config(&hp).expect_err("must fail");
        assert!(err.to_string().contains("size_kib must be greater than 0"));
    }

    #[test]
    fn hugepages_accepts_valid_config() {
        let hp = HugepagesConfig {
            enabled: true,
            size_kib: Some(1048576),
            mem_path: None,
            prealloc: true,
        };
        validate_hugepages_config(&hp).expect("must succeed");
    }
}
