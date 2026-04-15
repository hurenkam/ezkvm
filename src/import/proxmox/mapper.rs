use super::error::ImportError;
use super::model::{
    ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxNetEntry, ProxmoxStorageConfig, ProxmoxUsbEntry,
    ProxmoxVmConfig,
};
use crate::config::{
    BallooningConfig, BootConfig, ControllersConfig, DeviceConfig, DisplayConfig, DriveConfig,
    HostConfig, HostPciConfig, MemoryConfig, NetworkBackendConfig, NetworkConfig, RtcConfig,
    ScsiControllerConfig, SmbiosConfig, SystemConfig, TpmConfig, UsbDeviceConfig, VmConfig,
    VmOptions,
};
use std::collections::BTreeMap;

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
    let machine = map_machine(proxmox.scalars.get("machine"));

    let memory = proxmox
        .scalars
        .get("memory")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(2048);
    let vcpus = map_vcpus(&proxmox.scalars);
    let cpu_model = proxmox
        .scalars
        .get("cpu")
        .cloned()
        .unwrap_or_else(|| "host".to_string());

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

    let boot = BootConfig {
        firmware,
        ..Default::default()
    };

    // Parse boot order and SMBIOS early
    let boot_indices = parse_boot_order(&proxmox.scalars);
    let smbios_uuid = parse_smbios_uuid(&proxmox.scalars);

    let scsi_controllers = map_scsi_controllers(&proxmox.scalars, &proxmox.disks, &mut warnings);
    let mut drives = proxmox
        .disks
        .iter()
        .map(|disk| map_drive(disk, storage_config))
        .collect::<Vec<_>>();
    let mut networks = proxmox
        .networks
        .iter()
        .map(|network| map_network(network, &mut warnings))
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
    let host_pci = proxmox
        .host_pci
        .iter()
        .map(map_host_pci)
        .collect::<Vec<_>>();
    let host_usb = proxmox.usb.iter().map(map_usb).collect::<Vec<_>>();

    let ballooning = Some(BallooningConfig {
        enabled: true,
        free_page_reporting: false,
        model: "virtio-balloon-pci".to_string(),
        id: None,
        bus: None,
        addr: None,
    });

    let tpm = map_tpm(&proxmox.scalars, &boot);

    let smbios = smbios_uuid.map(|uuid| SmbiosConfig {
        manufacturer: None,
        product: None,
        version: None,
        serial: None,
        uuid: Some(uuid),
        sku: None,
        family: None,
        vm_generation_id: proxmox.scalars.get("vmgenid").cloned(),
    });

    let vm_config = VmConfig {
        name,
        backend: "qemu".to_string(),
        profiles: Vec::new(),
        system: SystemConfig {
            architecture,
            machine,
            machine_options: Vec::new(),
            memory: MemoryConfig {
                size: memory,
                ballooning,
                ivshmem: None,
            },
            cpu: crate::config::CpuConfig {
                model: cpu_model,
                vcpus,
                features: Vec::new(),
                numa: Vec::new(),
            },
            boot,
            tpm,
            smbios,
            readconfig: Vec::new(),
        },
        devices: DeviceConfig {
            drives,
            networks,
            displays,
            serials: Vec::new(),
            input: Vec::new(),
            audio: Vec::new(),
        },
        controllers: ControllersConfig {
            scsi: scsi_controllers,
            xhci: Vec::new(),
        },
        host: HostConfig {
            pci: host_pci,
            usb: host_usb,
        },
        spice: None,
        iscsi_disks: Vec::new(),
        hyperv: None,
        options: VmOptions {
            rtc: if is_windows {
                Some(RtcConfig {
                    base: Some("localtime".to_string()),
                    driftfix: Some("slew".to_string()),
                })
            } else {
                None
            },
            ..VmOptions::default()
        },
    };

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

fn map_machine(machine: Option<&String>) -> String {
    let m = machine.map(String::as_str).unwrap_or("q35");
    // Keep full machine version (e.g., "pc-q35-8.1"), don't strip to just "q35"
    m.to_string()
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
        scsi_id: None,
        rotation_rate: if ssd && disk.bus == "scsi" {
            Some(1)
        } else {
            None
        },
        bus: None,
        unit: None,
    }
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

