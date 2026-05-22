use super::error::ImportError;
use super::model::{QemuCmdModel, QemuCmdOption, QemuCmdOptionValue, QemuCsvPart};
use crate::config::{
    BootConfig, ControllersConfig, CpuConfig, DeviceConfig, HostConfig, MemoryConfig,
    NetworkBackendConfig, NetworkConfig, SystemConfig, VmConfig, VmOptions,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MappingWarningKind {
    UnsupportedFlag,
    UnsupportedValue,
    AmbiguousPairing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingWarning {
    pub source_field: String,
    pub kind: MappingWarningKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CanonicalMappingResult {
    pub yaml: String,
    pub warnings: Vec<MappingWarning>,
}

#[derive(Debug, Clone)]
struct NetdevDefinition {
    id: String,
    backend: NetworkBackendConfig,
}

pub fn map_qemu_cmd_to_canonical_yaml(
    qemu_cmd: &QemuCmdModel,
) -> Result<CanonicalMappingResult, ImportError> {
    let mut warnings = Vec::new();

    let name = map_vm_name(qemu_cmd).unwrap_or_else(|| "imported-qemu-cmd-vm".to_string());
    let memory_size = map_memory_size_mib(qemu_cmd, &mut warnings).unwrap_or(2048);
    let vcpus = map_vcpu_count(qemu_cmd, &mut warnings).unwrap_or(1);
    let (machine, machine_options) =
        map_machine(qemu_cmd).unwrap_or_else(|| ("q35".to_string(), vec!["hpet=off".to_string()]));
    let (cpu_model, cpu_features) =
        map_cpu(qemu_cmd).unwrap_or_else(|| ("host".to_string(), Vec::new()));

    let netdev_definitions = map_netdev_definitions(qemu_cmd, &mut warnings);
    let networks = map_networks(qemu_cmd, &netdev_definitions, &mut warnings);
    let profiles = infer_profile_names(qemu_cmd);

    let vm_config = VmConfig {
        name,
        backend: "qemu".to_string(),
        profiles,
        system: SystemConfig {
            architecture: "x86_64".to_string(),
            machine,
            machine_options,
            memory: MemoryConfig {
                size: memory_size,
                ballooning: None,
                ivshmem: None,
                hugepages: None,
            },
            cpu: CpuConfig {
                model: cpu_model,
                vcpus,
                features: cpu_features,
                numa: Vec::new(),
            },
            boot: BootConfig::default(),
            tpm: None,
            smbios: None,
            applesmc: None,
            readconfig: Vec::new(),
        },
        devices: DeviceConfig {
            drives: Vec::new(),
            networks,
            displays: Vec::new(),
            serials: Vec::new(),
            input: Vec::new(),
            audio: Vec::new(),
        },
        controllers: ControllersConfig::default(),
        host: HostConfig::default(),
        spice: map_spice(qemu_cmd, &mut warnings),
        vnc: None,
        iscsi_disks: Vec::new(),
        hyperv: None,
        iommu: None,
        options: VmOptions::default(),
    };

    collect_unsupported_flag_warnings(qemu_cmd, &mut warnings);

    let yaml = serde_yaml::to_string(&vm_config).map_err(|error| {
        ImportError::ParseError(format!("failed to serialize mapped YAML: {error}"))
    })?;

    Ok(CanonicalMappingResult { yaml, warnings })
}

fn map_vm_name(qemu_cmd: &QemuCmdModel) -> Option<String> {
    let option = qemu_cmd.options_for_flag("-name").next()?;
    let scalar = scalar_value(option)?;
    let trimmed = scalar.trim();
    if trimmed.is_empty() {
        return None;
    }

    let raw_name = trimmed.split(',').next().unwrap_or(trimmed).trim();
    if raw_name.is_empty() {
        None
    } else {
        Some(raw_name.to_string())
    }
}

fn map_memory_size_mib(qemu_cmd: &QemuCmdModel, warnings: &mut Vec<MappingWarning>) -> Option<u32> {
    let option = qemu_cmd.options_for_flag("-m").next()?;
    let scalar = scalar_value(option)?;

    parse_memory_to_mib(scalar).or_else(|| {
        warnings.push(MappingWarning {
            source_field: "-m".to_string(),
            kind: MappingWarningKind::UnsupportedValue,
            message: format!("unable to parse memory value '{scalar}'"),
        });
        None
    })
}

fn parse_memory_to_mib(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(number) = trimmed.strip_suffix(['G', 'g']) {
        let gib: u32 = number.trim().parse().ok()?;
        return gib.checked_mul(1024);
    }
    if let Some(number) = trimmed.strip_suffix(['M', 'm']) {
        return number.trim().parse().ok();
    }

    trimmed.parse().ok()
}

fn map_vcpu_count(qemu_cmd: &QemuCmdModel, warnings: &mut Vec<MappingWarning>) -> Option<u32> {
    let option = qemu_cmd.options_for_flag("-smp").next()?;
    let scalar = scalar_value(option)?;

    parse_vcpu_count(scalar).or_else(|| {
        warnings.push(MappingWarning {
            source_field: "-smp".to_string(),
            kind: MappingWarningKind::UnsupportedValue,
            message: format!("unable to parse vCPU count from '{scalar}'"),
        });
        None
    })
}

fn parse_vcpu_count(value: &str) -> Option<u32> {
    let first = value.split(',').next()?.trim();
    first.parse().ok()
}

fn map_machine(qemu_cmd: &QemuCmdModel) -> Option<(String, Vec<String>)> {
    let option = qemu_cmd.options_for_flag("-machine").next()?;

    match &option.value {
        QemuCmdOptionValue::Csv(parts) => {
            let mut machine = None;
            let mut machine_options = Vec::new();

            for part in parts {
                match part {
                    QemuCsvPart::Bare(value) => {
                        if machine.is_none() {
                            machine = Some(value.clone());
                        }
                    }
                    QemuCsvPart::KeyValue { key, value } => {
                        if key == "type" {
                            machine = Some(value.clone());
                        } else {
                            machine_options.push(format!("{key}={value}"));
                        }
                    }
                }
            }

            machine.map(|resolved_machine| (resolved_machine, machine_options))
        }
        QemuCmdOptionValue::Scalar(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((trimmed.to_string(), Vec::new()))
            }
        }
        _ => None,
    }
}

fn map_cpu(qemu_cmd: &QemuCmdModel) -> Option<(String, Vec<String>)> {
    let option = qemu_cmd.options_for_flag("-cpu").next()?;

    match &option.value {
        QemuCmdOptionValue::Csv(parts) => {
            let mut model = None;
            let mut features = Vec::new();

            for part in parts {
                match part {
                    QemuCsvPart::Bare(value) => {
                        if model.is_none() {
                            model = Some(value.clone());
                        } else {
                            features.push(value.clone());
                        }
                    }
                    QemuCsvPart::KeyValue { key, value } => {
                        features.push(format!("{key}={value}"));
                    }
                }
            }

            model.map(|resolved_model| (resolved_model, features))
        }
        QemuCmdOptionValue::Scalar(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some((trimmed.to_string(), Vec::new()))
            }
        }
        _ => None,
    }
}

fn map_netdev_definitions(
    qemu_cmd: &QemuCmdModel,
    warnings: &mut Vec<MappingWarning>,
) -> BTreeMap<String, NetdevDefinition> {
    let mut definitions = BTreeMap::new();

    for option in qemu_cmd.options_for_flag("-netdev") {
        let Some(parts) = csv_parts(option) else {
            warnings.push(MappingWarning {
                source_field: "-netdev".to_string(),
                kind: MappingWarningKind::UnsupportedValue,
                message: "expected CSV netdev payload".to_string(),
            });
            continue;
        };

        let id = csv_value(parts, "id").map(ToString::to_string);
        let backend_type = csv_value(parts, "type").map(ToString::to_string);

        let (Some(id), Some(backend_type)) = (id, backend_type) else {
            warnings.push(MappingWarning {
                source_field: "-netdev".to_string(),
                kind: MappingWarningKind::UnsupportedValue,
                message: "missing required netdev id/type fields".to_string(),
            });
            continue;
        };

        let vhost = csv_value(parts, "vhost").and_then(parse_on_off_bool);
        let queues = csv_value(parts, "queues").and_then(|v| v.parse::<u16>().ok());

        let mut extra = BTreeMap::new();
        for part in parts {
            if let QemuCsvPart::KeyValue { key, value } = part {
                if is_known_netdev_key(key) {
                    continue;
                }
                extra.insert(key.clone(), value.clone());
            }
        }

        let backend = NetworkBackendConfig {
            backend_type,
            ifname: csv_value(parts, "ifname").map(ToString::to_string),
            bridge: csv_value(parts, "br")
                .or_else(|| csv_value(parts, "bridge"))
                .map(ToString::to_string),
            script: csv_value(parts, "script").map(ToString::to_string),
            downscript: csv_value(parts, "downscript").map(ToString::to_string),
            helper: csv_value(parts, "helper").map(ToString::to_string),
            vhost,
            queues,
            hostfwd: csv_values(parts, "hostfwd"),
            listen: csv_value(parts, "listen").map(ToString::to_string),
            connect: csv_value(parts, "connect").map(ToString::to_string),
            fd: csv_value(parts, "fd").map(ToString::to_string),
            extra,
        };

        definitions.insert(id.clone(), NetdevDefinition { id, backend });
    }

    definitions
}

fn map_networks(
    qemu_cmd: &QemuCmdModel,
    netdev_definitions: &BTreeMap<String, NetdevDefinition>,
    warnings: &mut Vec<MappingWarning>,
) -> Vec<NetworkConfig> {
    let mut networks = Vec::new();

    for option in qemu_cmd.options_for_flag("-device") {
        let Some(parts) = csv_parts(option) else {
            continue;
        };

        let model = csv_first_bare(parts).map(ToString::to_string);
        let netdev_id = csv_value(parts, "netdev").map(ToString::to_string);

        let Some(netdev_id) = netdev_id else {
            continue;
        };

        let Some(model) = model else {
            warnings.push(MappingWarning {
                source_field: "-device".to_string(),
                kind: MappingWarningKind::AmbiguousPairing,
                message: format!("network device for netdev '{netdev_id}' is missing a model"),
            });
            continue;
        };

        if !is_supported_network_model(&model) {
            warnings.push(MappingWarning {
                source_field: "-device".to_string(),
                kind: MappingWarningKind::UnsupportedValue,
                message: format!(
                    "network model '{model}' for netdev '{netdev_id}' is not supported"
                ),
            });
            continue;
        }

        let Some(netdev_definition) = netdev_definitions.get(&netdev_id) else {
            warnings.push(MappingWarning {
                source_field: "-device".to_string(),
                kind: MappingWarningKind::AmbiguousPairing,
                message: format!("no matching -netdev definition found for id '{netdev_id}'"),
            });
            continue;
        };

        let id = csv_value(parts, "id")
            .map(ToString::to_string)
            .unwrap_or_else(|| netdev_definition.id.clone());

        let network = NetworkConfig {
            id,
            model,
            backend: Some(netdev_definition.backend.clone()),
            mac: csv_value(parts, "mac").map(ToString::to_string),
            rx_queue_size: csv_value(parts, "rx_queue_size").and_then(|v| v.parse().ok()),
            tx_queue_size: csv_value(parts, "tx_queue_size").and_then(|v| v.parse().ok()),
            boot_index: csv_value(parts, "bootindex").and_then(|v| v.parse().ok()),
            bus: csv_value(parts, "bus").map(ToString::to_string),
            addr: csv_value(parts, "addr").map(ToString::to_string),
        };

        networks.push(network);
    }

    networks
}

fn map_spice(
    qemu_cmd: &QemuCmdModel,
    warnings: &mut Vec<MappingWarning>,
) -> Option<crate::config::SpiceConfig> {
    let option = qemu_cmd.options_for_flag("-spice").next()?;
    let parts = csv_parts(option)?;

    let mut spice = crate::config::SpiceConfig {
        enabled: true,
        port: 5900,
        addr: "127.0.0.1".to_string(),
        disable_ticketing: false,
        audio: false,
        vdagent: true,
    };

    if let Some(port) = csv_value(parts, "port").and_then(|v| v.parse::<u16>().ok()) {
        spice.port = port;
    }
    if let Some(addr) = csv_value(parts, "addr") {
        spice.addr = addr.to_string();
    }
    if let Some(value) = csv_value(parts, "disable-ticketing") {
        spice.disable_ticketing = parse_on_off_bool(value).unwrap_or(false);
    }

    for part in parts {
        if let QemuCsvPart::KeyValue { key, value: _ } = part {
            if is_known_spice_key(key) {
                continue;
            }
            warnings.push(MappingWarning {
                source_field: "-spice".to_string(),
                kind: MappingWarningKind::UnsupportedValue,
                message: format!("unsupported spice key '{key}' was ignored"),
            });
        }
    }

    Some(spice)
}

fn collect_unsupported_flag_warnings(qemu_cmd: &QemuCmdModel, warnings: &mut Vec<MappingWarning>) {
    let supported_flags: BTreeSet<&str> = [
        "-name", "-machine", "-cpu", "-m", "-smp", "-netdev", "-device", "-spice", "-drive",
        "-smbios",
    ]
    .into_iter()
    .collect();

    for option in &qemu_cmd.options {
        if supported_flags.contains(option.flag.as_str()) {
            continue;
        }

        warnings.push(MappingWarning {
            source_field: option.flag.clone(),
            kind: MappingWarningKind::UnsupportedFlag,
            message: format!("option '{}' is not yet mapped", option.flag),
        });
    }
}

fn scalar_value(option: &QemuCmdOption) -> Option<&str> {
    match &option.value {
        QemuCmdOptionValue::Scalar(value) => Some(value.as_str()),
        _ => None,
    }
}

fn csv_parts(option: &QemuCmdOption) -> Option<&[QemuCsvPart]> {
    match &option.value {
        QemuCmdOptionValue::Csv(parts) => Some(parts.as_slice()),
        _ => None,
    }
}

fn csv_value<'a>(parts: &'a [QemuCsvPart], key: &str) -> Option<&'a str> {
    for part in parts {
        if let QemuCsvPart::KeyValue {
            key: part_key,
            value,
        } = part
            && part_key == key
        {
            return Some(value.as_str());
        }
    }
    None
}

