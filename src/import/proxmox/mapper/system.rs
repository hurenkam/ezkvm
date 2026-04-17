// Temporary over-size rationale (B-29): system-path mapping still combines several
// ownership areas (cpu/memory/boot/firmware). Closure target is <=250 lines by
// splitting memory and boot helpers while keeping behavior stable.
use super::super::model::{ProxmoxStorageConfig, ProxmoxVmConfig};
use super::helpers::{
    is_enabled, parse_human_size_to_bytes, parse_options, parse_source_and_options,
};
use super::storage::resolve_volume_reference;
use super::{
    BallooningConfig, BootConfig, GuestAgentConfig, HugepagesConfig, IommuConfig, MappingWarning,
    NumaConfig, TpmConfig,
};
use std::collections::BTreeMap;

pub(super) fn map_architecture(
    arch: Option<&String>,
    warnings: &mut Vec<MappingWarning>,
) -> String {
    match arch.map(String::as_str).unwrap_or("x86_64") {
        "amd64" => "x86_64".to_string(),
        "x86_64" | "aarch64" | "x86" | "ppc64" | "riscv64" => {
            arch.cloned().unwrap_or_else(|| "x86_64".to_string())
        }
        other => {
            warnings.push(MappingWarning {
                source_field: "arch".to_string(),
                message: format!(
                    "unsupported Proxmox architecture '{}' mapped to 'x86_64'",
                    other
                ),
            });
            "x86_64".to_string()
        }
    }
}

pub(super) fn apply_proxmox_q35_compat_if_needed(
    proxmox: &ProxmoxVmConfig,
    machine: &mut String,
    readconfig: &mut Vec<String>,
) {
    const PVE_Q35_READCONFIG: &str = "/usr/share/qemu-server/pve-q35-4.0.cfg";

    if !super::helpers::is_q35_machine(machine) {
        return;
    }

    let has_pve_machine_hint = proxmox
        .scalars
        .get("machine")
        .is_some_and(|value| value.contains("+pve"));

    let has_topology_bus_hints = !proxmox.host_pci.is_empty()
        || proxmox.host_pci.iter().any(|entry| {
            entry.options.get("bus").is_some_and(|bus| {
                bus.starts_with("pci.")
                    || bus.starts_with("pcie.")
                    || bus.starts_with("ich9-pcie-port")
            })
        })
        || proxmox.scalars.get("args").is_some_and(|args| {
            args.contains("bus=pci.")
                || args.contains("bus=pcie.")
                || args.contains("ich9-pcie-port")
        });

    if !has_pve_machine_hint && !has_topology_bus_hints {
        return;
    }

    if !machine.contains("+pve") {
        if machine == "q35" {
            *machine = "pc-q35-8.1+pve0".to_string();
        } else {
            *machine = format!("{}+pve0", machine);
        }
    }

    if !readconfig.iter().any(|path| path == PVE_Q35_READCONFIG) {
        readconfig.push(PVE_Q35_READCONFIG.to_string());
    }
}

pub(super) fn map_vcpus(scalars: &BTreeMap<String, String>) -> u32 {
    if let Some(vcpus) = scalars.get("vcpus").and_then(|v| v.parse::<u32>().ok()) {
        return vcpus.max(1);
    }

    let cores = scalars
        .get("cores")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let sockets = scalars
        .get("sockets")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let threads = scalars
        .get("threads")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);

    cores.saturating_mul(sockets).saturating_mul(threads).max(1)
}

pub(super) fn map_hugepages(scalars: &BTreeMap<String, String>) -> Option<HugepagesConfig> {
    let raw = scalars.get("hugepages")?;
    let raw = raw.trim();

    if raw == "0" || raw == "any" || raw.is_empty() {
        return None;
    }

    let size_mib: u64 = raw.parse().ok()?;
    let size_kib = size_mib * 1024;

    Some(HugepagesConfig {
        enabled: true,
        size_kib: Some(size_kib),
        mem_path: None,
        prealloc: true,
    })
}