fn map_network(network: &ProxmoxNetEntry, warnings: &mut Vec<MappingWarning>) -> NetworkConfig {
    let model = match network.model.as_str() {
        "virtio" => "virtio-net".to_string(),
        "virtio-net" | "virtio-net-pci" | "e1000" | "e1000e" | "rtl8139" => network.model.clone(),
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

    let backend_type = if network.options.contains_key("bridge") {
        "bridge".to_string()
    } else {
        "user".to_string()
    };

    let mut extra = BTreeMap::new();
    for (k, v) in &network.options {
        if k != "bridge" && k != "queues" {
            extra.insert(k.clone(), v.clone());
        }
    }

    let backend = NetworkBackendConfig {
        backend_type,
        ifname: None,
        bridge: network.options.get("bridge").cloned(),
        script: None,
        downscript: None,
        helper: None,
        vhost: None,
        queues: network
            .options
            .get("queues")
            .and_then(|v| v.parse::<u16>().ok()),
        hostfwd: Vec::new(),
        listen: None,
        connect: None,
        fd: None,
        extra,
    };

    NetworkConfig {
        id: network.key.clone(),
        model,
        backend: Some(backend),
        mac: network.mac.clone(),
        rx_queue_size: None,
        tx_queue_size: None,
        boot_index: None,
        bus: None,
        addr: None,
    }
}

fn map_host_pci(entry: &ProxmoxHostPciEntry) -> HostPciConfig {
    HostPciConfig {
        device: normalize_host_pci_device(&entry.host),
        id: entry.key.clone(),
        pcie: is_enabled(entry.options.get("pcie")),
        x_vga: is_enabled(entry.options.get("x-vga")) || is_enabled(entry.options.get("x_vga")),
        bus: entry.options.get("bus").cloned(),
        addr: entry.options.get("addr").cloned(),
        multifunction: is_enabled(entry.options.get("multifunction")),
        romfile: entry.options.get("romfile").cloned(),
    }
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

fn map_usb(entry: &ProxmoxUsbEntry) -> UsbDeviceConfig {
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
        bus: entry.options.get("bus").cloned(),
        port: entry.options.get("port").cloned(),
    }
}

fn map_tpm(scalars: &BTreeMap<String, String>, _boot: &BootConfig) -> Option<TpmConfig> {
    let (key, raw) = scalars
        .iter()
        .find(|(k, _)| k.starts_with("tpmstate"))
        .map(|(k, v)| (k.as_str(), v.as_str()))?;

    let mut version = "2.0".to_string();
    for token in raw.split(',') {
        let token = token.trim();
        if let Some(v) = token.strip_prefix("version=") {
            version = v.trim_start_matches('v').to_string();
        }
    }

    let model = "tpm-tis".to_string();

    Some(TpmConfig {
        version,
        backend: "emulator".to_string(),
        state_path: Some(format!("/var/run/ezkvm/{}-tpm.socket", key)),
        state_dir: None,
        state_backend_uri: None,
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

fn is_enabled(value: Option<&String>) -> bool {
    matches!(
        value.map(String::as_str),
        Some("1") | Some("on") | Some("yes") | Some("true")
    )
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
        assert_eq!(backend.backend_type, "bridge");
        assert_eq!(backend.bridge.as_deref(), Some("vmbr0"));
        assert_eq!(backend.queues, Some(4));
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

        assert_eq!(cfg.host.pci.len(), 1);
        assert_eq!(cfg.host.pci[0].device, "0000:03:00.0");
        assert!(cfg.host.pci[0].pcie);
        assert!(cfg.host.pci[0].x_vga);
        assert!(cfg.host.pci[0].multifunction);

        assert_eq!(cfg.host.usb.len(), 2);
        assert_eq!(cfg.host.usb[0].hostbus.as_deref(), Some("1"));
        assert_eq!(cfg.host.usb[0].hostport.as_deref(), Some("2"));
        assert_eq!(cfg.host.usb[1].host, "0451:16a0");
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
}