fn csv_values(parts: &[QemuCsvPart], key: &str) -> Vec<String> {
    let mut values = Vec::new();
    for part in parts {
        if let QemuCsvPart::KeyValue {
            key: part_key,
            value,
        } = part
            && part_key == key
        {
            values.push(value.clone());
        }
    }
    values
}

fn csv_first_bare(parts: &[QemuCsvPart]) -> Option<&str> {
    for part in parts {
        if let QemuCsvPart::Bare(value) = part {
            return Some(value.as_str());
        }
    }
    None
}

fn parse_on_off_bool(value: &str) -> Option<bool> {
    match value {
        "on" | "yes" | "true" | "1" => Some(true),
        "off" | "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn is_supported_network_model(model: &str) -> bool {
    matches!(
        model,
        "virtio-net" | "virtio-net-pci" | "e1000" | "e1000e" | "rtl8139"
    )
}

fn is_known_netdev_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "id"
            | "ifname"
            | "br"
            | "bridge"
            | "script"
            | "downscript"
            | "helper"
            | "vhost"
            | "queues"
            | "hostfwd"
            | "listen"
            | "connect"
            | "fd"
    )
}

fn is_known_spice_key(key: &str) -> bool {
    matches!(key, "port" | "addr" | "disable-ticketing")
}

