use super::helpers::{
    parse_options, parse_prefixed_options, parse_human_size_to_bytes, shell_split,
};
use super::super::model::{ProxmoxHostPciEntry, ProxmoxUsbEntry};
use super::{
    AppleSmcConfig, AudioDeviceConfig, DisplayConfig, InputDeviceConfig, IvshmemConfig,
    MappingWarning, SerialConfig, SpiceConfig, UsbDeviceConfig, VncConfig,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn map_host_pci_entries(
    entry: &ProxmoxHostPciEntry,
    explicit_host_functions: &BTreeSet<String>,
) -> Vec<super::HostPciConfig> {
    let has_explicit_function = entry
        .host
        .rsplit_once(':')
        .is_some_and(|(_, slot)| slot.contains('.'));
    let base_device = normalize_host_pci_device(&entry.host);
    let pcie = super::helpers::is_enabled(entry.options.get("pcie"));
    let requested_x_vga = super::helpers::is_enabled(entry.options.get("x-vga"))
        || super::helpers::is_enabled(entry.options.get("x_vga"));
    let x_vga = has_explicit_function && requested_x_vga;
    let wants_multifunction = super::helpers::is_enabled(entry.options.get("multifunction"));
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

    let mut mapped = vec![super::HostPciConfig {
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

        mapped.push(super::HostPciConfig {
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

pub(super) fn normalize_host_pci_device(device: &str) -> String {
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
    addr.strip_suffix(".0").map(|prefix| format!("{}.1", prefix))
}

pub(super) fn map_usb(entry: &ProxmoxUsbEntry, place_on_xhci: bool) -> UsbDeviceConfig {
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

pub(super) fn map_displays(
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

pub(super) fn map_serials(
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

pub(super) fn map_audio_and_spice(
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

pub(super) struct ArgsPassthroughOutput<'a> {
    pub spice: &'a mut Option<SpiceConfig>,
    pub vnc: &'a mut Option<VncConfig>,
    pub input_devices: &'a mut Vec<InputDeviceConfig>,
    pub ivshmem: &'a mut Option<IvshmemConfig>,
    pub applesmc: &'a mut Option<AppleSmcConfig>,
    pub smbios_type: &'a mut u8,
}

pub(super) fn apply_args_passthrough_subset(
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
