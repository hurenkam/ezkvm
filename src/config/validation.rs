//! Configuration validation module
//!
//! Validates VM configurations for correctness and compatibility.

use super::VmConfig;
use anyhow::{anyhow, Result};

/// Validate a complete VM configuration
pub fn validate_config(config: &VmConfig) -> Result<()> {
    // Validate backend
    if config.backend != "qemu" {
        return Err(anyhow!("Unsupported backend: {}. Only 'qemu' is currently supported.", config.backend));
    }
    
    // Validate system configuration
    validate_system_config(&config.system)?;
    
    // Validate boot configuration
    validate_boot_config(&config.boot)?;
    
    // Validate device configuration
    validate_device_config(&config.devices)?;
    
    // Validate TPM configuration
    if let Some(tpm) = &config.tpm {
        validate_tpm_config(tpm)?;
    }
    
    // Validate guest agent configuration
    if let Some(guest_agent) = &config.guest_agent {
        validate_guest_agent_config(guest_agent)?;
    }
    
    // Validate ballooning configuration
    if let Some(ballooning) = &config.ballooning {
        validate_ballooning_config(ballooning)?;
    }
    
    // Validate hardware passthrough configuration
    for hostpci in &config.hostpci {
        validate_hostpci_config(hostpci)?;
    }
    
    // Validate USB device configuration
    for usb_device in &config.usb_devices {
        validate_usb_device_config(usb_device)?;
    }
    
    // Validate SPICE configuration
    if let Some(spice) = &config.spice {
        validate_spice_config(spice)?;
    }

    // Validate audio device configuration
    validate_audio_devices(&config.audio_devices, config.spice.as_ref())?;

    // Validate input device configuration
    validate_input_devices(&config.input_devices)?;
    
    // Validate ivshmem configuration
    if let Some(ivshmem) = &config.ivshmem {
        validate_ivshmem_config(ivshmem)?;
    }
    
    // Validate SCSI controller configuration
    for scsi_controller in &config.scsi_controllers {
        validate_scsi_controller_config(scsi_controller)?;
    }
    
    // Validate iSCSI disk configuration
    for iscsi_disk in &config.iscsi_disks {
        validate_iscsi_disk_config(iscsi_disk)?;
    }
    
    // Validate QMP configuration
    if let Some(qmp) = &config.qmp {
        validate_qmp_config(qmp)?;
    }
    
    // Validate SMBIOS configuration
    if let Some(smbios) = &config.smbios {
        validate_smbios_config(smbios)?;
    }
    
    // Validate NUMA configuration
    for numa in &config.numa {
        validate_numa_config(numa)?;
    }
    
    // Validate Hyper-V configuration
    if let Some(hyperv) = &config.hyperv {
        validate_hyperv_config(hyperv)?;
    }

    validate_vm_options(&config.options)?;
    
    Ok(())
}

/// Validate system configuration
fn validate_system_config(system: &super::SystemConfig) -> Result<()> {
    // Validate architecture
    let valid_architectures = ["x86_64", "aarch64", "x86", "ppc64", "riscv64"];
    if !valid_architectures.contains(&system.architecture.as_str()) {
        return Err(anyhow!("Unsupported architecture: {}. Supported: {:?}", 
                          system.architecture, valid_architectures));
    }
    
    // Validate memory (reasonable bounds)
    if system.memory < 128 {
        return Err(anyhow!("Memory must be at least 128 MiB"));
    }
    if system.memory > 1024 * 1024 { // 1 TiB
        return Err(anyhow!("Memory cannot exceed 1 TiB"));
    }
    
    // Validate vCPUs
    if system.vcpus == 0 {
        return Err(anyhow!("Must have at least 1 vCPU"));
    }
    if system.vcpus > 1024 {
        return Err(anyhow!("Cannot have more than 1024 vCPUs"));
    }
    
    // Validate CPU features
    for feature in &system.cpu_features {
        if !feature.name.starts_with('+') && !feature.name.starts_with('-') {
            return Err(anyhow!("CPU feature '{}' must start with '+' or '-'", feature.name));
        }
    }

    for option in &system.machine_options {
        if option.trim().is_empty() {
            return Err(anyhow!("Machine options cannot be empty"));
        }
        if !option.contains('=') {
            return Err(anyhow!("Machine option '{}' must use key=value format", option));
        }
    }
    
    Ok(())
}