fn infer_profile_names(qemu_cmd: &QemuCmdModel) -> Vec<String> {
    let is_macos = has_macos_signals(qemu_cmd);
    let is_windows = !is_macos && has_windows_signals(qemu_cmd);
    let has_guest_agent = has_guest_agent_signal(qemu_cmd);
    let has_secure_boot = has_secure_boot_pflash_signal(qemu_cmd);
    let has_tpm = has_tpm_signal(qemu_cmd);

    let mut profiles = Vec::new();
    if is_macos {
        profiles.push("macos-kvm".to_string());
    } else if is_windows {
        profiles.push("windows-common".to_string());
        if has_secure_boot && has_tpm {
            profiles.push("windows-11".to_string());
        }
    } else if has_guest_agent {
        profiles.push("linux-l26-common".to_string());
    }

    profiles
}

fn has_windows_signals(qemu_cmd: &QemuCmdModel) -> bool {
    qemu_cmd.options_for_flag("-cpu").any(|option| {
        let Some(parts) = csv_parts(option) else {
            return false;
        };

        parts.iter().any(|part| match part {
            QemuCsvPart::Bare(value) => value.starts_with("hv_") || value == "kvm=off",
            QemuCsvPart::KeyValue { key, .. } => key.starts_with("hv_") || key == "kvm",
        })
    })
}

