use super::error::ImportError;
use super::model::{
    ProxmoxDiskEntry, ProxmoxHostPciEntry, ProxmoxNetEntry, ProxmoxUsbEntry, ProxmoxVmConfig,
};
use crate::config::{
    BootConfig, ControllersConfig, DeviceConfig, DisplayConfig, DriveConfig, HostConfig,
    HostPciConfig, MemoryConfig, NetworkBackendConfig, NetworkConfig, ScsiControllerConfig,
    SystemConfig, TpmConfig, UsbDeviceConfig, VmConfig, VmOptions,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct CanonicalMappingResult {
    pub yaml: String,
    pub warnings: Vec<String>,
}

pub fn map_proxmox_to_canonical_yaml(
    proxmox: &ProxmoxVmConfig,
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

    let boot = BootConfig {
        firmware,
        ..Default::default()
    };

    let scsi_controllers = map_scsi_controllers(&proxmox.scalars, &proxmox.disks, &mut warnings);
    let drives = proxmox.disks.iter().map(map_drive).collect::<Vec<_>>();
    let networks = proxmox
        .networks
        .iter()
        .map(|network| map_network(network, &mut warnings))
        .collect::<Vec<_>>();
    let displays = map_displays(&proxmox.scalars, &mut warnings);
    let host_pci = proxmox
        .host_pci
        .iter()
        .map(map_host_pci)
        .collect::<Vec<_>>();
    let host_usb = proxmox.usb.iter().map(map_usb).collect::<Vec<_>>();
    let tpm = map_tpm(&proxmox.scalars, &boot);

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
                ballooning: None,
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
            smbios: None,
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
        options: VmOptions::default(),
    };

    let yaml = serde_yaml::to_string(&vm_config)
        .map_err(|e| ImportError::ParseError(format!("failed to serialize mapped config: {e}")))?;

    Ok(CanonicalMappingResult { yaml, warnings })
}

fn map_architecture(arch: Option<&String>, warnings: &mut Vec<String>) -> String {
    match arch.map(String::as_str).unwrap_or("x86_64") {
        "amd64" => "x86_64".to_string(),
        "x86_64" | "aarch64" | "x86" | "ppc64" | "riscv64" => {
            arch.cloned().unwrap_or_else(|| "x86_64".to_string())
        }
        other => {
            warnings.push(format!(
                "unsupported Proxmox architecture '{}' mapped to 'x86_64'",
                other
            ));
            "x86_64".to_string()
        }
    }
}

fn map_machine(machine: Option<&String>) -> String {
    let m = machine.map(String::as_str).unwrap_or("q35");
    if m.contains("q35") {
        "q35".to_string()
    } else {
        m.to_string()
    }
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
    warnings: &mut Vec<String>,
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
            warnings.push(format!(
                "unsupported scsihw '{}' mapped to 'virtio-scsi-pci'",
                unknown
            ));
            "virtio-scsi-pci".to_string()
        }
    };

    vec![ScsiControllerConfig {
        id: "scsi0".to_string(),
        r#type: controller_type,
        iothread: None,
        max_targets: None,
        bus: None,
        addr: None,
    }]
}

fn map_drive(disk: &ProxmoxDiskEntry) -> DriveConfig {
    let is_cdrom = disk
        .options
        .get("media")
        .map(|v| v == "cdrom")
        .unwrap_or(false)
        || disk.source == "none";

    let path = if is_cdrom && disk.source == "none" {
        String::new()
    } else {
        disk.source.clone()
    };

    let format = disk.options.get("format").cloned().unwrap_or_else(|| {
        if is_cdrom {
            "raw".to_string()
        } else {
            "qcow2".to_string()
        }
    });

    let discard = is_enabled(disk.options.get("discard"));
    let ssd = is_enabled(disk.options.get("ssd"));
    let readonly = is_enabled(disk.options.get("readonly")) || is_enabled(disk.options.get("ro"));

    let boot_index = disk.options.get("boot").and_then(|v| v.parse::<u32>().ok());

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
        cache: disk.options.get("cache").cloned(),
        aio: disk.options.get("aio").cloned(),
        detect_zeroes: disk
            .options
            .get("detect_zeroes")
            .cloned()
            .or_else(|| disk.options.get("detect-zeroes").cloned()),
        controller: if disk.bus == "scsi" {
            Some("scsi0".to_string())
        } else {
            None
        },
        boot_index,
        scsi_id: None,
        bus: None,
        unit: None,
    }
}

fn map_network(network: &ProxmoxNetEntry, warnings: &mut Vec<String>) -> NetworkConfig {
    let model = match network.model.as_str() {
        "virtio" => "virtio-net".to_string(),
        "virtio-net" | "virtio-net-pci" | "e1000" | "e1000e" | "rtl8139" => network.model.clone(),
        other => {
            warnings.push(format!(
                "unsupported network model '{}' mapped to 'virtio-net'",
                other
            ));
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
        device: entry.host.clone(),
        id: entry.key.clone(),
        pcie: is_enabled(entry.options.get("pcie")),
        x_vga: is_enabled(entry.options.get("x-vga")) || is_enabled(entry.options.get("x_vga")),
        bus: entry.options.get("bus").cloned(),
        addr: entry.options.get("addr").cloned(),
        multifunction: is_enabled(entry.options.get("multifunction")),
        romfile: entry.options.get("romfile").cloned(),
    }
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

fn map_tpm(scalars: &BTreeMap<String, String>, boot: &BootConfig) -> Option<TpmConfig> {
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

    let model = if boot.firmware.as_deref() == Some("uefi") {
        "tpm-crb".to_string()
    } else {
        "tpm-tis".to_string()
    };

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
    warnings: &mut Vec<String>,
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
            warnings.push(format!(
                "unsupported display '{}' mapped to 'virtio-gpu'",
                other
            ));
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

#[cfg(test)]
mod tests {
    use super::map_proxmox_to_canonical_yaml;
    use crate::config::{VmConfig, validation};
    use crate::import::proxmox::parse_proxmox_config;

    fn map_and_validate(input: &str) -> (String, VmConfig) {
        let parsed = parse_proxmox_config(input).expect("parser should succeed");
        let mapped = map_proxmox_to_canonical_yaml(&parsed).expect("mapper should succeed");
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
        assert_eq!(cfg.controllers.scsi[0].r#type, "virtio-scsi-pci");
        assert_eq!(cfg.devices.drives.len(), 2);
        assert_eq!(cfg.devices.drives[0].interface, "scsi");
        assert!(cfg.devices.drives[0].discard);
        assert!(cfg.devices.drives[0].ssd);
        assert_eq!(cfg.devices.drives[1].r#type, "cdrom");
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
            hostpci0: 0000:03:00.0,pcie=1,x-vga=1,multifunction=1
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
        assert_eq!(tpm.model, "tpm-crb");
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
}