pub(super) fn synthesize_hugepages_numa(memory_mib: u32, vcpus: u32) -> Vec<NumaConfig> {
    let cpus: Vec<u32> = (0..vcpus).collect();
    vec![NumaConfig {
        id: 0,
        memory: memory_mib,
        cpus,
        host_node: None,
    }]
}

pub(super) fn map_iommu(
    machine_options: &[String],
    raw_args: Option<&String>,
) -> Option<IommuConfig> {
    for option in machine_options {
        if let Some(iommu_type) = option.strip_prefix("viommu=") {
            return Some(IommuConfig {
                r#type: iommu_type.to_string(),
                ..Default::default()
            });
        }
    }

    if let Some(args) = raw_args {
        if args.contains("intel-iommu") {
            let intremap = args.contains("intremap=on");
            let caching_mode = args.contains("caching-mode=on");
            let eim = args.contains("eim=on");
            return Some(IommuConfig {
                r#type: "intel".to_string(),
                id: "iommu0".to_string(),
                intremap,
                caching_mode,
                eim,
            });
        }
        if args.contains("amd-iommu") {
            return Some(IommuConfig {
                r#type: "amd".to_string(),
                id: "iommu0".to_string(),
                intremap: false,
                caching_mode: false,
                eim: false,
            });
        }
    }

    None
}

pub(super) fn map_tpm(
    scalars: &BTreeMap<String, String>,
    storage_config: Option<&ProxmoxStorageConfig>,
    vmid: Option<u32>,
) -> Option<TpmConfig> {
    let (key, raw) = scalars
        .iter()
        .find(|(k, _)| k.starts_with("tpmstate"))
        .map(|(k, v)| (k.as_str(), v.as_str()))?;

    let (source, _) = parse_source_and_options(raw);
    let mut version = "2.0".to_string();
    for token in raw.split(',') {
        let token = token.trim();
        if let Some(v) = token.strip_prefix("version=") {
            version = v.trim_start_matches('v').to_string();
        }
    }

    let state_backend_uri = if source.is_empty() || source == "none" {
        None
    } else {
        let resolved =
            resolve_volume_reference(source, storage_config).unwrap_or_else(|| source.to_string());
        if resolved.starts_with('/') {
            Some(resolved)
        } else {
            None
        }
    };

    let model = "tpm-tis".to_string();

    Some(TpmConfig {
        version,
        backend: "emulator".to_string(),
        state_path: Some(match vmid {
            Some(id) => format!("/var/run/qemu-server/{}.swtpm", id),
            None => format!("/var/run/ezkvm/{}-tpm.socket", key),
        }),
        state_dir: None,
        state_backend_uri,
        model,
    })
}

pub(super) fn apply_efidisk0_to_boot(
    scalars: &BTreeMap<String, String>,
    storage_config: Option<&ProxmoxStorageConfig>,
    boot: &mut BootConfig,
    warnings: &mut Vec<MappingWarning>,
) {
    let Some(raw) = scalars.get("efidisk0") else {
        return;
    };

    if boot.firmware.as_deref() == Some("bios") {
        warnings.push(MappingWarning {
            source_field: "efidisk0".to_string(),
            message: "efidisk0 requires UEFI firmware; overriding BIOS firmware mapping to 'uefi'"
                .to_string(),
        });
    }

    if boot.firmware.is_none() || boot.firmware.as_deref() == Some("bios") {
        boot.firmware = Some("uefi".to_string());
    }

    let (source, options) = parse_source_and_options(raw);
    if !source.is_empty() && source != "none" {
        boot.uefi_vars = Some(
            resolve_volume_reference(source, storage_config).unwrap_or_else(|| source.to_string()),
        );
    }

    boot.uefi_vars_size = map_efidisk_vars_size(&options);

    let mut secure_boot_signals = Vec::new();

    if let Some(ms_cert) = options.get("ms-cert") {
        boot.uefi_ms_cert = Some(ms_cert.clone());
        match parse_ms_cert_signal(ms_cert) {
            Some(enabled) => secure_boot_signals.push(enabled),
            None => warnings.push(MappingWarning {
                source_field: "efidisk0".to_string(),
                message: format!(
                    "ms-cert='{}' metadata was preserved, but the exact certificate mode is not representable; secure boot was left unchanged",
                    ms_cert
                ),
            }),
        }
    }

    if let Some(pek) = options.get("pre-enrolled-keys") {
        boot.uefi_pre_enrolled_keys = Some(pek.clone());
        match parse_boolish_signal(pek) {
            Some(enabled) => secure_boot_signals.push(enabled),
            None => warnings.push(MappingWarning {
                source_field: "efidisk0".to_string(),
                message: format!(
                    "pre-enrolled-keys='{}' metadata was preserved, but the value is not representable; secure boot was left unchanged",
                    pek
                ),
            }),
        }
    }

    if secure_boot_signals.into_iter().any(|enabled| enabled) {
        boot.secure_boot = true;
    }
}

