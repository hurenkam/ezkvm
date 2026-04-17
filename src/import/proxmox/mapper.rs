use super::error::ImportError;
use super::model::{
    ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxNetEntry, ProxmoxStorageConfig, ProxmoxUsbEntry,
    ProxmoxVmConfig,
};
use crate::config::{
    AppleSmcConfig, AudioDeviceConfig, BallooningConfig, BootConfig, ControllersConfig,
    DeviceConfig, DisplayConfig, DriveConfig, GuestAgentConfig, HostConfig, HostPciConfig,
    HugepagesConfig, InputDeviceConfig, IommuConfig, IvshmemConfig, MemoryConfig,
    NetworkBackendConfig, NetworkConfig, NumaConfig, RtcConfig, SataControllerConfig,
    ScsiControllerConfig, SerialConfig, SmbiosConfig, SpiceConfig, SystemConfig, TpmConfig,
    UsbDeviceConfig, VmConfig, VmOptions, VncConfig, XhciControllerConfig,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingWarning {
    pub source_field: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalMappingResult {
    pub yaml: String,
    pub warnings: Vec<MappingWarning>,
}

pub fn map_proxmox_to_canonical_yaml(
    proxmox: &ProxmoxVmConfig,
) -> Result<CanonicalMappingResult, ImportError> {
    map_proxmox_to_canonical_yaml_with_storage(proxmox, None)
}

pub fn map_proxmox_to_canonical_yaml_with_storage(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> Result<CanonicalMappingResult, ImportError> {
    let mut warnings = Vec::new();

    let name = proxmox
        .scalars
        .get("name")
        .cloned()
        .unwrap_or_else(|| "imported-vm".to_string());
    let architecture = map_architecture(proxmox.scalars.get("arch"), &mut warnings);
    let (mut machine, machine_options) = parse_machine_and_options(proxmox.scalars.get("machine"));
    let mut readconfig = Vec::new();
    apply_proxmox_q35_compat_if_needed(proxmox, &mut machine, &mut readconfig);
    let iommu = map_iommu(&machine_options, proxmox.scalars.get("args"));

    let memory = proxmox
        .scalars
        .get("memory")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2048);
    let hugepages = map_hugepages(&proxmox.scalars);
    let vcpus = map_vcpus(&proxmox.scalars);
    let (cpu_model, cpu_features) = parse_cpu_model_and_features(proxmox.scalars.get("cpu"));

    let firmware = proxmox
        .scalars
        .get("bios")
        .and_then(|bios| match bios.as_str() {
            "ovmf" => Some("uefi".to_string()),
            "seabios" => Some("bios".to_string()),
            _ => None,
        });

    // Determine Windows status for RTC and ballooning
    let ostype = proxmox.scalars.get("ostype").map(String::as_str);
    let is_windows = ostype.is_some_and(|ot| ot.starts_with("win"));

    let mut boot = BootConfig {
        firmware,
        ..Default::default()
    };
    apply_efidisk0_to_boot(&proxmox.scalars, storage_config, &mut boot, &mut warnings);

    // Parse boot order and SMBIOS early
    let boot_indices = parse_boot_order(&proxmox.scalars);
    let smbios_uuid = parse_smbios_uuid(&proxmox.scalars);

    let scsi_controllers = map_scsi_controllers(&proxmox.scalars, &proxmox.disks, &mut warnings);
    let sata_controllers = map_sata_controllers(&proxmox.disks);
    let inferred_vmid = infer_proxmox_vmid(proxmox);
    let mut drives = proxmox
        .disks
        .iter()
        .map(|disk| map_drive(disk, storage_config))
        .collect::<Vec<_>>();
    let mut networks = proxmox
        .networks
        .iter()
        .map(|network| map_network(network, inferred_vmid, is_windows, &mut warnings))
        .collect::<Vec<_>>();

    // Apply boot indices from parsed boot order
    for drive in &mut drives {
        if let Some(idx) = boot_indices.get(&drive.id) {
            drive.boot_index = Some(*idx);
        }
    }
    for network in &mut networks {
        if let Some(idx) = boot_indices.get(&network.id) {
            network.boot_index = Some(*idx);
        }
    }
    let displays = map_displays(&proxmox.scalars, &mut warnings);
    let serials = map_serials(&proxmox.scalars, inferred_vmid, &mut warnings);
    let (audio, mut spice) = map_audio_and_spice(&proxmox.scalars, &mut warnings);
    let mut vnc = None;
    let mut input_devices = Vec::new();
    let mut ivshmem = None;
    let mut applesmc = None;
    let mut smbios_type = 1u8;
    apply_args_passthrough_subset(
        &proxmox.scalars,
        &mut ArgsPassthroughOutput {
            spice: &mut spice,
            vnc: &mut vnc,
            input_devices: &mut input_devices,
            ivshmem: &mut ivshmem,
            applesmc: &mut applesmc,
            smbios_type: &mut smbios_type,
        },
        &mut warnings,
    );
    let explicit_host_functions = proxmox
        .host_pci
        .iter()
        .filter(|entry| {
            entry
                .host
                .rsplit_once(':')
                .is_some_and(|(_, slot)| slot.contains('.'))
        })
        .map(|entry| normalize_host_pci_device(&entry.host))
        .collect::<BTreeSet<_>>();

    let host_pci = proxmox
        .host_pci
        .iter()
        .flat_map(|entry| map_host_pci_entries(entry, &explicit_host_functions))
        .collect::<Vec<_>>();
    let use_explicit_xhci = !proxmox.usb.is_empty();
    let host_usb = proxmox
        .usb
        .iter()
        .map(|entry| map_usb(entry, use_explicit_xhci))
        .collect::<Vec<_>>();

    let ballooning = Some(BallooningConfig {
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
    });

    let tpm = map_tpm(&proxmox.scalars, storage_config, inferred_vmid);
    let guest_agent = map_guest_agent(&proxmox.scalars, inferred_vmid);

    let vm_generation_id = proxmox.scalars.get("vmgenid").cloned();
    let smbios = if smbios_uuid.is_some() || vm_generation_id.is_some() || smbios_type != 1 {
        Some(SmbiosConfig {
            smbios_type,
            manufacturer: None,
            product: None,
            version: None,
            serial: None,
            uuid: smbios_uuid,
            sku: None,
            family: None,
            vm_generation_id,
        })
    } else {
        None
    };

    let mut vm_config = VmConfig {
        name,
        backend: "qemu".to_string(),
        profiles: Vec::new(),
        system: SystemConfig {
            architecture,
            machine,
            machine_options,
            memory: MemoryConfig {
                size: memory,
                ballooning,
                ivshmem,
                hugepages: hugepages.clone(),
            },
            cpu: crate::config::CpuConfig {
                model: cpu_model,
                vcpus,
                features: cpu_features,
                numa: if hugepages.is_some() {
                    synthesize_hugepages_numa(memory, vcpus)
                } else {
                    Vec::new()
                },
            },
            boot,
            tpm,
            smbios,
            applesmc,
            readconfig,
        },
        devices: DeviceConfig {
            drives,
            networks,
            displays,
            serials,
            input: input_devices,
            audio,
        },
        controllers: ControllersConfig {
            scsi: scsi_controllers,
            sata: sata_controllers,
            xhci: if use_explicit_xhci {
                vec![XhciControllerConfig {
                    id: "xhci".to_string(),
                    p2: Some(15),
                    p3: Some(15),
                    bus: Some("pci.1".to_string()),
                    addr: Some("0x1b".to_string()),
                }]
            } else {
                Vec::new()
            },
        },
        host: HostConfig {
            pci: host_pci,
            usb: host_usb,
        },
        spice,
        vnc,
        iscsi_disks: Vec::new(),
        hyperv: None,
        iommu,
        options: VmOptions {
            rtc: if is_windows {
                Some(RtcConfig {
                    base: Some("localtime".to_string()),
                    driftfix: Some("slew".to_string()),
                })
            } else {
                None
            },
            guest_agent,
            ..VmOptions::default()
        },
    };

    vm_config.profiles = infer_profile_names(proxmox, &vm_config);

    let yaml = serde_yaml::to_string(&vm_config)
        .map_err(|e| ImportError::ParseError(format!("failed to serialize mapped config: {e}")))?;

    Ok(CanonicalMappingResult { yaml, warnings })
}

fn map_architecture(arch: Option<&String>, warnings: &mut Vec<MappingWarning>) -> String {
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

fn parse_machine_and_options(machine: Option<&String>) -> (String, Vec<String>) {
    let raw = machine.map(String::as_str).unwrap_or("q35").trim();
    if raw.is_empty() {
        return ("q35".to_string(), Vec::new());
    }

    let mut parts = raw
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return ("q35".to_string(), Vec::new());
    }

    if let Some(machine_part) = parts.first()
        && let Some(machine_type) = machine_part.strip_prefix("type=")
    {
        let machine = machine_type.trim().to_string();
        parts.remove(0);
        return (machine, parts.iter().map(|p| p.to_string()).collect());
    }

    let machine = parts.remove(0).to_string();
    let options = parts.iter().map(|p| p.to_string()).collect();
    (machine, options)
}

fn parse_cpu_model_and_features(cpu: Option<&String>) -> (String, Vec<String>) {
    let raw = cpu.map(String::as_str).unwrap_or("host").trim();
    if raw.is_empty() {
        return ("host".to_string(), Vec::new());
    }

    let mut parts = raw
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return ("host".to_string(), Vec::new());
    }

    let model = parts.remove(0).to_string();
    let features = parts.iter().map(|p| p.to_string()).collect();
    (model, features)
}

fn apply_proxmox_q35_compat_if_needed(
    proxmox: &ProxmoxVmConfig,
    machine: &mut String,
    readconfig: &mut Vec<String>,
) {
    const PVE_Q35_READCONFIG: &str = "/usr/share/qemu-server/pve-q35-4.0.cfg";

    if !is_q35_machine(machine) {
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

fn is_q35_machine(machine: &str) -> bool {
    machine == "q35" || machine.contains("q35")
}

fn map_vcpus(scalars: &BTreeMap<String, String>) -> u32 {
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

fn map_scsi_controllers(
    scalars: &BTreeMap<String, String>,
    disks: &[ProxmoxDiskEntry],
    warnings: &mut Vec<MappingWarning>,
) -> Vec<ScsiControllerConfig> {
    let has_scsi_disks = disks.iter().any(|d| d.bus == "scsi");
    let scsihw = scalars.get("scsihw").map(String::as_str);

    if !has_scsi_disks && scsihw.is_none() {
        return Vec::new();
    }

    let controller_type = match scsihw.unwrap_or("virtio-scsi-single") {
        "virtio-scsi-single" | "virtio-scsi-pci" => "virtio-scsi-pci".to_string(),
        "pvscsi" => "pvscsi".to_string(),
        "lsi" => "lsi".to_string(),
        "lsi53c810" | "lsi53c895a" => "lsi53c895a".to_string(),
        "megasas" => "megasas".to_string(),
        "megasas-gen2" => "megasas-gen2".to_string(),
        unknown => {
            warnings.push(MappingWarning {
                source_field: "scsihw".to_string(),
                message: format!(
                    "unsupported scsihw '{}' mapped to 'virtio-scsi-pci'",
                    unknown
                ),
            });
            "virtio-scsi-pci".to_string()
        }
    };

    vec![ScsiControllerConfig {
        id: "scsihw0".to_string(),
        r#type: controller_type,
        iothread: None,
        max_targets: None,
        bus: None,
        addr: None,
    }]
}

fn map_drive(
    disk: &ProxmoxDiskEntry,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> DriveConfig {
    let is_cdrom = disk
        .options
        .get("media")
        .map(|v| v == "cdrom")
        .unwrap_or(false)
        || disk.source == "none";

    let path = if is_cdrom && disk.source == "none" {
        String::new()
    } else {
        resolve_volume_reference(&disk.source, storage_config)
            .unwrap_or_else(|| disk.source.clone())
    };

    let format = disk.options.get("format").cloned().unwrap_or_else(|| {
        if is_cdrom || path.starts_with("/dev/") {
            "raw".to_string()
        } else {
            "qcow2".to_string()
        }
    });

    let discard = is_enabled(disk.options.get("discard"));
    let ssd = is_enabled(disk.options.get("ssd"));
    let readonly = is_enabled(disk.options.get("readonly")) || is_enabled(disk.options.get("ro"));

    let boot_index = disk.options.get("boot").and_then(|v| v.parse::<u32>().ok());

    // Set I/O defaults for block devices (LVM, ZFS, etc.)
    let is_block_device = path.starts_with("/dev/");
    let cache = disk.options.get("cache").cloned().or_else(|| {
        if is_block_device {
            Some("none".to_string())
        } else {
            None
        }
    });
    let aio = disk.options.get("aio").cloned().or_else(|| {
        if is_block_device {
            Some("io_uring".to_string())
        } else {
            None
        }
    });
    let detect_zeroes = disk
        .options
        .get("detect_zeroes")
        .cloned()
        .or_else(|| disk.options.get("detect-zeroes").cloned())
        .or_else(|| {
            if is_block_device {
                Some("unmap".to_string())
            } else {
                None
            }
        });

    DriveConfig {
        id: disk.key.clone(),
        path,
        interface: disk.bus.clone(),
        r#type: if is_cdrom {
            "cdrom".to_string()
        } else {
            "disk".to_string()
        },
        format,
        readonly,
        discard,
        ssd,
        cache,
        aio,
        detect_zeroes,
        controller: if disk.bus == "scsi" {
            Some("scsihw0".to_string())
        } else {
            None
        },
        boot_index,
        scsi_id: if disk.bus == "scsi" {
            Some(disk.index as u32)
        } else {
            None
        },
        rotation_rate: if ssd && disk.bus == "scsi" {
            Some(1)
        } else {
            None
        },
        bus: if disk.bus == "sata" {
            Some(format!("sata0.{}", disk.index))
        } else {
            None
        },
        unit: if disk.bus == "sata" { Some(0) } else { None },
    }
}

fn map_sata_controllers(disks: &[ProxmoxDiskEntry]) -> Vec<SataControllerConfig> {
    let has_sata_disks = disks.iter().any(|d| d.bus == "sata");
    if !has_sata_disks {
        return Vec::new();
    }

    vec![SataControllerConfig {
        id: "sata0".to_string(),
        r#type: "ahci".to_string(),
        bus: None,
        addr: None,
    }]
}

fn resolve_volume_reference(
    source: &str,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> Option<String> {
    if source.starts_with('/') {
        return Some(source.to_string());
    }

    let (store_id, volume) = source.split_once(':')?;
    let storage = storage_config?.storages.get(store_id)?;

    match storage.storage_type.as_str() {
        "dir" => storage
            .options
            .get("path")
            .map(|base_path| resolve_dir_volume(base_path, volume)),
        "lvm" | "lvmthin" => storage.options.get("vgname").and_then(|vgname| {
            if volume.contains('/') {
                None
            } else {
                Some(format!("/dev/{}/{}", vgname, volume))
            }
        }),
        "zfspool" => storage
            .options
            .get("pool")
            .map(|pool| resolve_zfspool_volume(pool, volume)),
        _ => None,
    }
}

fn resolve_zfspool_volume(pool: &str, volume: &str) -> String {
    let trimmed_pool = pool.trim_matches('/');
    let trimmed_volume = volume.trim_start_matches('/');
    format!("/dev/zvol/{}/{}", trimmed_pool, trimmed_volume)
}

fn resolve_dir_volume(base_path: &str, volume: &str) -> String {
    let trimmed_base = base_path.trim_end_matches('/');

    if volume.starts_with('/') {
        return volume.to_string();
    }

    if volume.starts_with("images/")
        || volume.starts_with("iso/")
        || volume.starts_with("vztmpl/")
        || volume.starts_with("backup/")
        || volume.starts_with("snippets/")
        || volume.starts_with("template/")
        || volume.starts_with("rootdir/")
    {
        return format!("{}/{}", trimmed_base, volume);
    }

    if let Some((prefix, _)) = volume.split_once('/')
        && prefix.chars().all(|ch| ch.is_ascii_digit())
    {
        return format!("{}/images/{}", trimmed_base, volume);
    }

    format!("{}/{}", trimmed_base, volume)
}

fn map_network(
    network: &ProxmoxNetEntry,
    vmid: Option<u32>,
    is_windows: bool,
    warnings: &mut Vec<MappingWarning>,
) -> NetworkConfig {
    let model = match network.model.as_str() {
        "virtio" => {
            if is_windows {
                "virtio-net-pci".to_string()
            } else {
                "virtio-net".to_string()
            }
        }
        "virtio-net" => {
            if is_windows {
                "virtio-net-pci".to_string()
            } else {
                "virtio-net".to_string()
            }
        }
        "virtio-net-pci" | "e1000" | "e1000e" | "rtl8139" => network.model.clone(),
        other => {
            warnings.push(MappingWarning {
                source_field: network.key.clone(),
                message: format!(
                    "unsupported network model '{}' mapped to 'virtio-net'",
                    other
                ),
            });
            "virtio-net".to_string()
        }
    };

    let has_bridge = network.options.contains_key("bridge");
    let backend_type = if has_bridge {
        "tap".to_string()
    } else {
        "user".to_string()
    };

    let ifname = network.options.get("ifname").cloned().or_else(|| {
        if has_bridge {
            vmid.map(|id| format!("tap{}i{}", id, network.index))
        } else {
            None
        }
    });
    let script = if has_bridge {
        network
            .options
            .get("script")
            .cloned()
            .or_else(|| Some("/usr/libexec/qemu-server/pve-bridge".to_string()))
    } else {
        network.options.get("script").cloned()
    };
    let downscript = if has_bridge {
        network
            .options
            .get("downscript")
            .cloned()
            .or_else(|| Some("/usr/libexec/qemu-server/pve-bridgedown".to_string()))
    } else {
        network.options.get("downscript").cloned()
    };
    let vhost = network
        .options
        .get("vhost")
        .map(|value| is_enabled(Some(value)))
        .or(if has_bridge { Some(true) } else { None });

    let backend = NetworkBackendConfig {
        backend_type,
        ifname,
        bridge: network.options.get("bridge").cloned(),
        script,
        downscript,
        helper: None,
        vhost,
        queues: network
            .options
            .get("queues")
            .and_then(|v| v.parse::<u16>().ok()),
        hostfwd: Vec::new(),
        listen: None,
        connect: None,
        fd: None,
        extra: BTreeMap::new(),
    };

    // B-15: Extract queue sizes and PCI placement details
    let rx_queue_size = network
        .options
        .get("rx_queue_size")
        .or_else(|| network.options.get("rxqueuesz"))
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| {
            if is_windows && model == "virtio-net-pci" {
                Some(1024)
            } else {
                None
            }
        });

    let tx_queue_size = network
        .options
        .get("tx_queue_size")
        .or_else(|| network.options.get("txqueuesz"))
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| {
            if is_windows && model == "virtio-net-pci" {
                Some(256)
            } else {
                None
            }
        });

    let bus = network.options.get("bus").cloned().or_else(|| {
        if is_windows && model == "virtio-net-pci" {
            Some("pci.0".to_string())
        } else {
            None
        }
    });
    let addr = network.options.get("addr").cloned().or_else(|| {
        if is_windows && model == "virtio-net-pci" {
            Some(format!("0x{:x}", 0x12 + network.index as u32))
        } else {
            None
        }
    });

    NetworkConfig {
        id: network.key.clone(),
        model,
        backend: Some(backend),
        mac: network.mac.clone(),
        rx_queue_size,
        tx_queue_size,
        boot_index: None,
        bus,
        addr,
    }
}

fn infer_proxmox_vmid(proxmox: &ProxmoxVmConfig) -> Option<u32> {
    if let Some(vmid) = proxmox
        .scalars
        .get("vmid")
        .and_then(|value| value.trim().parse::<u32>().ok())
    {
        return Some(vmid);
    }

    for source in proxmox.disks.iter().map(|disk| disk.source.as_str()) {
        if let Some(vmid) = extract_vmid_from_source(source) {
            return Some(vmid);
        }
    }

    for key in ["efidisk0", "tpmstate0"] {
        if let Some(raw) = proxmox.scalars.get(key) {
            let (source, _) = parse_source_and_options(raw);
            if let Some(vmid) = extract_vmid_from_source(source) {
                return Some(vmid);
            }
        }
    }

    None
}

fn extract_vmid_from_source(source: &str) -> Option<u32> {
    let marker = "vm-";
    let start = source.find(marker)? + marker.len();
    let tail = &source[start..];
    let digit_count = tail.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }
    tail[..digit_count].parse::<u32>().ok()
}

fn map_host_pci_entries(
    entry: &ProxmoxHostPciEntry,
    explicit_host_functions: &BTreeSet<String>,
) -> Vec<HostPciConfig> {
    let has_explicit_function = entry
        .host
        .rsplit_once(':')
        .is_some_and(|(_, slot)| slot.contains('.'));
    let base_device = normalize_host_pci_device(&entry.host);
    let pcie = is_enabled(entry.options.get("pcie"));
    let requested_x_vga =
        is_enabled(entry.options.get("x-vga")) || is_enabled(entry.options.get("x_vga"));
    // Proxmox frequently expands hostpciN: <slot> into .0/.1 pairs without carrying x-vga to
    // the generated command line. Keep expansion behavior, but only emit x-vga when the source
    // explicitly targets a concrete PCI function.
    let x_vga = has_explicit_function && requested_x_vga;
    let wants_multifunction = is_enabled(entry.options.get("multifunction"));
    let should_expand_pair = !has_explicit_function && (requested_x_vga || wants_multifunction);

    let default_bus = if should_expand_pair {
        Some("ich9-pcie-port-1".to_string())
    } else {
        None
    };
    let base_bus = entry.options.get("bus").cloned().or(default_bus);
    let base_addr = entry.options.get("addr").cloned().or_else(|| {
        if should_expand_pair {
            Some("0x0.0".to_string())
        } else {
            None
        }
    });

    let base_id = if should_expand_pair {
        format!("{}.0", entry.key)
    } else {
        entry.key.clone()
    };

    let mut mapped = vec![HostPciConfig {
        device: base_device.clone(),
        id: base_id,
        pcie,
        x_vga,
        bus: base_bus.clone(),
        addr: base_addr.clone(),
        multifunction: wants_multifunction || should_expand_pair,
        romfile: entry.options.get("romfile").cloned(),
    }];

    if should_expand_pair && let Some(function_one) = sibling_function_one(&base_device) {
        if explicit_host_functions.contains(&function_one) {
            return mapped;
        }

        let second_addr = base_addr
            .as_deref()
            .and_then(increment_function_address)
            .or_else(|| Some("0x0.1".to_string()));

        mapped.push(HostPciConfig {
            device: function_one,
            id: format!("{}.1", entry.key),
            pcie: false,
            x_vga: false,
            bus: base_bus,
            addr: second_addr,
            multifunction: false,
            romfile: None,
        });
    }

    mapped
}

fn normalize_host_pci_device(device: &str) -> String {
    let trimmed = device.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if let Some((prefix, slot)) = trimmed.rsplit_once(':')
        && !slot.contains('.')
    {
        return format!("{}:{}.0", prefix, slot);
    }

    trimmed.to_string()
}

fn sibling_function_one(device: &str) -> Option<String> {
    let (prefix, slot_function) = device.rsplit_once(':')?;
    let (slot, function) = slot_function.split_once('.')?;
    if function != "0" {
        return None;
    }

    Some(format!("{}:{}.1", prefix, slot))
}

fn increment_function_address(addr: &str) -> Option<String> {
    addr.strip_suffix(".0")
        .map(|prefix| format!("{}.1", prefix))
}

fn map_usb(entry: &ProxmoxUsbEntry, place_on_xhci: bool) -> UsbDeviceConfig {
    let mut host = entry.host.clone();
    if let Some(value) = host.strip_prefix("host=") {
        host = value.to_string();
    }

    let (mapped_host, hostbus, hostport) = if let Some((bus, port)) = host.split_once('-') {
        if bus.chars().all(|c| c.is_ascii_digit())
            && port.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            (String::new(), Some(bus.to_string()), Some(port.to_string()))
        } else {
            (host, None, None)
        }
    } else {
        (host, None, None)
    };

    UsbDeviceConfig {
        id: entry.key.clone(),
        host: mapped_host,
        hostbus: hostbus.or_else(|| entry.options.get("hostbus").cloned()),
        hostport: hostport.or_else(|| entry.options.get("hostport").cloned()),
        bus: entry.options.get("bus").cloned().or_else(|| {
            if place_on_xhci {
                Some("xhci.0".to_string())
            } else {
                None
            }
        }),
        port: entry.options.get("port").cloned().or_else(|| {
            if place_on_xhci {
                Some((entry.index + 1).to_string())
            } else {
                None
            }
        }),
    }
}

fn map_tpm(
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

fn map_displays(
    scalars: &BTreeMap<String, String>,
    warnings: &mut Vec<MappingWarning>,
) -> Vec<DisplayConfig> {
    let Some(vga) = scalars.get("vga") else {
        return Vec::new();
    };

    let mut tokens = vga.split(',');
    let kind = tokens.next().unwrap_or("virtio").trim();

    let r#type = match kind {
        "virtio" | "virtio-gl" | "virtio-vga" => "virtio-gpu".to_string(),
        "qxl" => "qxl".to_string(),
        "cirrus" | "std" => "cirrus".to_string(),
        "vmware" | "vmware-svga" => "vmware-svga".to_string(),
        "none" => "none".to_string(),
        other => {
            warnings.push(MappingWarning {
                source_field: "vga".to_string(),
                message: format!("unsupported display '{}' mapped to 'virtio-gpu'", other),
            });
            "virtio-gpu".to_string()
        }
    };

    let mut vram = None;
    for token in tokens {
        let token = token.trim();
        if let Some((k, v)) = token.split_once('=')
            && k == "memory"
        {
            vram = v.parse::<u32>().ok();
        }
    }

    vec![DisplayConfig { r#type, vram }]
}

fn map_hugepages(scalars: &BTreeMap<String, String>) -> Option<HugepagesConfig> {
    let raw = scalars.get("hugepages")?;
    let raw = raw.trim();

    // "0" or "any" means disabled / use any available
    if raw == "0" || raw == "any" || raw.is_empty() {
        return None;
    }

    // Proxmox encodes hugepage size in MiB ("2" = 2 MiB, "1024" = 1 GiB)
    let size_mib: u64 = raw.parse().ok()?;
    let size_kib = size_mib * 1024;

    Some(HugepagesConfig {
        enabled: true,
        size_kib: Some(size_kib),
        mem_path: None, // will be derived from size_kib
        prealloc: true,
    })
}

fn synthesize_hugepages_numa(memory_mib: u32, vcpus: u32) -> Vec<NumaConfig> {
    let cpus: Vec<u32> = (0..vcpus).collect();
    vec![NumaConfig {
        id: 0,
        memory: memory_mib,
        cpus,
        host_node: None,
    }]
}

fn map_iommu(machine_options: &[String], raw_args: Option<&String>) -> Option<IommuConfig> {
    // Detect viommu=intel or viommu=amd in machine options (e.g. "q35,viommu=intel")
    for option in machine_options {
        if let Some(iommu_type) = option.strip_prefix("viommu=") {
            return Some(IommuConfig {
                r#type: iommu_type.to_string(),
                ..Default::default()
            });
        }
    }

    // Detect intel-iommu or amd-iommu in raw args passthrough
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

fn map_serials(
    scalars: &BTreeMap<String, String>,
    vmid: Option<u32>,
    warnings: &mut Vec<MappingWarning>,
) -> Vec<SerialConfig> {
    let mut serials = Vec::new();

    for (key, value) in scalars {
        let Some(port) = key
            .strip_prefix("serial")
            .and_then(|index| index.parse::<u32>().ok())
        else {
            continue;
        };

        let raw = value.trim();
        if raw.is_empty() || raw == "none" {
            continue;
        }

        if raw == "socket" {
            serials.push(default_socket_serial(port, vmid));
            continue;
        }

        if let Some(path) = raw.strip_prefix("file:") {
            serials.push(SerialConfig {
                r#type: "file".to_string(),
                id: Some(format!("serial{}", port)),
                port: Some(port),
                path: Some(path.trim().to_string()),
                host: None,
                socket_port: None,
                server: true,
                wait: false,
                chardev: None,
            });
            continue;
        }

        if raw == "pty" || raw == "stdio" {
            serials.push(SerialConfig {
                r#type: raw.to_string(),
                id: Some(format!("serial{}", port)),
                port: Some(port),
                path: None,
                host: None,
                socket_port: None,
                server: true,
                wait: false,
                chardev: None,
            });
            continue;
        }

        if let Some(chardev) = raw
            .strip_prefix("chardev:")
            .or_else(|| raw.strip_prefix("chardev="))
        {
            serials.push(SerialConfig {
                r#type: "chardev".to_string(),
                id: Some(format!("serial{}", port)),
                port: Some(port),
                path: None,
                host: None,
                socket_port: None,
                server: true,
                wait: false,
                chardev: Some(chardev.trim().to_string()),
            });
            continue;
        }

        if raw.starts_with("socket,") {
            serials.push(parse_socket_serial(raw, port, vmid));
            continue;
        }

        warnings.push(MappingWarning {
            source_field: key.clone(),
            message: format!(
                "unsupported serial backend '{}' mapped to default socket",
                raw
            ),
        });
        serials.push(default_socket_serial(port, vmid));
    }

    serials
}

fn default_socket_serial(port: u32, vmid: Option<u32>) -> SerialConfig {
    SerialConfig {
        r#type: "socket".to_string(),
        id: Some(format!("serial{}", port)),
        port: Some(port),
        path: vmid.map(|id| format!("/var/run/qemu-server/{}.serial{}", id, port)),
        host: if vmid.is_none() {
            Some("127.0.0.1".to_string())
        } else {
            None
        },
        socket_port: if vmid.is_none() {
            Some(4444 + port as u16)
        } else {
            None
        },
        server: true,
        wait: false,
        chardev: None,
    }
}

fn parse_socket_serial(raw: &str, port: u32, vmid: Option<u32>) -> SerialConfig {
    let mut serial = default_socket_serial(port, vmid);

    for token in raw.split(',').skip(1).map(str::trim) {
        if let Some((key, value)) = token.split_once('=') {
            match key.trim() {
                "path" => {
                    serial.path = Some(value.trim().to_string());
                    serial.host = None;
                    serial.socket_port = None;
                }
                "host" => {
                    serial.host = Some(value.trim().to_string());
                }
                "port" => {
                    serial.socket_port = value.trim().parse::<u16>().ok();
                    serial.path = None;
                }
                "server" => {
                    serial.server = matches!(value.trim(), "1" | "on" | "yes" | "true");
                }
                "wait" => {
                    serial.wait = matches!(value.trim(), "1" | "on" | "yes" | "true");
                }
                _ => {}
            }
        }
    }

    serial
}

fn is_enabled(value: Option<&String>) -> bool {
    matches!(
        value.map(String::as_str),
        Some("1") | Some("on") | Some("yes") | Some("true")
    )
}

fn infer_profile_names(proxmox: &ProxmoxVmConfig, config: &VmConfig) -> Vec<String> {
    let mut profiles = Vec::new();
    let ostype = proxmox.scalars.get("ostype").map(String::as_str);

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

fn parse_boot_order(scalars: &BTreeMap<String, String>) -> BTreeMap<String, u32> {
    let mut boot_indices = BTreeMap::new();
    if let Some(boot_str) = scalars.get("boot") {
        // Parse "order=scsi0;ide2;net0" format
        if let Some(order) = boot_str.strip_prefix("order=") {
            for (index, device_key) in order.split(';').enumerate() {
                let dev_key = device_key.trim();
                if !dev_key.is_empty() {
                    // Proxmox bootindex typically starts at 100, 101, 102...
                    boot_indices.insert(dev_key.to_string(), 100 + index as u32);
                }
            }
        }
    }
    boot_indices
}

fn parse_smbios_uuid(scalars: &BTreeMap<String, String>) -> Option<String> {
    scalars.get("smbios1").and_then(|smbios_str| {
        // Parse "uuid=04d064c3-66a1-4aa7-9589-f8b3ecf91cd7" format
        for token in smbios_str.split(',') {
            if let Some(uuid) = token.strip_prefix("uuid=") {
                return Some(uuid.trim().to_string());
            }
        }
        None
    })
}

fn apply_efidisk0_to_boot(
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

    // B-16: Map Microsoft certificate support from efidisk0 metadata
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

    // B-16: Map pre-enrolled keys indicator from efidisk0 metadata
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

fn parse_source_and_options(raw: &str) -> (&str, BTreeMap<String, String>) {
    let mut tokens = raw.split(',').map(str::trim).filter(|t| !t.is_empty());
    let source = tokens.next().unwrap_or("");
    let mut options = BTreeMap::new();

    for token in tokens {
        if let Some((k, v)) = token.split_once('=') {
            options.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    (source, options)
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

fn parse_human_size_to_bytes(raw: &str) -> Option<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut digits_end = 0;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            digits_end = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    if digits_end == 0 {
        return None;
    }

    let number = trimmed[..digits_end].parse::<u64>().ok()?;
    let suffix = trimmed[digits_end..].trim().to_ascii_lowercase();

    let multiplier = match suffix.as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024_u64.pow(2),
        "g" | "gb" => 1024_u64.pow(3),
        "t" | "tb" => 1024_u64.pow(4),
        _ => return None,
    };

    number.checked_mul(multiplier)
}

fn map_audio_and_spice(
    scalars: &BTreeMap<String, String>,
    warnings: &mut Vec<MappingWarning>,
) -> (Vec<AudioDeviceConfig>, Option<SpiceConfig>) {
    let Some(raw) = scalars.get("audio0") else {
        return (Vec::new(), None);
    };

    let options = parse_options(raw);
    let device = options
        .get("device")
        .map(String::as_str)
        .unwrap_or("ich9-intel-hda");
    if device != "ich9-intel-hda" {
        warnings.push(MappingWarning {
            source_field: "audio0".to_string(),
            message: format!(
                "unsupported audio device '{}' omitted (currently only ich9-intel-hda is mapped)",
                device
            ),
        });
        return (Vec::new(), None);
    }

    let driver = options
        .get("driver")
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "spice".to_string());
    if driver != "spice" {
        warnings.push(MappingWarning {
            source_field: "audio0".to_string(),
            message: format!(
                "unsupported audio driver '{}' omitted (currently only spice is mapped)",
                driver
            ),
        });
        return (Vec::new(), None);
    }

    let controller_id = "audiodev0".to_string();
    let backend_id = "spice-backend0".to_string();

    let audio = vec![
        AudioDeviceConfig {
            r#type: "ich9-intel-hda".to_string(),
            id: controller_id.clone(),
            bus: Some("pci.2".to_string()),
            addr: Some("0xc".to_string()),
            cad: None,
            audiodev: None,
        },
        AudioDeviceConfig {
            r#type: "hda-micro".to_string(),
            id: format!("{}-codec0", controller_id),
            bus: Some("audiodev0.0".to_string()),
            addr: None,
            cad: Some(0),
            audiodev: Some(backend_id.clone()),
        },
        AudioDeviceConfig {
            r#type: "hda-duplex".to_string(),
            id: "audiodev0-codec1".to_string(),
            bus: Some("audiodev0.0".to_string()),
            addr: None,
            cad: Some(1),
            audiodev: Some(backend_id),
        },
    ];

    (
        audio,
        Some(SpiceConfig {
            enabled: true,
            port: 5900,
            addr: "127.0.0.1".to_string(),
            disable_ticketing: false,
            audio: true,
            vdagent: true,
        }),
    )
}

fn map_guest_agent(
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

struct ArgsPassthroughOutput<'a> {
    spice: &'a mut Option<SpiceConfig>,
    vnc: &'a mut Option<VncConfig>,
    input_devices: &'a mut Vec<InputDeviceConfig>,
    ivshmem: &'a mut Option<IvshmemConfig>,
    applesmc: &'a mut Option<AppleSmcConfig>,
    smbios_type: &'a mut u8,
}

fn apply_args_passthrough_subset(
    scalars: &BTreeMap<String, String>,
    out: &mut ArgsPassthroughOutput<'_>,
    warnings: &mut Vec<MappingWarning>,
) {
    let Some(raw_args) = scalars.get("args") else {
        return;
    };

    let tokens = shell_split(raw_args);
    let mut index = 0usize;

    let mut ivshmem_memdev: Option<String> = None;
    let mut ivshmem_bus: Option<String> = None;
    let mut ivshmem_id: Option<String> = None;
    let mut ivshmem_mem_path: Option<String> = None;
    let mut ivshmem_size: Option<u32> = None;

    while index < tokens.len() {
        match tokens[index].as_str() {
            "-spice" => {
                if let Some(spec) = tokens.get(index + 1) {
                    apply_spice_spec(spec, out.spice);
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-spice missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            "-vnc" => {
                if let Some(spec) = tokens.get(index + 1) {
                    apply_vnc_spec(spec, out.vnc);
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-vnc missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            "-chardev" => {
                if let Some(spec) = tokens.get(index + 1) {
                    if is_vdagent_spicevmc(spec) {
                        ensure_spice(out.spice).vdagent = true;
                    }
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-chardev missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            "-device" => {
                if let Some(spec) = tokens.get(index + 1) {
                    let (device_type, options) = parse_prefixed_options(spec);
                    match device_type.as_str() {
                        "virtio-mouse" | "virtio-keyboard" => {
                            if !out.input_devices.iter().any(|d| d.r#type == device_type) {
                                out.input_devices.push(InputDeviceConfig {
                                    r#type: device_type,
                                });
                            }
                        }
                        "virtserialport" => {
                            if options.get("chardev").map(String::as_str) == Some("vdagent")
                                && options.get("name").map(String::as_str)
                                    == Some("com.redhat.spice.0")
                            {
                                ensure_spice(out.spice).vdagent = true;
                            }
                        }
                        "ivshmem-plain" => {
                            ivshmem_memdev = options.get("memdev").cloned();
                            ivshmem_bus = options.get("bus").cloned();
                        }
                        "isa-applesmc" => {
                            let osk = options.get("osk").cloned().unwrap_or_default();
                            *out.applesmc = Some(AppleSmcConfig { enabled: true, osk });
                        }
                        _ => {}
                    }
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-device missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            "-object" => {
                if let Some(spec) = tokens.get(index + 1) {
                    let (object_type, options) = parse_prefixed_options(spec);
                    if object_type == "memory-backend-file" {
                        ivshmem_id = options.get("id").cloned();
                        ivshmem_mem_path = options.get("mem-path").cloned();
                        ivshmem_size = options
                            .get("size")
                            .and_then(|raw| parse_human_size_to_bytes(raw))
                            .and_then(|bytes| u32::try_from(bytes / (1024 * 1024)).ok());
                    }
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-object missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            "-smbios" => {
                if let Some(spec) = tokens.get(index + 1) {
                    for token in spec.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                        if let Some((k, v)) = token.split_once('=')
                            && k.trim() == "type"
                            && let Ok(parsed) = v.trim().parse::<u8>()
                        {
                            *out.smbios_type = parsed;
                        }
                    }
                    index += 2;
                } else {
                    warnings.push(MappingWarning {
                        source_field: "args".to_string(),
                        message: "-smbios missing argument".to_string(),
                    });
                    index += 1;
                }
            }
            token if token.starts_with('-') => {
                warnings.push(MappingWarning {
                    source_field: "args".to_string(),
                    message: format!("unsupported args token '{}' ignored", token),
                });
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }

    let has_ivshmem_device = ivshmem_memdev.is_some() || ivshmem_bus.is_some();
    let has_ivshmem_object =
        ivshmem_id.is_some() || ivshmem_mem_path.is_some() || ivshmem_size.is_some();
    if has_ivshmem_device && has_ivshmem_object {
        let memdev_matches = ivshmem_memdev
            .as_ref()
            .zip(ivshmem_id.as_ref())
            .is_none_or(|(memdev, id)| memdev == id);

        if memdev_matches {
            *out.ivshmem = Some(IvshmemConfig {
                enabled: true,
                size: ivshmem_size.unwrap_or(32),
                vectors: 1,
                id: ivshmem_id.unwrap_or_else(|| "ivshmem0".to_string()),
                bus: ivshmem_bus,
                mem_path: ivshmem_mem_path.unwrap_or_else(|| "/dev/kvmfr0".to_string()),
            });
        } else {
            warnings.push(MappingWarning {
                source_field: "args".to_string(),
                message: "ivshmem memdev/id mismatch in args; ivshmem mapping skipped".to_string(),
            });
        }
    }
}

fn shell_split(raw: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for ch in raw.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse_prefixed_options(raw: &str) -> (String, BTreeMap<String, String>) {
    let mut tokens = raw.split(',').map(str::trim).filter(|t| !t.is_empty());
    let prefix = tokens.next().unwrap_or("").to_string();
    let mut options = BTreeMap::new();

    for token in tokens {
        if let Some((k, v)) = token.split_once('=') {
            options.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    (prefix, options)
}

fn apply_spice_spec(spec: &str, spice: &mut Option<SpiceConfig>) {
    let spice_cfg = ensure_spice(spice);
    for token in spec.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        if let Some((k, v)) = token.split_once('=') {
            match k.trim() {
                "port" => {
                    if let Ok(port) = v.trim().parse::<u16>() {
                        spice_cfg.port = port;
                    }
                }
                "addr" => spice_cfg.addr = v.trim().to_string(),
                "disable-ticketing" => {
                    spice_cfg.disable_ticketing = matches!(v.trim(), "on" | "1" | "yes" | "true")
                }
                _ => {}
            }
        }
    }
}

fn ensure_spice(spice: &mut Option<SpiceConfig>) -> &mut SpiceConfig {
    spice.get_or_insert(SpiceConfig {
        enabled: true,
        port: 5900,
        addr: "127.0.0.1".to_string(),
        disable_ticketing: false,
        audio: false,
        vdagent: false,
    })
}

fn apply_vnc_spec(spec: &str, vnc: &mut Option<VncConfig>) {
    let vnc_cfg = ensure_vnc(vnc);
    let mut parts = spec
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty());

    if let Some(display) = parts.next() {
        vnc_cfg.display = display.to_string();
    }

    for token in parts {
        if let Some((k, val)) = token.split_once('=')
            && k.trim() == "password"
        {
            vnc_cfg.password = matches!(val.trim(), "on" | "1" | "yes" | "true");
        }
    }
}

fn ensure_vnc(vnc: &mut Option<VncConfig>) -> &mut VncConfig {
    vnc.get_or_insert(VncConfig {
        enabled: true,
        display: "127.0.0.1:0".to_string(),
        password: false,
    })
}

fn is_vdagent_spicevmc(spec: &str) -> bool {
    let (prefix, options) = parse_prefixed_options(spec);
    prefix == "spicevmc"
        && options.get("id").map(String::as_str) == Some("vdagent")
        && options.get("name").map(String::as_str) == Some("vdagent")
}

fn parse_options(raw: &str) -> BTreeMap<String, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            token
                .split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{map_proxmox_to_canonical_yaml, map_proxmox_to_canonical_yaml_with_storage};
    use crate::config::{VmConfig, validation};
    use crate::import::proxmox::{parse_proxmox_config, parse_proxmox_storage_config};

    fn map_and_validate(input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        let config: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&config).expect("config should validate");
        (mapped.yaml, config)
    }

    fn map_and_validate_with_storage(input: &str, storage_input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let storage =
            parse_proxmox_storage_config(storage_input).expect("storage parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml_with_storage(&parsed, Some(&storage))
            .expect("mapper should succeed");
        let config: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&config).expect("config should validate");
        (mapped.yaml, config)
    }

    #[test]
    fn maps_cpu_and_memory() {
        let (yaml, cfg) = map_and_validate(
            r#"
            name: vm-cpu
            cpu: host
            cores: 4
            sockets: 2
            memory: 8192
            "#,
        );

        assert!(yaml.contains("backend: qemu"));
        assert_eq!(cfg.system.cpu.model, "host");
        assert_eq!(cfg.system.cpu.vcpus, 8);
        assert_eq!(cfg.system.memory.size, 8192);
    }

    #[test]
    fn maps_machine_options_and_cpu_features_from_scalar_fields() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-machine-cpu
            machine: type=pc-q35-8.1+pve0,hpet=off
            cpu: host,hv_ipi,hv_relaxed,kvm=off
            cores: 4
            "#,
        );

        assert_eq!(cfg.system.machine, "pc-q35-8.1+pve0");
        assert_eq!(cfg.system.machine_options, vec!["hpet=off".to_string()]);
        assert_eq!(cfg.system.cpu.model, "host");
        assert_eq!(
            cfg.system.cpu.features,
            vec![
                "hv_ipi".to_string(),
                "hv_relaxed".to_string(),
                "kvm=off".to_string()
            ]
        );
    }

    #[test]
    fn maps_machine_value_with_inline_options_without_type_prefix() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-machine-inline
            machine: pc-q35-8.1,hpet=off
            "#,
        );

        assert_eq!(cfg.system.machine, "pc-q35-8.1");
        assert_eq!(cfg.system.machine_options, vec!["hpet=off".to_string()]);
    }

    #[test]
    fn maps_storage_and_scsi_controller() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-storage
            scsihw: virtio-scsi-single
            scsi0: local-lvm:vm-200-disk-0,discard=on,ssd=1,cache=none
            ide2: none,media=cdrom
            "#,
        );

        assert_eq!(cfg.controllers.scsi.len(), 1);
        assert_eq!(cfg.controllers.scsi[0].id, "scsihw0");
        assert_eq!(cfg.controllers.scsi[0].r#type, "virtio-scsi-pci");
        assert_eq!(cfg.devices.drives[0].controller.as_deref(), Some("scsihw0"));
        assert_eq!(cfg.devices.drives.len(), 2);
        assert_eq!(cfg.devices.drives[0].interface, "scsi");
        assert!(cfg.devices.drives[0].discard);
        assert!(cfg.devices.drives[0].ssd);
        assert_eq!(cfg.devices.drives[1].r#type, "cdrom");
        assert!(
            cfg.profiles
                .contains(&"storage-virtio-scsi-single".to_string())
        );
    }

    #[test]
    fn maps_sata_drives_with_ahci_controller_and_port_buses() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-sata
            sata0: /var/lib/vm/sata0.raw,format=raw,discard=on,boot=100
            sata1: none,media=cdrom,ro=1,boot=101
            "#,
        );

        assert_eq!(cfg.controllers.sata.len(), 1);
        assert_eq!(cfg.controllers.sata[0].id, "sata0");
        assert_eq!(cfg.controllers.sata[0].r#type, "ahci");

        assert_eq!(cfg.devices.drives.len(), 2);

        let sata0 = cfg
            .devices
            .drives
            .iter()
            .find(|drive| drive.id == "sata0")
            .expect("sata0 should be present");
        assert_eq!(sata0.interface, "sata");
        assert_eq!(sata0.bus.as_deref(), Some("sata0.0"));
        assert_eq!(sata0.unit, Some(0));
        assert_eq!(sata0.boot_index, Some(100));

        let sata1 = cfg
            .devices
            .drives
            .iter()
            .find(|drive| drive.id == "sata1")
            .expect("sata1 should be present");
        assert_eq!(sata1.interface, "sata");
        assert_eq!(sata1.r#type, "cdrom");
        assert_eq!(sata1.bus.as_deref(), Some("sata0.1"));
        assert_eq!(sata1.unit, Some(0));
        assert_eq!(sata1.boot_index, Some(101));
    }

    #[test]
    fn maps_serial_socket_and_file_backends() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-serial
            serial0: socket,path=/tmp/serial0.sock,server=1,wait=0
            serial1: file:/tmp/serial1.log
            "#,
        );

        assert_eq!(cfg.devices.serials.len(), 2);

        let serial0 = &cfg.devices.serials[0];
        assert_eq!(serial0.r#type, "socket");
        assert_eq!(serial0.id.as_deref(), Some("serial0"));
        assert_eq!(serial0.port, Some(0));
        assert_eq!(serial0.path.as_deref(), Some("/tmp/serial0.sock"));
        assert!(serial0.server);
        assert!(!serial0.wait);

        let serial1 = &cfg.devices.serials[1];
        assert_eq!(serial1.r#type, "file");
        assert_eq!(serial1.id.as_deref(), Some("serial1"));
        assert_eq!(serial1.port, Some(1));
        assert_eq!(serial1.path.as_deref(), Some("/tmp/serial1.log"));
    }

    #[test]
    fn maps_serial_socket_default_path_from_vmid() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-serial-default
            scsi0: local-lvm:vm-108-disk-0,size=10G
            serial0: socket
            "#,
        );

        assert_eq!(cfg.devices.serials.len(), 1);
        let serial0 = &cfg.devices.serials[0];
        assert_eq!(serial0.r#type, "socket");
        assert_eq!(
            serial0.path.as_deref(),
            Some("/var/run/qemu-server/108.serial0")
        );
        assert_eq!(serial0.host, None);
        assert_eq!(serial0.socket_port, None);
    }

    #[test]
    fn resolves_storage_backed_disk_paths() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-storage-paths
            scsi0: vm1-pool:vm-108-boot,discard=on
            ide2: local:iso/virtio-win.iso,media=cdrom
            "#,
            r#"
            dir: local
                path /var/lib/vz
                content iso,vztmpl

            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        assert_eq!(cfg.devices.drives[0].path, "/dev/vm1/vm-108-boot");
        assert_eq!(cfg.devices.drives[0].format, "raw");
        assert_eq!(cfg.devices.drives[1].path, "/var/lib/vz/iso/virtio-win.iso");
        assert_eq!(cfg.devices.drives[1].format, "raw");
    }

    #[test]
    fn resolves_zfspool_backed_disk_paths() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-zfs-paths
            scsi0: local-zfs:vm-500-disk-0,discard=on
            "#,
            r#"
            zfspool: local-zfs
                pool rpool/data
                content images,rootdir
            "#,
        );

        assert_eq!(
            cfg.devices.drives[0].path,
            "/dev/zvol/rpool/data/vm-500-disk-0"
        );
        assert_eq!(cfg.devices.drives[0].format, "raw");
    }

    #[test]
    fn maps_network_bridge_and_mac() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0,queues=4,firewall=1
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 1);
        let net = &cfg.devices.networks[0];
        assert_eq!(net.model, "virtio-net");
        assert_eq!(net.mac.as_deref(), Some("52:54:00:12:34:56"));
        let backend = net.backend.as_ref().expect("backend must be set");
        assert_eq!(backend.backend_type, "tap");
        assert_eq!(backend.bridge.as_deref(), Some("vmbr0"));
        assert_eq!(backend.queues, Some(4));
        assert_eq!(
            backend.script.as_deref(),
            Some("/usr/libexec/qemu-server/pve-bridge")
        );
        assert_eq!(
            backend.downscript.as_deref(),
            Some("/usr/libexec/qemu-server/pve-bridgedown")
        );
        assert_eq!(backend.vhost, Some(true));
    }

    #[test]
    fn infers_tap_ifname_from_vmid_for_bridge_networks() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-ifname
            scsi0: local-lvm:vm-108-disk-0
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0
            "#,
        );

        let net = &cfg.devices.networks[0];
        let backend = net.backend.as_ref().expect("backend must be set");
        assert_eq!(backend.ifname.as_deref(), Some("tap108i0"));
    }

    #[test]
    fn maps_host_pci_and_usb() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-host
            hostpci0: 0000:03:00,pcie=1,x-vga=1,multifunction=1
            usb0: host=1-2
            usb1: host=0451:16a0
            "#,
        );

        assert_eq!(cfg.system.machine, "pc-q35-8.1+pve0");
        assert_eq!(
            cfg.system.readconfig,
            vec!["/usr/share/qemu-server/pve-q35-4.0.cfg".to_string()]
        );

        assert_eq!(cfg.host.pci.len(), 2);
        assert_eq!(cfg.host.pci[0].device, "0000:03:00.0");
        assert_eq!(cfg.host.pci[0].id, "hostpci0.0");
        assert!(cfg.host.pci[0].pcie);
        assert!(!cfg.host.pci[0].x_vga);
        assert!(cfg.host.pci[0].multifunction);
        assert_eq!(cfg.host.pci[0].bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(cfg.host.pci[0].addr.as_deref(), Some("0x0.0"));

        assert_eq!(cfg.host.pci[1].device, "0000:03:00.1");
        assert_eq!(cfg.host.pci[1].id, "hostpci0.1");
        assert!(!cfg.host.pci[1].pcie);
        assert!(!cfg.host.pci[1].x_vga);
        assert!(!cfg.host.pci[1].multifunction);
        assert_eq!(cfg.host.pci[1].bus.as_deref(), Some("ich9-pcie-port-1"));
        assert_eq!(cfg.host.pci[1].addr.as_deref(), Some("0x0.1"));

        assert_eq!(cfg.host.usb.len(), 2);
        assert_eq!(cfg.host.usb[0].hostbus.as_deref(), Some("1"));
        assert_eq!(cfg.host.usb[0].hostport.as_deref(), Some("2"));
        assert_eq!(cfg.host.usb[1].host, "0451:16a0");
    }

    #[test]
    fn preserves_explicit_hostpci_function_entries_without_synthetic_pairing() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-host-explicit
            hostpci0: 0000:03:00.0,pcie=1,x-vga=1,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=1
            hostpci1: 0000:03:00.1,bus=ich9-pcie-port-1,addr=0x0.1
            "#,
        );

        assert_eq!(cfg.host.pci.len(), 2);
        assert_eq!(cfg.host.pci[0].id, "hostpci0");
        assert_eq!(cfg.host.pci[0].device, "0000:03:00.0");
        assert!(cfg.host.pci[0].x_vga);
        assert_eq!(cfg.host.pci[1].id, "hostpci1");
        assert_eq!(cfg.host.pci[1].device, "0000:03:00.1");
    }

    #[test]
    fn maps_tpm_when_tpmstate_present() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-tpm
            bios: ovmf
            tpmstate0: local-lvm:vm-108-tpmstate,size=4M,version=v2.0
            "#,
        );

        let tpm = cfg.system.tpm.as_ref().expect("tpm should be mapped");
        assert_eq!(tpm.version, "2.0");
        assert_eq!(tpm.backend, "emulator");
        assert_eq!(tpm.model, "tpm-tis");
        assert_eq!(tpm.state_backend_uri, None);
    }

    #[test]
    fn maps_tpm_backend_uri_with_storage_resolution() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-tpm
            bios: ovmf
            tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
            "#,
            r#"
            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        let tpm = cfg.system.tpm.as_ref().expect("tpm should be mapped");
        assert_eq!(tpm.version, "2.0");
        assert_eq!(
            tpm.state_backend_uri.as_deref(),
            Some("/dev/vm1/vm-108-tpmstate")
        );
    }

    #[test]
    fn maps_efidisk0_into_boot_uefi_vars_with_storage_resolution() {
        let (_, cfg) = map_and_validate_with_storage(
            r#"
            name: vm-uefi
            bios: ovmf
            efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,size=4M
            "#,
            r#"
            lvmthin: vm1-pool
                thinpool pool
                vgname vm1
                content images,rootdir
            "#,
        );

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert_eq!(
            cfg.system.boot.uefi_vars.as_deref(),
            Some("/dev/vm1/vm-108-efidisk")
        );
        assert_eq!(cfg.system.boot.uefi_vars_size, Some(540_672));
    }

    #[test]
    fn maps_efidisk0_size_from_size_option_when_efitype_is_absent() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-uefi-size
            bios: ovmf
            efidisk0: /var/lib/vm/vars.fd,size=4M
            "#,
        );

        assert_eq!(
            cfg.system.boot.uefi_vars.as_deref(),
            Some("/var/lib/vm/vars.fd")
        );
        assert_eq!(cfg.system.boot.uefi_vars_size, Some(4 * 1024 * 1024));
    }

    #[test]
    fn maps_audio0_to_hda_devices_with_spice_backend() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-audio
            audio0: device=ich9-intel-hda,driver=spice
            "#,
        );

        assert_eq!(cfg.devices.audio.len(), 3);
        assert_eq!(cfg.devices.audio[0].r#type, "ich9-intel-hda");
        assert_eq!(cfg.devices.audio[0].id, "audiodev0");
        assert_eq!(cfg.devices.audio[1].r#type, "hda-micro");
        assert_eq!(cfg.devices.audio[2].r#type, "hda-duplex");

        let spice = cfg.spice.as_ref().expect("spice should be configured");
        assert!(spice.enabled);
        assert!(spice.audio);
        assert_eq!(spice.addr, "127.0.0.1");
        assert_eq!(spice.port, 5900);
    }

    #[test]
    fn unsupported_audio_driver_emits_warning_and_skips_audio_mapping() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-audio-unsupported
            audio0: device=ich9-intel-hda,driver=alsa
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert!(cfg.devices.audio.is_empty());
        assert!(cfg.spice.is_none());
        assert!(mapped.warnings.iter().any(|w| {
            w.source_field == "audio0" && w.message.contains("unsupported audio driver")
        }));
    }

    #[test]
    fn maps_agent_enabled_into_guest_agent_config() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-agent
            agent: 1
            "#,
        );

        let agent = cfg
            .options
            .guest_agent
            .as_ref()
            .expect("guest agent should be configured");
        assert!(agent.enabled);
        assert_eq!(
            agent.socket_path.as_deref(),
            Some("/var/run/qemu-server/qga.sock")
        );
        assert_eq!(agent.bus.as_deref(), Some("pci.0"));
        assert_eq!(agent.addr.as_deref(), Some("0x8"));
    }

    #[test]
    fn skips_guest_agent_mapping_when_agent_disabled() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-agent-off
            agent: enabled=0
            "#,
        );

        assert!(cfg.options.guest_agent.is_none());
    }

    #[test]
    fn maps_args_subset_for_spice_input_and_ivshmem() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-args
            args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on -chardev spicevmc,id=vdagent,name=vdagent -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 -device virtio-mouse -device virtio-keyboard -device ivshmem-plain,memdev=ivshmem0,bus=pcie.0 -object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
            "#,
        );

        let spice = cfg.spice.as_ref().expect("spice should be configured");
        assert!(spice.enabled);
        assert_eq!(spice.port, 5903);
        assert_eq!(spice.addr, "0.0.0.0");
        assert!(spice.disable_ticketing);
        assert!(spice.vdagent);

        assert!(cfg.devices.input.iter().any(|d| d.r#type == "virtio-mouse"));
        assert!(
            cfg.devices
                .input
                .iter()
                .any(|d| d.r#type == "virtio-keyboard")
        );

        let iv = cfg
            .system
            .memory
            .ivshmem
            .as_ref()
            .expect("ivshmem should be configured");
        assert!(iv.enabled);
        assert_eq!(iv.id, "ivshmem0");
        assert_eq!(iv.bus.as_deref(), Some("pcie.0"));
        assert_eq!(iv.mem_path, "/dev/kvmfr0");
        assert_eq!(iv.size, 128);
        assert!(cfg.profiles.contains(&"looking-glass".to_string()));
    }

    #[test]
    fn infers_windows_11_profile_stack() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-win11
            ostype: win11
            bios: ovmf
            machine: pc-q35-8.1+pve0
            agent: 1
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=2023,pre-enrolled-keys=1
            tpmstate0: /var/lib/vm/tpmstate,size=4M,version=v2.0
            audio0: device=ich9-intel-hda,driver=spice
            args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on -chardev spicevmc,id=vdagent,name=vdagent -device virtserialport,chardev=vdagent,name=com.redhat.spice.0 -device virtio-mouse -device virtio-keyboard
            scsi0: local-lvm:vm-108-disk-0
            scsihw: virtio-scsi-pci
            hostpci0: 0000:03:00.0,pcie=1
            "#,
        );

        assert_eq!(
            cfg.profiles,
            vec![
                "proxmox-q35-uefi".to_string(),
                "storage-virtio-scsi-pci".to_string(),
                "windows-common".to_string(),
                "windows-11".to_string(),
                "remote-viewer-spice".to_string(),
                "gpu-passthrough".to_string(),
            ]
        );
    }

    #[test]
    fn infers_macos_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-macos
            ostype: other
            bios: ovmf
            machine: pc-q35-5.2
            cpu: Penryn
            vga: none
            args: -device isa-applesmc,osk=dummy -smbios type=2
            hostpci0: 0000:07:00.0,pcie=1
            "#,
        );

        assert!(cfg.profiles.contains(&"macos-kvm".to_string()));
        assert!(cfg.profiles.contains(&"gpu-passthrough".to_string()));
        assert_eq!(cfg.system.smbios.as_ref().map(|s| s.smbios_type), Some(2));
        assert_eq!(
            cfg.system.applesmc.as_ref().map(|a| a.osk.as_str()),
            Some("dummy")
        );
    }

    #[test]
    fn infers_hugepages_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-huge
            ostype: l26
            bios: ovmf
            machine: pc-q35-8.1+pve0
            hugepages: 1024
            memory: 8192
            cores: 4
            "#,
        );

        assert!(cfg.profiles.contains(&"hugepages".to_string()));
    }

    #[test]
    fn maps_vnc_from_args_and_infers_headless_vnc_profile() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-headless-vnc
            vga: none
            args: -vnc unix:/var/run/qemu-server/405.vnc,password=on
            "#,
        );

        let vnc = cfg.vnc.as_ref().expect("vnc should be configured");
        assert!(vnc.enabled);
        assert_eq!(vnc.display, "unix:/var/run/qemu-server/405.vnc");
        assert!(vnc.password);
        assert!(cfg.profiles.contains(&"headless-vnc".to_string()));
        assert!(!cfg.profiles.contains(&"headless-serial".to_string()));
    }

    #[test]
    fn infers_viommu_and_hidden_hypervisor_profiles() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-hidden
            machine: q35,viommu=intel
            cpu: host,hidden=1
            "#,
        );

        assert!(cfg.profiles.contains(&"viommu".to_string()));
        assert!(cfg.profiles.contains(&"hidden-hypervisor".to_string()));
    }

    #[test]
    fn unsupported_args_token_emits_warning() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-args-unsupported
            args: -foo bar
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        assert!(mapped.warnings.iter().any(|w| {
            w.source_field == "args" && w.message.contains("unsupported args token '-foo'")
        }));
    }

    #[test]
    fn maps_display_from_vga() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-display
            vga: virtio-gl,memory=64
            "#,
        );

        assert_eq!(cfg.devices.displays.len(), 1);
        assert_eq!(cfg.devices.displays[0].r#type, "virtio-gpu");
        assert_eq!(cfg.devices.displays[0].vram, Some(64));
    }

    #[test]
    fn maps_with_defaults_when_fields_missing() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-defaults
            "#,
        );

        assert_eq!(cfg.backend, "qemu");
        assert_eq!(cfg.system.architecture, "x86_64");
        assert_eq!(cfg.system.machine, "q35");
        assert_eq!(cfg.system.cpu.vcpus, 1);
        assert_eq!(cfg.system.memory.size, 2048);
    }

    #[test]
    fn warnings_include_source_field_reference() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-warn
            arch: sparc
            memory: 2048
            cores: 2
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        assert_eq!(mapped.warnings.len(), 1);
        assert_eq!(mapped.warnings[0].source_field, "arch");
        assert!(
            mapped.warnings[0]
                .message
                .contains("unsupported Proxmox architecture")
        );
    }

    #[test]
    fn b14_maps_cpu_hyperv_features_for_windows_guests() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-windows-hyperv
            cores: 4
            cpu: host,+hyperv-vendor-id,+hyperv-enlightened-vmx,+hyperv-time,+hyperv-synic
            "#,
        );

        let cpu = &cfg.system.cpu;
        assert_eq!(cpu.vcpus, 4);
        assert_eq!(cpu.model, "host");
        assert!(cpu.features.contains(&"+hyperv-vendor-id".to_string()));
        assert!(
            cpu.features
                .contains(&"+hyperv-enlightened-vmx".to_string())
        );
        assert!(cpu.features.contains(&"+hyperv-time".to_string()));
        assert!(cpu.features.contains(&"+hyperv-synic".to_string()));
    }

    #[test]
    fn b15_maps_network_device_queue_sizes_and_pci_placement() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-net-queues
            net0: virtio=52:54:00:12:34:56,bridge=vmbr0,rx_queue_size=256,tx_queue_size=256,bus=pci.0,addr=1f.0
            "#,
        );

        assert_eq!(cfg.devices.networks.len(), 1);
        let net = &cfg.devices.networks[0];
        assert_eq!(net.rx_queue_size, Some(256));
        assert_eq!(net.tx_queue_size, Some(256));
        assert_eq!(net.bus.as_deref(), Some("pci.0"));
        assert_eq!(net.addr.as_deref(), Some("1f.0"));
    }

    #[test]
    fn b16_maps_efidisk0_with_ms_cert_and_pre_enrolled_keys() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-efi-secure
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=2023,pre-enrolled-keys=1
            "#,
        );

        let boot = &cfg.system.boot;
        assert_eq!(boot.firmware.as_deref(), Some("uefi"));
        assert_eq!(boot.uefi_vars.as_deref(), Some("/var/lib/vm/vars.fd"));
        assert_eq!(boot.uefi_vars_size, Some(540_672));
        assert_eq!(boot.uefi_ms_cert.as_deref(), Some("2023"));
        assert_eq!(boot.uefi_pre_enrolled_keys.as_deref(), Some("1"));
        assert!(boot.secure_boot);
    }

    #[test]
    fn efidisk_metadata_can_disable_secure_boot_signal() {
        let (_, cfg) = map_and_validate(
            r#"
            name: vm-efi-non-secure
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=none,pre-enrolled-keys=0
            "#,
        );

        let boot = &cfg.system.boot;
        assert_eq!(boot.firmware.as_deref(), Some("uefi"));
        assert!(!boot.secure_boot);
    }

    #[test]
    fn efidisk_overrides_bios_firmware_with_warning() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-efi-override
            bios: seabios
            efidisk0: /var/lib/vm/vars.fd,efitype=4m
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert!(mapped.warnings.iter().any(|warning| {
            warning.source_field == "efidisk0"
                && warning
                    .message
                    .contains("overriding BIOS firmware mapping to 'uefi'")
        }));
    }

    #[test]
    fn efidisk_warns_when_secure_boot_metadata_is_not_representable() {
        let parsed = parse_proxmox_config(
            r#"
            name: vm-efi-warning
            efidisk0: /var/lib/vm/vars.fd,efitype=4m,ms-cert=custom,pre-enrolled-keys=maybe
            "#,
        )
        .expect("parser should succeed");

        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
        let cfg: VmConfig = serde_yaml::from_str(&mapped.yaml).expect("yaml should deserialize");
        validation::validate_config(&cfg).expect("config should validate");

        assert_eq!(cfg.system.boot.firmware.as_deref(), Some("uefi"));
        assert!(!cfg.system.boot.secure_boot);
        assert_eq!(mapped.warnings.len(), 2);
        assert!(
            mapped
                .warnings
                .iter()
                .all(|warning| warning.source_field == "efidisk0")
        );
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.message.contains("ms-cert='custom'"))
        );
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.message.contains("pre-enrolled-keys='maybe'"))
        );
    }
}
