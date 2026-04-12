use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::config::{
    AudioDeviceConfig, BallooningConfig, GuestAgentConfig, HostPciConfig, HypervConfig,
    InputDeviceConfig, IscsiDiskConfig, IvshmemConfig, NumaConfig, QmpConfig, ScsiControllerConfig,
    SmbiosConfig, SpiceConfig, TpmConfig, UsbDeviceConfig, XhciControllerConfig,
};

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

pub(crate) fn validate_usb_device_config(usb_device: &UsbDeviceConfig) -> Result<()> {
    let has_host_spec = !usb_device.host.trim().is_empty();
    let has_hostbus = usb_device
        .hostbus
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_hostport = usb_device
        .hostport
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    if has_host_spec {
        if !is_valid_usb_spec(&usb_device.host) {
            return Err(anyhow!(
                "Invalid USB device specification: {}. Expected format: 'bus-port.path', 'vendor:product', or use hostbus/hostport fields",
                usb_device.host
            ));
        }
    } else if !(has_hostbus && has_hostport) {
        return Err(anyhow!(
            "USB device '{}' requires either host or both hostbus and hostport",
            usb_device.id
        ));
    }

    if has_hostbus
        && !usb_device
            .hostbus
            .as_ref()
            .unwrap()
            .chars()
            .all(|c| c.is_ascii_digit())
    {
        return Err(anyhow!(
            "USB hostbus must be numeric: {}",
            usb_device.hostbus.as_ref().unwrap()
        ));
    }

    if has_hostport && !is_valid_usb_hostport(usb_device.hostport.as_ref().unwrap()) {
        return Err(anyhow!(
            "Invalid USB hostport format: {}",
            usb_device.hostport.as_ref().unwrap()
        ));
    }

    Ok(())
}

pub(crate) fn validate_xhci_controller_config(
    xhci_controller: &XhciControllerConfig,
) -> Result<()> {
    if xhci_controller.id.trim().is_empty() {
        return Err(anyhow!("XHCI controller id cannot be empty"));
    }

    if let Some(p2) = xhci_controller.p2
        && p2 == 0
    {
        return Err(anyhow!(
            "XHCI controller p2 must be greater than 0 when specified"
        ));
    }

    if let Some(p3) = xhci_controller.p3
        && p3 == 0
    {
        return Err(anyhow!(
            "XHCI controller p3 must be greater than 0 when specified"
        ));
    }

    if let Some(bus) = &xhci_controller.bus
        && bus.trim().is_empty()
    {
        return Err(anyhow!("XHCI controller bus cannot be empty"));
    }

    if let Some(addr) = &xhci_controller.addr
        && addr.trim().is_empty()
    {
        return Err(anyhow!("XHCI controller addr cannot be empty"));
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

pub(crate) fn validate_audio_devices(
    audio_devices: &[AudioDeviceConfig],
    spice: Option<&SpiceConfig>,
) -> Result<()> {
    if audio_devices.is_empty() {
        if let Some(spice) = spice
            && spice.enabled
            && spice.audio
        {
            return Err(anyhow!(
                "SPICE audio requires at least one configured audio device"
            ));
        }
        return Ok(());
    }

    match spice {
        Some(spice) if spice.enabled && spice.audio => {}
        Some(_) => {
            return Err(anyhow!(
                "Audio devices currently require spice.enabled=true and spice.audio=true"
            ));
        }
        None => {
            return Err(anyhow!(
                "Audio devices currently require a SPICE configuration with audio enabled"
            ));
        }
    }

    let mut has_controller = false;

    for audio_device in audio_devices {
        match audio_device.r#type.as_str() {
            "ich9-intel-hda" => {
                has_controller = true;

                if audio_device.id.trim().is_empty() {
                    return Err(anyhow!("Audio controller ID cannot be empty"));
                }

                if audio_device.cad.is_some() {
                    return Err(anyhow!(
                        "Audio controller '{}' cannot define cad",
                        audio_device.id
                    ));
                }

                if audio_device.audiodev.is_some() {
                    return Err(anyhow!(
                        "Audio controller '{}' cannot define audiodev",
                        audio_device.id
                    ));
                }
            }
            "hda-micro" | "hda-duplex" => {
                if audio_device.id.trim().is_empty() {
                    return Err(anyhow!("Audio codec ID cannot be empty"));
                }

                if audio_device.bus.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(anyhow!(
                        "Audio codec '{}' requires a bus assignment",
                        audio_device.id
                    ));
                }

                if audio_device
                    .audiodev
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
                {
                    return Err(anyhow!(
                        "Audio codec '{}' requires an audiodev backend ID",
                        audio_device.id
                    ));
                }

                if audio_device.cad.is_none() {
                    return Err(anyhow!(
                        "Audio codec '{}' requires a cad value",
                        audio_device.id
                    ));
                }
            }
            other => {
                return Err(anyhow!(
                    "Unsupported audio device type: {}. Supported: [\"ich9-intel-hda\", \"hda-micro\", \"hda-duplex\"]",
                    other
                ));
            }
        }
    }

    if !has_controller {
        return Err(anyhow!(
            "Audio device configuration requires an ich9-intel-hda controller"
        ));
    }

    Ok(())
}

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

fn is_valid_pci_address(addr: &str) -> bool {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 3 {
        return false;
    }

    let bus_slot_func: Vec<&str> = parts[2].split('.').collect();
    if bus_slot_func.len() != 2 {
        return false;
    }

    if parts[0].len() != 4 || parts[1].len() != 2 {
        return false;
    }

    if bus_slot_func[0].len() != 2 || bus_slot_func[1].len() != 1 {
        return false;
    }

    u16::from_str_radix(parts[0], 16).is_ok()
        && u8::from_str_radix(parts[1], 16).is_ok()
        && u8::from_str_radix(bus_slot_func[0], 16).is_ok()
        && u8::from_str_radix(bus_slot_func[1], 16).is_ok()
}

fn is_valid_usb_spec(spec: &str) -> bool {
    if spec.contains('-') && spec.contains('.') {
        return true;
    }

    if spec.contains(':') {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() == 2 {
            return u16::from_str_radix(parts[0], 16).is_ok()
                && u16::from_str_radix(parts[1], 16).is_ok();
        }
    }

    false
}

fn is_valid_usb_hostport(hostport: &str) -> bool {
    !hostport.is_empty()
        && hostport
            .split('.')
            .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()))
}