fn parse_ms_cert_signal(value: &str) -> Option<bool> {
    if let Some(enabled) = parse_boolish_signal(value) {
        return Some(enabled);
    }

    value.trim().parse::<u32>().ok().map(|numeric| numeric > 0)
}

fn parse_boolish_signal(value: &str) -> Option<bool> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "1" | "on" | "true" | "yes" => Some(true),
        "0" | "off" | "false" | "no" | "none" => Some(false),
        _ => None,
    }
}

fn map_efidisk_vars_size(options: &BTreeMap<String, String>) -> Option<u64> {
    if let Some(efitype) = options.get("efitype") {
        match efitype.trim().to_ascii_lowercase().as_str() {
            "4m" => return Some(540_672),
            "2m" => return Some(131_072),
            _ => {}
        }
    }

    options
        .get("size")
        .and_then(|value| parse_human_size_to_bytes(value))
}

pub(super) fn map_guest_agent(
    scalars: &BTreeMap<String, String>,
    vmid: Option<u32>,
) -> Option<GuestAgentConfig> {
    let raw = scalars.get("agent")?.trim();
    if raw.is_empty() {
        return None;
    }

    let options = parse_options(raw);
    let enabled = if options.is_empty() {
        matches!(raw, "1" | "on" | "yes" | "true")
    } else {
        options
            .get("enabled")
            .is_none_or(|value| is_enabled(Some(value)))
    };

    if !enabled {
        return None;
    }

    let socket_path = options
        .get("path")
        .or_else(|| options.get("socket"))
        .cloned()
        .or_else(|| vmid.map(|id| format!("/var/run/qemu-server/{}.qga", id)))
        .or_else(|| Some("/var/run/qemu-server/qga.sock".to_string()));

    Some(GuestAgentConfig {
        enabled: true,
        socket_path,
        freeze_cpu: false,
        bus: Some("pci.0".to_string()),
        addr: Some("0x8".to_string()),
    })
}

pub(super) fn parse_boot_order(scalars: &BTreeMap<String, String>) -> BTreeMap<String, u32> {
    let mut boot_indices = BTreeMap::new();
    if let Some(boot_str) = scalars.get("boot")
        && let Some(order) = boot_str.strip_prefix("order=")
    {
        for (index, device_key) in order.split(';').enumerate() {
            let dev_key = device_key.trim();
            if !dev_key.is_empty() {
                boot_indices.insert(dev_key.to_string(), 100 + index as u32);
            }
        }
    }
    boot_indices
}

pub(super) fn parse_smbios_uuid(scalars: &BTreeMap<String, String>) -> Option<String> {
    scalars.get("smbios1").and_then(|smbios_str| {
        for token in smbios_str.split(',') {
            if let Some(uuid) = token.strip_prefix("uuid=") {
                return Some(uuid.trim().to_string());
            }
        }
        None
    })
}

pub(super) fn map_ballooning(is_windows: bool) -> Option<BallooningConfig> {
    Some(BallooningConfig {
        enabled: true,
        free_page_reporting: is_windows,
        model: "virtio-balloon-pci".to_string(),
        id: if is_windows {
            Some("balloon0".to_string())
        } else {
            None
        },
        bus: if is_windows {
            Some("pci.0".to_string())
        } else {
            None
        },
        addr: if is_windows {
            Some("0x3".to_string())
        } else {
            None
        },
    })
}