fn has_macos_signals(qemu_cmd: &QemuCmdModel) -> bool {
    let has_applesmc = qemu_cmd.options_for_flag("-device").any(|option| {
        csv_parts(option)
            .and_then(csv_first_bare)
            .is_some_and(|model| model == "isa-applesmc")
    });

    let has_smbios_type2 = qemu_cmd.options_for_flag("-smbios").any(|option| {
        let value = option
            .raw_value
            .as_deref()
            .or_else(|| scalar_value(option))
            .unwrap_or_default();
        value.split(',').any(|part| part.trim() == "type=2")
    });

    has_applesmc || has_smbios_type2
}

fn has_guest_agent_signal(qemu_cmd: &QemuCmdModel) -> bool {
    qemu_cmd.options_for_flag("-device").any(|option| {
        let Some(parts) = csv_parts(option) else {
            return false;
        };

        let model = csv_first_bare(parts);
        let channel_name = csv_value(parts, "name");
        model == Some("virtserialport") && channel_name == Some("org.qemu.guest_agent.0")
    })
}

fn has_secure_boot_pflash_signal(qemu_cmd: &QemuCmdModel) -> bool {
    qemu_cmd.options_for_flag("-drive").any(|option| {
        let value = option
            .raw_value
            .as_deref()
            .or_else(|| scalar_value(option))
            .unwrap_or_default();
        value.contains("OVMF_CODE_4M.secboot.fd") || value.contains("OVMF_CODE.secboot.fd")
    })
}