/// Validate boot configuration
fn validate_boot_config(boot: &super::BootConfig) -> Result<()> {
    // Validate firmware
    if let Some(firmware) = &boot.firmware {
        let valid_firmware = ["uefi", "bios", "ovmf"];
        if !valid_firmware.contains(&firmware.as_str()) {
            return Err(anyhow!("Unsupported firmware: {}. Supported: {:?}", 
                              firmware, valid_firmware));
        }
    }
    
    // Validate boot order
    let valid_boot_devices = ["disk", "cdrom", "network", "hd", "cd"];
    for device in &boot.boot_order {
        if !valid_boot_devices.contains(&device.as_str()) {
            return Err(anyhow!("Unsupported boot device: {}. Supported: {:?}", 
                              device, valid_boot_devices));
        }
    }
    
    // Validate UEFI paths if firmware is UEFI
    if let Some(firmware) = &boot.firmware {
        if firmware == "uefi" || firmware == "ovmf" {
            if let Some(code_path) = &boot.uefi_code {
                if !std::path::Path::new(code_path).exists() {
                    eprintln!("Warning: UEFI code path '{}' does not exist", code_path);
                }
            }
            if let Some(vars_path) = &boot.uefi_vars {
                if !std::path::Path::new(vars_path).exists() {
                    eprintln!("Warning: UEFI vars path '{}' does not exist", vars_path);
                }
            }
        }
    }

    if let Some(splash) = &boot.splash {
        if splash.trim().is_empty() {
            return Err(anyhow!("Boot splash path cannot be empty"));
        }
    }
    
    Ok(())
}

/// Validate device configuration
fn validate_device_config(devices: &super::DeviceConfig) -> Result<()> {
    // Validate drives
    for drive in &devices.drives {
        validate_drive_config(drive)?;
    }
    
    // Validate networks
    for network in &devices.networks {
        validate_network_config(network)?;
    }
    
    // Validate displays
    for display in &devices.displays {
        validate_display_config(display)?;
    }
    
    Ok(())
}

