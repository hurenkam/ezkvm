use super::helpers::is_q35_machine;
use super::{InputDeviceConfig, ProxmoxVmConfig, VmConfig};

pub(super) fn infer_profile_names(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> Vec<String> {
    let mut profiles = Vec::new();
    let ostype = proxmox.scalars.get("ostype").map(String::as_str);

    // Always assign the Proxmox base profile to provide guest-semantic defaults
    // (boot menu, kvm-pit, drive tuning, tap network device model, queue sizes, placement)
    // that are not stored in the Proxmox .conf file but are applied by Proxmox at launch time.
    profiles.push("proxmox-base".to_string());

    // Always assign the parity-runtime profile to provide Proxmox host-specific runtime
    // path literals (boot splash asset, pve-bridge helper scripts) that are required for
    // strict Proxmox parity. This profile is assigned unconditionally here because explicit
    // runtime target selection (B-40) is not yet implemented. Once B-40 lands, this profile
    // will only be assigned for proxmox-parity target imports.
    profiles.push("proxmox-parity-runtime".to_string());

    if config.system.architecture == "x86_64"
        && config.system.boot.firmware.as_deref() == Some("uefi")
        && is_q35_machine(&config.system.machine)
    {
        profiles.push("proxmox-q35-uefi".to_string());
    }

    if let Some(controller_type) = proxmox.scalars.get("scsihw").map(String::as_str) {
        match controller_type {
            "virtio-scsi-single" => profiles.push("storage-virtio-scsi-single".to_string()),
            "virtio-scsi-pci" => profiles.push("storage-virtio-scsi-pci".to_string()),
            _ => {}
        }
    }

    match ostype {
        Some("win11") => {
            // proxmox-windows provides the HV CPU flags Proxmox adds for Windows guests
            profiles.push("proxmox-windows".to_string());
            if config.options.rtc.as_ref().is_some_and(|rtc| {
                rtc.base.as_deref() == Some("localtime") && rtc.driftfix.as_deref() == Some("slew")
            }) {
                profiles.push("windows-common".to_string());
            }
            if config.system.boot.secure_boot && config.system.tpm.is_some() {
                profiles.push("windows-11".to_string());
            }
        }
        Some("win10") => {
            // proxmox-windows provides the HV CPU flags Proxmox adds for Windows guests
            profiles.push("proxmox-windows".to_string());
            if config.options.rtc.as_ref().is_some_and(|rtc| {
                rtc.base.as_deref() == Some("localtime") && rtc.driftfix.as_deref() == Some("slew")
            }) {
                profiles.push("windows-common".to_string());
            }
            profiles.push("windows-10".to_string());
        }
        Some("l26") => {
            if config.options.guest_agent.is_some() {
                profiles.push("linux-l26-common".to_string());
            }
        }
        Some("other") if is_macos_guest(proxmox, config) => {
            profiles.push("macos-kvm".to_string());
        }
        _ => {}
    }

    if config.system.memory.ivshmem.is_some() {
        profiles.push("looking-glass".to_string());
    } else if config.spice.is_some() && has_remote_viewer_input_devices(&config.devices.input) {
        profiles.push("remote-viewer-spice".to_string());
    }

    if !config.host.pci.is_empty() {
        profiles.push("gpu-passthrough".to_string());
    }

    if config.system.memory.hugepages.is_some() {
        profiles.push("hugepages".to_string());
    }

    if config.iommu.is_some() {
        profiles.push("viommu".to_string());
    }

    if has_hidden_hypervisor_signals(proxmox, config) {
        profiles.push("hidden-hypervisor".to_string());
    }

    let has_headless_display = config
        .devices
        .displays
        .iter()
        .all(|display| display.r#type == "none");

    if config.vnc.as_ref().is_some_and(|vnc| vnc.enabled) && has_headless_display {
        profiles.push("headless-vnc".to_string());
    }

    if proxmox.scalars.contains_key("serial0")
        && config.vnc.is_none()
        && config.spice.is_none()
        && has_headless_display
    {
        profiles.push("headless-serial".to_string());
    }

    profiles.dedup();
    profiles
}

fn has_hidden_hypervisor_signals(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> bool {
    config
        .system
        .cpu
        .features
        .iter()
        .any(|feature| matches!(feature.as_str(), "kvm=off" | "-hypervisor" | "hidden=1"))
        || proxmox.scalars.get("args").is_some_and(|args| {
            args.contains("kvm=off") || args.contains("-hypervisor") || args.contains("hidden=1")
        })
}

fn has_remote_viewer_input_devices(input_devices: &[InputDeviceConfig]) -> bool {
    let has_mouse = input_devices
        .iter()
        .any(|device| device.r#type == "virtio-mouse");
    let has_keyboard = input_devices
        .iter()
        .any(|device| device.r#type == "virtio-keyboard");
    has_mouse && has_keyboard
}

fn is_macos_guest(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> bool {
    let has_applesmc = proxmox
        .scalars
        .get("args")
        .is_some_and(|args| args.contains("isa-applesmc"));
    let has_macos_display = config
        .devices
        .displays
        .iter()
        .any(|display| display.r#type == "none");
    has_applesmc && has_macos_display && is_q35_machine(&config.system.machine)
}