fn has_tpm_signal(qemu_cmd: &QemuCmdModel) -> bool {
    qemu_cmd.options_for_flag("-device").any(|option| {
        csv_parts(option)
            .and_then(csv_first_bare)
            .is_some_and(|model| model == "tpm-tis")
    })
}

#[cfg(test)]
mod tests {
    use super::{MappingWarningKind, map_qemu_cmd_to_canonical_yaml};
    use crate::import::common::validate::validate_generated_vm_yaml;
    use crate::import::qemu_cmd::parser::parse_qemu_cmd;

    fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
        let _guard = crate::test_support::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let old = std::env::var_os("EZKVM_CONFIG");
        let central_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_path);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));

        unsafe {
            match old {
                Some(value) => std::env::set_var("EZKVM_CONFIG", value),
                None => std::env::remove_var("EZKVM_CONFIG"),
            }
        }

        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    #[test]
    fn maps_core_options_and_validates() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -name 'vm-a,debug-threads=on' -machine type=q35,hpet=off -cpu host,hv_time -m 4096 -smp 4 -netdev type=bridge,id=net0,br=vmbr0 -device virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56,id=net0 -spice port=5903,addr=127.0.0.1,disable-ticketing=on",
        )
        .expect("parser should succeed");

        let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("mapping should succeed");

        assert!(mapped.warnings.is_empty());
        with_repo_profiles(|| {
            validate_generated_vm_yaml(&mapped.yaml).expect("mapped yaml should validate");
        });

        let config = with_repo_profiles(|| {
            crate::config::VmConfig::from_str(&mapped.yaml).expect("vm config parse")
        });
        assert_eq!(config.name, "vm-a");
        assert_eq!(config.system.machine, "q35");
        assert_eq!(config.system.memory.size, 4096);
        assert_eq!(config.system.cpu.vcpus, 4);
        assert_eq!(config.devices.networks.len(), 1);
        assert!(config.spice.is_some());
    }

    #[test]
    fn warns_on_unsupported_and_ambiguous_options() {
        let parsed =
            parse_qemu_cmd("/usr/bin/kvm -incoming defer -device virtio-net-pci,netdev=missing0")
                .expect("parser should succeed");

        let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("mapping should succeed");

        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.kind == MappingWarningKind::UnsupportedFlag)
        );
        assert!(
            mapped
                .warnings
                .iter()
                .any(|warning| warning.kind == MappingWarningKind::AmbiguousPairing)
        );
    }

    #[test]
    fn infers_windows_profiles_from_hyperv_tpm_and_secure_boot_signals() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -name win-vm -cpu host,hv_time,hv_vapic,hv_spinlocks=0x1fff,kvm=off -drive if=pflash,file=/usr/share/OVMF_CODE_4M.secboot.fd -device tpm-tis,tpmdev=tpm0",
        )
        .expect("parser should succeed");

        let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("mapping should succeed");
        let config = with_repo_profiles(|| {
            crate::config::VmConfig::from_str(&mapped.yaml).expect("yaml should deserialize")
        });

        assert!(config.profiles.contains(&"windows-common".to_string()));
        assert!(config.profiles.contains(&"windows-11".to_string()));
    }

    #[test]
    fn infers_linux_profile_from_guest_agent_without_windows_or_macos_signals() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -name linux-vm -cpu host,+kvm_pv_eoi -device virtio-serial,id=qga0 -device virtserialport,chardev=qga0,name=org.qemu.guest_agent.0",
        )
        .expect("parser should succeed");

        let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("mapping should succeed");
        let config = with_repo_profiles(|| {
            crate::config::VmConfig::from_str(&mapped.yaml).expect("yaml should deserialize")
        });

        assert!(config.profiles.contains(&"linux-l26-common".to_string()));
        assert!(!config.profiles.contains(&"windows-common".to_string()));
    }

    #[test]
    fn infers_macos_profile_with_applesmc_or_smbios_type_2() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -name macos-vm -device isa-applesmc,osk=<OSK> -smbios type=2 -cpu Penryn",
        )
        .expect("parser should succeed");

        let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("mapping should succeed");
        let config = with_repo_profiles(|| {
            crate::config::VmConfig::from_str(&mapped.yaml).expect("yaml should deserialize")
        });

        assert!(config.profiles.contains(&"macos-kvm".to_string()));
        assert!(!config.profiles.contains(&"windows-common".to_string()));
    }

    #[test]
    fn maps_representative_fixtures_and_validates_supported_shape() {
        let fixtures = [
            "/home/hurenkam/Workspace/ezkvm/input/felucia/108.qemu.cmd",
            "/home/hurenkam/Workspace/ezkvm/input/zbp-server-mh2/201.qemu.cmd",
            "/home/hurenkam/Workspace/ezkvm/input/coruscant/505.qemu.cmd",
        ];

        for fixture in fixtures {
            let input = std::fs::read_to_string(fixture).expect("fixture should read");
            let parsed = parse_qemu_cmd(&input).expect("fixture should parse");
            let mapped = map_qemu_cmd_to_canonical_yaml(&parsed).expect("fixture should map");

            with_repo_profiles(|| {
                validate_generated_vm_yaml(&mapped.yaml)
                    .expect("mapped fixture yaml should deserialize and validate");
            });
        }
    }
}
