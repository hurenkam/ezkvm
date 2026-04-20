use super::helpers::is_q35_machine;
use super::{InputDeviceConfig, ProxmoxVmConfig, VmConfig};
use crate::import::proxmox::RuntimeTarget;

pub(super) fn infer_profile_names(
    proxmox: &ProxmoxVmConfig,
    config: &VmConfig,
    runtime_target: RuntimeTarget,
) -> Vec<String> {
    let mut profiles = Vec::new();
    let ostype = proxmox.scalars.get("ostype").map(String::as_str);

    // Always assign the Proxmox base profile to provide guest-semantic defaults
    // (boot menu, kvm-pit, drive tuning, tap network device model, queue sizes, placement)
    // that are not stored in the Proxmox .conf file but are applied by Proxmox at launch time.
    profiles.push("proxmox-base".to_string());

    // Conditionally assign parity-runtime profile based on target
    if runtime_target == RuntimeTarget::ProxmoxParity {
        // For strict Proxmox parity, include host-specific runtime path literals
        // (boot splash asset, pve-bridge helper scripts) that enable byte-close
        // parity with Proxmox QEMU commands.
        profiles.push("proxmox-parity-runtime".to_string());
    }
    // For portable-linux target, parity-runtime is omitted; host-specific paths
    // will be resolved at runtime via B-41 capability resolution.

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

    let has_mixed_storage_buses = proxmox
        .disks
        .iter()
        .map(|disk| disk.bus.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        > 1;

    if has_mixed_storage_buses {
        infer_storage_profile_from_controller_type(config, &mut profiles);
    }

    let has_hyperv_variant = has_hyperv_variant_signals(proxmox, config);

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
            if config.options.guest_agent.is_some()
                || has_nested_virtualization_signals(proxmox, config)
            {
                profiles.push("linux-l26-common".to_string());
            }
        }
        Some("other") if is_macos_guest(proxmox, config) => {
            profiles.push("macos-kvm".to_string());
        }
        _ => {}
    }

    // Some Proxmox exports provide Hyper-V CPU variants without a strict win10/win11 ostype.
    // Preserve these by applying Windows tuning profiles when guest signals are explicit.
    if has_hyperv_variant {
        profiles.push("proxmox-windows".to_string());

        if config.system.boot.secure_boot && config.system.tpm.is_some() {
            profiles.push("windows-11".to_string());
        }
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
    config.system.cpu.features.iter().any(|feature| {
        matches!(
            feature.as_str(),
            "kvm=off" | "-hypervisor" | "hidden=1" | "vmport=off"
        ) || feature.starts_with("hv_vendor_id=")
    }) || proxmox.scalars.get("args").is_some_and(|args| {
        args.contains("kvm=off")
            || args.contains("-hypervisor")
            || args.contains("hidden=1")
            || args.contains("vmport=off")
            || args.contains("hv_vendor_id=")
    })
}

fn has_hyperv_variant_signals(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> bool {
    let has_cpu_signals = config.system.cpu.features.iter().any(|feature| {
        feature.starts_with("hv_")
            || feature.starts_with("+hyperv-")
            || feature.starts_with("hyperv-")
            || feature.starts_with("hv-")
    });

    has_cpu_signals
        || proxmox.scalars.get("args").is_some_and(|args| {
            args.contains("hyperv-")
                || args.contains("hv_relaxed")
                || args.contains("hv_synic")
                || args.contains("hv_time")
        })
}

fn has_nested_virtualization_signals(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> bool {
    let has_cpu_nested_flags = config.system.cpu.features.iter().any(|feature| {
        matches!(
            feature.as_str(),
            "+vmx" | "vmx=on" | "+svm" | "svm=on" | "+hyperv-enlightened-vmx"
        )
    });

    has_cpu_nested_flags
        || proxmox.scalars.get("args").is_some_and(|args| {
            args.contains("+vmx")
                || args.contains("vmx=on")
                || args.contains("+svm")
                || args.contains("svm=on")
                || args.contains("hyperv-enlightened-vmx")
        })
}

fn infer_storage_profile_from_controller_type(config: &VmConfig, profiles: &mut Vec<String>) {
    if let Some(first_scsi) = config.controllers.scsi.first() {
        match first_scsi.r#type.as_str() {
            "virtio-scsi-single" => profiles.push("storage-virtio-scsi-single".to_string()),
            "virtio-scsi-pci" => profiles.push("storage-virtio-scsi-pci".to_string()),
            _ => {}
        }
    }
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
    let has_applesmc = config.system.applesmc.is_some()
        || proxmox
            .scalars
            .get("args")
            .is_some_and(|args| args.contains("isa-applesmc"));
    let has_smbios_type2 = config
        .system
        .smbios
        .as_ref()
        .is_some_and(|smbios| smbios.smbios_type == 2)
        || proxmox
            .scalars
            .get("args")
            .is_some_and(|args| args.contains("-smbios type=2"));
    let has_macos_display = config
        .devices
        .displays
        .iter()
        .any(|display| display.r#type == "none");
    has_applesmc && has_smbios_type2 && has_macos_display && is_q35_machine(&config.system.machine)
}