/// Validate drive configuration
fn validate_drive_config(drive: &super::DriveConfig) -> Result<()> {
    // Validate interface
    let valid_interfaces = ["virtio", "scsi", "ide", "nvme"];
    if !valid_interfaces.contains(&drive.interface.as_str()) {
        return Err(anyhow!("Unsupported drive interface: {}. Supported: {:?}", 
                          drive.interface, valid_interfaces));
    }
    
    // Validate type
    let valid_types = ["disk", "cdrom"];
    if !valid_types.contains(&drive.r#type.as_str()) {
        return Err(anyhow!("Unsupported drive type: {}. Supported: {:?}", 
                          drive.r#type, valid_types));
    }
    
    // Validate format
    let valid_formats = ["qcow2", "raw", "vmdk", "vdi"];
    if !valid_formats.contains(&drive.format.as_str()) {
        return Err(anyhow!("Unsupported drive format: {}. Supported: {:?}", 
                          drive.format, valid_formats));
    }

    if let Some(cache) = &drive.cache {
        let valid_cache = ["none", "writeback", "writethrough", "unsafe", "directsync"];
        if !valid_cache.contains(&cache.as_str()) {
            return Err(anyhow!("Unsupported drive cache mode: {}. Supported: {:?}", cache, valid_cache));
        }
    }

    if let Some(aio) = &drive.aio {
        let valid_aio = ["threads", "native", "io_uring"];
        if !valid_aio.contains(&aio.as_str()) {
            return Err(anyhow!("Unsupported drive aio mode: {}. Supported: {:?}", aio, valid_aio));
        }
    }

    if let Some(detect_zeroes) = &drive.detect_zeroes {
        let valid_detect_zeroes = ["off", "on", "unmap"];
        if !valid_detect_zeroes.contains(&detect_zeroes.as_str()) {
            return Err(anyhow!("Unsupported detect-zeroes mode: {}. Supported: {:?}", detect_zeroes, valid_detect_zeroes));
        }
    }
    
    // Check if path exists (optional, but warn if not)
    if !std::path::Path::new(&drive.path).exists() {
        eprintln!("Warning: Drive path '{}' does not exist", drive.path);
    }
    
    Ok(())
}

/// Validate network configuration
fn validate_network_config(network: &super::NetworkConfig) -> Result<()> {
    // Validate model
    let valid_models = ["virtio-net", "virtio-net-pci", "e1000", "e1000e", "rtl8139"];
    if !valid_models.contains(&network.model.as_str()) {
        return Err(anyhow!("Unsupported network model: {}. Supported: {:?}", 
                          network.model, valid_models));
    }
    
    // Validate mode
    let valid_mode_prefixes = ["user", "bridge", "socket", "tap"];
    let mode_valid = valid_mode_prefixes.iter()
        .any(|prefix| network.mode.starts_with(prefix));
    if !mode_valid {
        return Err(anyhow!("Unsupported network mode: {}. Must start with one of: {:?}", 
                          network.mode, valid_mode_prefixes));
    }
    
    // Validate MAC address format if provided
    if let Some(mac) = &network.mac {
        if !is_valid_mac_address(mac) {
            return Err(anyhow!("Invalid MAC address format: {}", mac));
        }
    }

    if let Some(rx_queue_size) = network.rx_queue_size {
        if rx_queue_size == 0 {
            return Err(anyhow!("RX queue size must be greater than 0"));
        }
    }

    if let Some(tx_queue_size) = network.tx_queue_size {
        if tx_queue_size == 0 {
            return Err(anyhow!("TX queue size must be greater than 0"));
        }
    }

    if let Some(bus) = &network.bus {
        if bus.trim().is_empty() {
            return Err(anyhow!("Network bus cannot be empty"));
        }
    }

    if let Some(addr) = &network.addr {
        if addr.trim().is_empty() {
            return Err(anyhow!("Network device address cannot be empty"));
        }
    }
    
    Ok(())
}

/// Validate display configuration
fn validate_display_config(display: &super::DisplayConfig) -> Result<()> {
    // Validate type
    let valid_types = ["virtio-gpu", "qxl", "cirrus", "vga", "vmware-svga", "none"];
    if !valid_types.contains(&display.r#type.as_str()) {
        return Err(anyhow!("Unsupported display type: {}. Supported: {:?}", 
                          display.r#type, valid_types));
    }
    
    // Validate VRAM
    if let Some(vram) = display.vram {
        if vram > 1024 { // 1 GiB
            return Err(anyhow!("VRAM cannot exceed 1024 MiB"));
        }
    }
    
    Ok(())
}

/// Check if a string is a valid MAC address
fn is_valid_mac_address(mac: &str) -> bool {
    // Basic MAC address validation (XX:XX:XX:XX:XX:XX format)
    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return false;
    }
    
    for part in parts {
        if part.len() != 2 {
            return false;
        }
        if u8::from_str_radix(part, 16).is_err() {
            return false;
        }
    }
    
    true
}

/// Validate TPM configuration
fn validate_tpm_config(tpm: &super::TpmConfig) -> Result<()> {
    // Validate version
    let valid_versions = ["1.2", "2.0"];
    if !valid_versions.contains(&tpm.version.as_str()) {
        return Err(anyhow!("Unsupported TPM version: {}. Supported: {:?}", 
                          tpm.version, valid_versions));
    }
    
    // Validate backend
    let valid_backends = ["emulator", "passthrough"];
    if !valid_backends.contains(&tpm.backend.as_str()) {
        return Err(anyhow!("Unsupported TPM backend: {}. Supported: {:?}", 
                          tpm.backend, valid_backends));
    }
    
    // Validate model
    let valid_models = ["tpm-tis", "tpm-crb"];
    if !valid_models.contains(&tpm.model.as_str()) {
        return Err(anyhow!("Unsupported TPM model: {}. Supported: {:?}", 
                          tpm.model, valid_models));
    }
    
    Ok(())
}

/// Validate guest agent configuration
fn validate_guest_agent_config(_guest_agent: &super::GuestAgentConfig) -> Result<()> {
    // Basic validation - guest agent config is mostly boolean flags
    // Could add socket path validation if needed
    Ok(())
}

fn validate_vm_options(options: &super::VmOptions) -> Result<()> {
    for global in &options.global_options {
        if global.trim().is_empty() {
            return Err(anyhow!("Global QEMU options cannot be empty"));
        }
    }

    if let Some(rtc) = &options.rtc {
        if let Some(base) = &rtc.base {
            let valid = ["utc", "localtime"];
            if !valid.contains(&base.as_str()) {
                return Err(anyhow!("Unsupported RTC base: {}. Supported: {:?}", base, valid));
            }
        }

        if let Some(driftfix) = &rtc.driftfix {
            let valid = ["none", "slew"];
            if !valid.contains(&driftfix.as_str()) {
                return Err(anyhow!("Unsupported RTC driftfix: {}. Supported: {:?}", driftfix, valid));
            }
        }
    }

    if let Some(pid_file) = &options.pid_file {
        if pid_file.trim().is_empty() {
            return Err(anyhow!("PID file path cannot be empty"));
        }
    }

    if let Some(log_dir) = &options.log_dir {
        if log_dir.trim().is_empty() {
            return Err(anyhow!("Log directory cannot be empty"));
        }
    }

    if let Some(log_keep) = options.log_keep {
        if log_keep == 0 {
            return Err(anyhow!("log_keep must be greater than 0"));
        }
    }

    Ok(())
}

/// Validate ballooning configuration
fn validate_ballooning_config(ballooning: &super::BallooningConfig) -> Result<()> {
    // Validate model
    let valid_models = ["virtio-balloon-pci", "virtio-balloon-ccw"];
    if !valid_models.contains(&ballooning.model.as_str()) {
        return Err(anyhow!("Unsupported balloon model: {}. Supported: {:?}", 
                          ballooning.model, valid_models));
    }
    
    Ok(())
}

/// Validate hardware passthrough configuration
fn validate_hostpci_config(hostpci: &super::HostPciConfig) -> Result<()> {
    // Validate PCI device address format (basic check)
    if !is_valid_pci_address(&hostpci.device) {
        return Err(anyhow!("Invalid PCI device address format: {}", hostpci.device));
    }
    
    // Check if ROM file exists if specified
    if let Some(romfile) = &hostpci.romfile {
        if !std::path::Path::new(romfile).exists() {
            eprintln!("Warning: ROM file '{}' does not exist", romfile);
        }
    }
    
    Ok(())
}

/// Validate USB device configuration
fn validate_usb_device_config(usb_device: &super::UsbDeviceConfig) -> Result<()> {
    // Validate USB host specification format
    // Should be in format "bus.port" or "vendor:product"
    if !is_valid_usb_spec(&usb_device.host) {
        return Err(anyhow!("Invalid USB device specification: {}. Expected format: 'bus.port' or 'vendor:product'", usb_device.host));
    }
    
    Ok(())
}

/// Validate SPICE configuration
fn validate_spice_config(spice: &super::SpiceConfig) -> Result<()> {
    // Validate port range
    if spice.port == 0 {
        return Err(anyhow!("Invalid SPICE port: {}. Must be between 1 and 65535", spice.port));
    }
    
    // Validate IP address format (basic check)
    if spice.addr.is_empty() {
        return Err(anyhow!("SPICE address cannot be empty"));
    }
    
    Ok(())
}

/// Validate audio device configuration
fn validate_audio_devices(audio_devices: &[super::AudioDeviceConfig], spice: Option<&super::SpiceConfig>) -> Result<()> {
    if audio_devices.is_empty() {
        if let Some(spice) = spice {
            if spice.enabled && spice.audio {
                return Err(anyhow!("SPICE audio requires at least one configured audio device"));
            }
        }
        return Ok(());
    }

    match spice {
        Some(spice) if spice.enabled && spice.audio => {}
        Some(_) => {
            return Err(anyhow!("Audio devices currently require spice.enabled=true and spice.audio=true"));
        }
        None => {
            return Err(anyhow!("Audio devices currently require a SPICE configuration with audio enabled"));
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
                    return Err(anyhow!("Audio controller '{}' cannot define cad", audio_device.id));
                }

                if audio_device.audiodev.is_some() {
                    return Err(anyhow!("Audio controller '{}' cannot define audiodev", audio_device.id));
                }
            }
            "hda-micro" | "hda-duplex" => {
                if audio_device.id.trim().is_empty() {
                    return Err(anyhow!("Audio codec ID cannot be empty"));
                }

                if audio_device.bus.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(anyhow!("Audio codec '{}' requires a bus assignment", audio_device.id));
                }

                if audio_device.audiodev.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(anyhow!("Audio codec '{}' requires an audiodev backend ID", audio_device.id));
                }

                if audio_device.cad.is_none() {
                    return Err(anyhow!("Audio codec '{}' requires a cad value", audio_device.id));
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
        return Err(anyhow!("Audio device configuration requires an ich9-intel-hda controller"));
    }

    Ok(())
}

/// Validate input device configuration
fn validate_input_devices(input_devices: &[super::InputDeviceConfig]) -> Result<()> {
    let valid_types = ["virtio-mouse", "virtio-keyboard"];
    let mut seen_types = std::collections::HashSet::new();

    for input_device in input_devices {
        if !valid_types.contains(&input_device.r#type.as_str()) {
            return Err(anyhow!(
                "Unsupported input device type: {}. Supported: {:?}",
                input_device.r#type,
                valid_types
            ));
        }

        if !seen_types.insert(input_device.r#type.as_str()) {
            return Err(anyhow!("Duplicate input device type configured: {}", input_device.r#type));
        }
    }

    Ok(())
}

/// Validate ivshmem configuration
fn validate_ivshmem_config(ivshmem: &super::IvshmemConfig) -> Result<()> {
    // Validate shared memory size (reasonable bounds)
    if ivshmem.size == 0 {
        return Err(anyhow!("ivshmem size must be greater than 0"));
    }
    if ivshmem.size > 1024 { // 1 GiB
        return Err(anyhow!("ivshmem size cannot exceed 1024 MiB"));
    }
    
    // Validate vectors (reasonable bounds)  run_dir: "/var/run/ezkvm"

    if ivshmem.vectors == 0 || ivshmem.vectors > 32 {
        return Err(anyhow!("ivshmem vectors must be between 1 and 32"));
    }
    
    Ok(())
}

/// Validate SCSI controller configuration
fn validate_scsi_controller_config(scsi_controller: &super::ScsiControllerConfig) -> Result<()> {
    // Validate controller type
    let valid_types = ["virtio-scsi-pci", "pvscsi", "lsi", "lsi53c895a", "megasas", "megasas-gen2"];
    if !valid_types.contains(&scsi_controller.r#type.as_str()) {
        return Err(anyhow!("Unsupported SCSI controller type: {}. Supported: {:?}", 
                          scsi_controller.r#type, valid_types));
    }
    
    // Validate max_targets if specified
    if let Some(max_targets) = scsi_controller.max_targets {
        if max_targets == 0 || max_targets > 256 {
            return Err(anyhow!("max_targets must be between 1 and 256"));
        }
    }
    
    Ok(())
}

/// Validate iSCSI disk configuration
fn validate_iscsi_disk_config(iscsi_disk: &super::IscsiDiskConfig) -> Result<()> {
    // Validate portal format (host:port)
    if !iscsi_disk.portal.contains(':') {
        return Err(anyhow!("iSCSI portal must be in format 'host:port'"));
    }
    
    // Validate target IQN format (basic check)
    if !iscsi_disk.target.starts_with("iqn.") {
        return Err(anyhow!("iSCSI target must be a valid IQN starting with 'iqn.'"));
    }
    
    // Validate LUN
    if iscsi_disk.lun > 255 {
        return Err(anyhow!("iSCSI LUN cannot exceed 255"));
    }
    
    // Validate initiator IQN if provided
    if let Some(initiator) = &iscsi_disk.initiator {
        if !initiator.starts_with("iqn.") {
            return Err(anyhow!("iSCSI initiator must be a valid IQN starting with 'iqn.'"));
        }
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
            return Err(anyhow!("iSCSI authentication requires both username and password"));
        }
        (None, None) => {}
    }
    
    Ok(())
}

/// Validate QMP configuration
fn validate_qmp_config(qmp: &super::QmpConfig) -> Result<()> {
    // Validate socket path if provided
    if let Some(socket_path) = &qmp.socket_path {
        if socket_path.is_empty() {
            return Err(anyhow!("QMP socket path cannot be empty"));
        }
        // Check if it's an absolute path
        if !socket_path.starts_with('/') {
            return Err(anyhow!("QMP socket path must be an absolute path"));
        }
    }
    
    Ok(())
}

/// Validate SMBIOS configuration
fn validate_smbios_config(smbios: &super::SmbiosConfig) -> Result<()> {
    // Validate UUID format if provided
    if let Some(uuid) = &smbios.uuid {
        if uuid.len() != 36 || !uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(anyhow!("SMBIOS UUID must be in format XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"));
        }
    }
    
    // Validate VM generation ID format if provided
    if let Some(vm_gen_id) = &smbios.vm_generation_id {
        if vm_gen_id.len() != 36 || !vm_gen_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(anyhow!("VM generation ID must be in format XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"));
        }
    }
    
    Ok(())
}

/// Validate NUMA configuration
fn validate_numa_config(numa: &super::NumaConfig) -> Result<()> {
    // Validate memory size
    if numa.memory == 0 {
        return Err(anyhow!("NUMA node memory must be greater than 0"));
    }
    
    // Validate CPU list is not empty
    if numa.cpus.is_empty() {
        return Err(anyhow!("NUMA node must have at least one CPU"));
    }
    
    // Validate CPU IDs are reasonable
    for cpu in &numa.cpus {
        if *cpu > 1023 {
            return Err(anyhow!("CPU ID {} exceeds maximum of 1023", cpu));
        }
    }
    
    // Check for duplicate CPU IDs in this node
    let mut seen_cpus = std::collections::HashSet::new();
    for cpu in &numa.cpus {
        if !seen_cpus.insert(cpu) {
            return Err(anyhow!("Duplicate CPU ID {} in NUMA node {}", cpu, numa.id));
        }
    }
    
    Ok(())
}

/// Validate Hyper-V configuration
fn validate_hyperv_config(hyperv: &super::HypervConfig) -> Result<()> {
    // Validate vendor_id if provided
    if let Some(vendor_id) = &hyperv.vendor_id {
        if vendor_id.len() > 12 {
            return Err(anyhow!("Hyper-V vendor_id cannot exceed 12 characters"));
        }
    }
    
    // Validate spinlock_retry if provided
    if let Some(retry_count) = hyperv.spinlock_retry {
        if retry_count == 0 {
            return Err(anyhow!("Hyper-V spinlock_retry must be between 1 and 4294967295"));
        }
    }
    
    Ok(())
}

/// Check if a string is a valid PCI device address
fn is_valid_pci_address(addr: &str) -> bool {
    // Basic PCI address validation: XXXX:XX:XX.X format
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 3 {
        return false;
    }
    
    // Check domain:bus:slot.function format
    let bus_slot_func: Vec<&str> = parts[2].split('.').collect();
    if bus_slot_func.len() != 2 {
        return false;
    }
    
    // Domain should be 4 chars, bus should be 2 chars
    if parts[0].len() != 4 || parts[1].len() != 2 {
        return false;
    }
    
    // Bus/slot should be 2 chars, function should be 1 char
    if bus_slot_func[0].len() != 2 || bus_slot_func[1].len() != 1 {
        return false;
    }
    
    // All parts should be valid hex
    u16::from_str_radix(parts[0], 16).is_ok() &&
    u8::from_str_radix(parts[1], 16).is_ok() &&
    u8::from_str_radix(bus_slot_func[0], 16).is_ok() && 
    u8::from_str_radix(bus_slot_func[1], 16).is_ok()
}

/// Check if a string is a valid USB device specification
fn is_valid_usb_spec(spec: &str) -> bool {
    // Check for bus.port format (e.g., "1-2.3")
    if spec.contains('-') && spec.contains('.') {
        // Basic validation - could be more strict
        return true;
    }
    
    // Check for vendor:product format (e.g., "1234:5678")
    if spec.contains(':') {
        let parts: Vec<&str> = spec.split(':').collect();
        if parts.len() == 2 {
            return u16::from_str_radix(parts[0], 16).is_ok() && 
                   u16::from_str_radix(parts[1], 16).is_ok();
        }
    }
    
    false
}