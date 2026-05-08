// Temporary over-size rationale (B-29): high-level import flow and its test suite
// remain in one module. Closure target is <=250 lines by extracting validation/
// write-output helpers and moving heavy test setup into dedicated test modules.
use super::{
    ImportError,
    mapper::{
        MappingWarning, map_proxmox_to_canonical_yaml, map_proxmox_to_canonical_yaml_with_storage,
    },
    model::ProxmoxVmConfig,
    parser::parse_proxmox_config,
    profile_compact::compact_profile_owned_fields,
    storage_parser::parse_proxmox_storage_config,
    yaml_compact::compact_sequence_mappings,
};
use crate::config::{VmConfig, validation};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportOutputMode {
    Canonical,
    #[default]
    Compact,
    DebugCanonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeTarget {
    /// Portable Linux target: host-independent semantics, no Proxmox paths
    #[default]
    PortableLinux,
    /// Proxmox Parity target: strict Proxmox runtime behavior (explicit opt-in)
    ProxmoxParity,
}

#[derive(Debug, Clone)]
pub struct ImportRunOptions {
    pub output_path: Option<String>,
    pub storage_path: Option<String>,
    pub strict: bool,
    pub dry_run: bool,
    pub compact_lists: bool,
    pub output_mode: ImportOutputMode,
    pub runtime_target: RuntimeTarget,
}

#[derive(Debug, Clone)]
pub struct ImportRunResult {
    pub output_path: String,
    pub yaml: String,
    pub warnings: Vec<MappingWarning>,
}

pub fn run_import_from_files(
    input_path: &str,
    options: &ImportRunOptions,
) -> Result<ImportRunResult, ImportError> {
    let input = std::fs::read_to_string(input_path).map_err(|e| {
        ImportError::ParseError(format!("unable to read input file '{}': {}", input_path, e))
    })?;

    let storage_config = if let Some(storage_path) = options.storage_path.as_deref() {
        let storage_text = std::fs::read_to_string(storage_path).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to read storage file '{}': {}",
                storage_path, e
            ))
        })?;
        Some(parse_proxmox_storage_config(&storage_text)?)
    } else {
        None
    };

    let parsed = parse_proxmox_config(&input)?;
    let mapped = match storage_config.as_ref() {
        Some(storage_config) => map_proxmox_to_canonical_yaml_with_storage(
            &parsed,
            Some(storage_config),
            options.runtime_target,
        )?,
        None => map_proxmox_to_canonical_yaml(&parsed, options.runtime_target)?,
    };

    let config = VmConfig::from_str(&mapped.yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "generated canonical YAML failed to deserialize or merge profiles: {e}"
        ))
    })?;
    validation::validate_config(&config).map_err(|e| {
        ImportError::ParseError(format!("generated canonical YAML failed validation: {e}"))
    })?;

    if options.strict && !mapped.warnings.is_empty() {
        return Err(ImportError::ParseError(format!(
            "strict import failed due to {} warning(s): {}",
            mapped.warnings.len(),
            format_warnings(&mapped.warnings)
        )));
    }

    let mut rendered_yaml = render_export_yaml(&parsed, &mapped.yaml, options)?;
    rendered_yaml = rewrite_drives_under_storage_controllers(&rendered_yaml)?;

    if options.compact_lists {
        rendered_yaml = compact_sequence_mappings(&rendered_yaml)?;
    }

    if options.output_mode == ImportOutputMode::DebugCanonical {
        rendered_yaml = format!(
            "{}\n{}",
            build_debug_source_comments(&parsed, &mapped.warnings),
            rendered_yaml
        );
    }

    let output_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(input_path));

    if !options.dry_run {
        std::fs::write(&output_path, &rendered_yaml).map_err(|e| {
            ImportError::ParseError(format!(
                "unable to write output file '{}': {}",
                output_path, e
            ))
        })?;
    }

    Ok(ImportRunResult {
        output_path,
        yaml: rendered_yaml,
        warnings: mapped.warnings,
    })
}

fn default_output_path(input_path: &str) -> String {
    let input = Path::new(input_path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("imported-vm");

    format!("{}.yaml", stem)
}

fn format_warnings(warnings: &[MappingWarning]) -> String {
    warnings
        .iter()
        .map(|warning| format!("{}: {}", warning.source_field, warning.message))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_export_yaml(
    parsed: &ProxmoxVmConfig,
    canonical_yaml: &str,
    options: &ImportRunOptions,
) -> Result<String, ImportError> {
    let mut rendered = match options.output_mode {
        ImportOutputMode::Canonical => canonical_yaml.to_string(),
        ImportOutputMode::Compact => compact_profile_owned_fields(canonical_yaml)?,
        ImportOutputMode::DebugCanonical => render_debug_canonical_yaml(parsed, canonical_yaml)?,
    };

    if options.output_mode == ImportOutputMode::Compact {
        rendered = omit_reconstructable_hostpci_bus_addr(&rendered, options.runtime_target)?;
        rendered = omit_default_spice_addr(&rendered)?;
    }

    Ok(rendered)
}

fn omit_default_spice_addr(input_yaml: &str) -> Result<String, ImportError> {
    use serde_yaml::Value;

    let mut root: Value = serde_yaml::from_str(input_yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse compact YAML for spice default omission: {e}"
        ))
    })?;

    let Value::Mapping(root_map) = &mut root else {
        return Ok(input_yaml.to_string());
    };

    let Some(spice_value) = root_map.get_mut(Value::String("spice".to_string())) else {
        return Ok(input_yaml.to_string());
    };
    let Some(spice_map) = spice_value.as_mapping_mut() else {
        return Ok(input_yaml.to_string());
    };

    let addr_key = Value::String("addr".to_string());
    let keep_default_addr = spice_map
        .get(&addr_key)
        .and_then(Value::as_str)
        .is_some_and(|addr| addr == "127.0.0.1");
    if keep_default_addr {
        spice_map.remove(&addr_key);
    }

    serde_yaml::to_string(&root).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to serialize compact YAML after spice default omission: {e}"
        ))
    })
}

fn omit_reconstructable_hostpci_bus_addr(
    input_yaml: &str,
    runtime_target: RuntimeTarget,
) -> Result<String, ImportError> {
    use serde_yaml::{Mapping, Value};
    use std::collections::HashMap;

    const MAX_Q35_HOSTPCI_ROOT_PORTS: usize = 8;

    #[derive(Clone)]
    struct HostPciView {
        device: String,
        id: String,
        pcie: bool,
        x_vga: bool,
        multifunction: bool,
    }

    #[derive(Clone)]
    struct SlotState {
        first_seen: usize,
        min_hostpci_index: usize,
        device_count: usize,
        should_assign_root_port: bool,
        has_multifunction_hint: bool,
    }

    #[derive(Clone)]
    struct SlotPlacement {
        bus: Option<String>,
        assign_function_addrs: bool,
    }

    fn hostpci_slot_key(device: &str) -> Option<&str> {
        device.rsplit_once('.').map(|(slot, _)| slot)
    }

    fn hostpci_function(device: &str) -> Option<u8> {
        let (_, slot_function) = device.rsplit_once(':')?;
        let (_, function) = slot_function.split_once('.')?;
        function.parse().ok()
    }

    fn hostpci_id_index(id: &str) -> Option<usize> {
        id.strip_prefix("hostpci")?.split('.').next()?.parse().ok()
    }

    fn map_bool(map: &Mapping, key: &str) -> bool {
        map.get(Value::String(key.to_string()))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    fn map_string(map: &Mapping, key: &str) -> Option<String> {
        map.get(Value::String(key.to_string()))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    if runtime_target != RuntimeTarget::PortableLinux {
        return Ok(input_yaml.to_string());
    }

    let mut root: Value = serde_yaml::from_str(input_yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse compact YAML for hostpci omission: {e}"
        ))
    })?;

    let Value::Mapping(root_map) = &mut root else {
        return Ok(input_yaml.to_string());
    };

    let readconfig_has_q35 = VmConfig::from_str(input_yaml).ok().is_some_and(|config| {
        config
            .system
            .readconfig
            .iter()
            .any(|path| path.contains("pve-q35") || path.contains("ezkvm-q35"))
    }) || root_map
        .get(Value::String("system".to_string()))
        .and_then(Value::as_mapping)
        .and_then(|system| system.get(Value::String("readconfig".to_string())))
        .and_then(Value::as_sequence)
        .is_some_and(|readconfig| {
            readconfig
                .iter()
                .filter_map(Value::as_str)
                .any(|path| path.contains("pve-q35") || path.contains("ezkvm-q35"))
        });

    if !readconfig_has_q35 {
        return Ok(input_yaml.to_string());
    }

    let Some(host_value) = root_map.get_mut(Value::String("host".to_string())) else {
        return Ok(input_yaml.to_string());
    };
    let Some(host) = host_value.as_mapping_mut() else {
        return Ok(input_yaml.to_string());
    };

    let Some(host_pci_value) = host.get_mut(Value::String("pci".to_string())) else {
        return Ok(input_yaml.to_string());
    };
    let Some(host_pci_seq) = host_pci_value.as_sequence_mut() else {
        return Ok(input_yaml.to_string());
    };

    let views = host_pci_seq
        .iter()
        .map(|item| {
            let map = item.as_mapping()?;
            Some(HostPciView {
                device: map_string(map, "device")?,
                id: map_string(map, "id").unwrap_or_default(),
                pcie: map_bool(map, "pcie"),
                x_vga: map_bool(map, "x_vga"),
                multifunction: map_bool(map, "multifunction"),
            })
        })
        .collect::<Option<Vec<_>>>();

    let Some(views) = views else {
        return Ok(input_yaml.to_string());
    };

    let mut states = HashMap::<String, SlotState>::new();
    for (index, device) in views.iter().enumerate() {
        let Some(slot) = hostpci_slot_key(&device.device) else {
            continue;
        };

        let state = states.entry(slot.to_string()).or_insert_with(|| SlotState {
            first_seen: index,
            min_hostpci_index: hostpci_id_index(&device.id).unwrap_or(index),
            device_count: 0,
            should_assign_root_port: false,
            has_multifunction_hint: false,
        });

        state.min_hostpci_index = state
            .min_hostpci_index
            .min(hostpci_id_index(&device.id).unwrap_or(index));
        state.device_count += 1;
        state.should_assign_root_port |= device.pcie || device.multifunction || device.x_vga;
        state.has_multifunction_hint |= device.multifunction || device.x_vga;
    }

    let mut ordered_slots = states
        .iter()
        .filter_map(|(slot, state)| {
            if state.should_assign_root_port {
                Some((state.min_hostpci_index, state.first_seen, slot.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ordered_slots.sort();

    let default_buses = ordered_slots
        .into_iter()
        .enumerate()
        .map(|(position, (_, _, slot))| {
            let bus = if position < MAX_Q35_HOSTPCI_ROOT_PORTS {
                format!("ich9-pcie-port-{}", position + 1)
            } else {
                "pcie.0".to_string()
            };
            (slot, bus)
        })
        .collect::<HashMap<_, _>>();

    let placements = states
        .into_iter()
        .map(|(slot, state)| {
            (
                slot.clone(),
                SlotPlacement {
                    bus: default_buses.get(&slot).cloned(),
                    assign_function_addrs: state.has_multifunction_hint || state.device_count > 1,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    for (idx, value) in host_pci_seq.iter_mut().enumerate() {
        let Some(map) = value.as_mapping_mut() else {
            continue;
        };

        let Some(slot) = hostpci_slot_key(&views[idx].device) else {
            continue;
        };
        let Some(placement) = placements.get(slot) else {
            continue;
        };

        let bus_key = Value::String("bus".to_string());
        let addr_key = Value::String("addr".to_string());

        let inferred_bus = placement.bus.as_deref();
        let current_bus = map.get(&bus_key).and_then(Value::as_str);
        if current_bus == inferred_bus {
            map.remove(&bus_key);
        }

        let inferred_addr = if placement.assign_function_addrs && inferred_bus.is_some() {
            hostpci_function(&views[idx].device).map(|function| format!("0x0.{function}"))
        } else {
            None
        };
        let current_addr = map.get(&addr_key).and_then(Value::as_str);
        if current_addr == inferred_addr.as_deref() {
            map.remove(&addr_key);
        }
    }

    serde_yaml::to_string(&root).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to serialize compact YAML after hostpci omission: {e}"
        ))
    })
}

fn render_debug_canonical_yaml(
    _parsed: &ProxmoxVmConfig,
    canonical_yaml: &str,
) -> Result<String, ImportError> {
    let mut config = serde_yaml::from_str::<VmConfig>(canonical_yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "generated canonical YAML failed to deserialize for debug export: {e}"
        ))
    })?;
    config.assign_default_device_ids();

    serde_yaml::to_string(&config).map_err(|e| {
        ImportError::ParseError(format!("failed to serialize debug-canonical YAML: {e}"))
    })
}

fn rewrite_drives_under_storage_controllers(input_yaml: &str) -> Result<String, ImportError> {
    use serde_yaml::{Mapping, Value};

    let parsed: Value = serde_yaml::from_str(input_yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse rendered YAML for B-35 reshape: {e}"
        ))
    })?;

    let mut root = match parsed {
        Value::Mapping(map) => map,
        _ => return Ok(input_yaml.to_string()),
    };

    let devices_key = Value::String("devices".to_string());
    let controllers_key = Value::String("controllers".to_string());
    let nested_controllers_key = Value::String("controllers".to_string());
    let drives_key = Value::String("drives".to_string());
    let scsi_key = Value::String("scsi".to_string());
    let sata_key = Value::String("sata".to_string());

    let mut devices_map = match root.remove(&devices_key) {
        Some(Value::Mapping(map)) => map,
        Some(other) => {
            root.insert(devices_key, other);
            return Ok(input_yaml.to_string());
        }
        None => return Ok(input_yaml.to_string()),
    };

    let mut controllers_map = match root.remove(&controllers_key) {
        Some(Value::Mapping(map)) => map,
        Some(other) => {
            root.insert(controllers_key, other);
            root.insert(devices_key, Value::Mapping(devices_map));
            return Ok(input_yaml.to_string());
        }
        // No controllers key in the compact YAML (e.g., IDE-only VMs after profile
        // compaction strips an empty controllers: {}). Proceed with an empty map so
        // IDE drives can still be nested under devices.controllers.ide.
        None => Mapping::new(),
    };

    let Some(Value::Sequence(drives)) = devices_map.remove(&drives_key) else {
        root.insert(devices_key, Value::Mapping(devices_map));
        root.insert(controllers_key, Value::Mapping(controllers_map));
        return serialize_vm_root_mapping(
            root,
            "failed to serialize rendered YAML for B-35 reshape",
        );
    };

    let mut scsi_controllers = match controllers_map.remove(&scsi_key) {
        Some(Value::Sequence(seq)) => seq,
        Some(other) => {
            controllers_map.insert(scsi_key.clone(), other);
            Vec::new()
        }
        None => Vec::new(),
    };
    let mut sata_controllers = match controllers_map.remove(&sata_key) {
        Some(Value::Sequence(seq)) => seq,
        Some(other) => {
            controllers_map.insert(sata_key.clone(), other);
            Vec::new()
        }
        None => Vec::new(),
    };

    let scsi_ids = controller_effective_ids(&scsi_controllers, "scsihw");
    let sata_ids = controller_effective_ids(&sata_controllers, "sata");

    let mut leftovers = Vec::new();

    for drive in drives {
        let Some((family, index, normalized_drive)) =
            assign_drive_to_controller(drive.clone(), &scsi_ids, &sata_ids)
        else {
            leftovers.push(drive);
            continue;
        };

        let controller_value = match family {
            "scsi" => scsi_controllers.get_mut(index),
            "sata" => sata_controllers.get_mut(index),
            _ => None,
        };

        let Some(Value::Mapping(controller_map)) = controller_value else {
            leftovers.push(normalized_drive);
            continue;
        };

        let drives_entry = controller_map
            .entry(Value::String("drives".to_string()))
            .or_insert_with(|| Value::Sequence(Vec::new()));

        if let Value::Sequence(items) = drives_entry {
            items.push(normalized_drive);
        } else {
            leftovers.push(normalized_drive);
        }
    }

    // Separate IDE drives from other non-controller drives.
    // The IDE controller is implicit in the chipset; group IDE drives under
    // devices.controllers.ide so they appear alongside scsi/sata controllers.
    let ide_key = Value::String("ide".to_string());
    let interface_key = Value::String("interface".to_string());
    let bus_key = Value::String("bus".to_string());
    let unit_key = Value::String("unit".to_string());
    let mut ide_drives = Vec::new();
    let mut other_leftovers = Vec::new();
    for drive in leftovers {
        let is_ide = if let Value::Mapping(ref map) = drive {
            map.get(interface_key.clone()).and_then(Value::as_str) == Some("ide")
        } else {
            false
        };
        if is_ide {
            if let Value::Mapping(mut map) = drive {
                map.remove(interface_key.clone());
                map.remove(bus_key.clone());
                map.remove(unit_key.clone());
                ide_drives.push(Value::Mapping(map));
            }
        } else {
            other_leftovers.push(drive);
        }
    }
    if !other_leftovers.is_empty() {
        devices_map.insert(drives_key, Value::Sequence(other_leftovers));
    }

    let mut nested_devices_controllers = Mapping::new();
    if !scsi_controllers.is_empty() {
        nested_devices_controllers.insert(scsi_key, Value::Sequence(scsi_controllers));
    }
    if !sata_controllers.is_empty() {
        nested_devices_controllers.insert(sata_key, Value::Sequence(sata_controllers));
    }
    if !ide_drives.is_empty() {
        let mut ide_controller = Mapping::new();
        ide_controller.insert(
            Value::String("drives".to_string()),
            Value::Sequence(ide_drives),
        );
        nested_devices_controllers.insert(
            ide_key,
            Value::Sequence(vec![Value::Mapping(ide_controller)]),
        );
    }
    if !nested_devices_controllers.is_empty() {
        devices_map.insert(
            nested_controllers_key,
            Value::Mapping(nested_devices_controllers),
        );
    }

    if !devices_map.is_empty() {
        root.insert(devices_key, Value::Mapping(devices_map));
    }
    if !controllers_map.is_empty() {
        root.insert(controllers_key, Value::Mapping(controllers_map));
    }

    serialize_vm_root_mapping(root, "failed to serialize rendered YAML for B-35 reshape")
}

fn serialize_vm_root_mapping(
    root: serde_yaml::Mapping,
    context: &str,
) -> Result<String, ImportError> {
    let ordered_root = reorder_vm_root_mapping(root);
    serde_yaml::to_string(&serde_yaml::Value::Mapping(ordered_root))
        .map_err(|e| ImportError::ParseError(format!("{}: {e}", context)))
}

fn reorder_vm_root_mapping(mut root: serde_yaml::Mapping) -> serde_yaml::Mapping {
    use serde_yaml::Value;

    const PREFERRED_ROOT_KEYS: &[&str] = &[
        "name",
        "backend",
        "profiles",
        "system",
        "devices",
        "controllers",
        "host",
        "spice",
        "vnc",
        "options",
        "hyperv",
        "iommu",
        "iscsi_disks",
    ];

    let mut ordered = serde_yaml::Mapping::new();
    for key in PREFERRED_ROOT_KEYS {
        let key_value = Value::String((*key).to_string());
        if let Some(value) = root.remove(&key_value) {
            ordered.insert(key_value, value);
        }
    }

    let mut remaining = root.into_iter().collect::<Vec<_>>();
    remaining.sort_by(|(left, _), (right, _)| {
        yaml_key_sort_label(left).cmp(&yaml_key_sort_label(right))
    });
    for (key, value) in remaining {
        ordered.insert(key, value);
    }

    ordered
}

fn yaml_key_sort_label(key: &serde_yaml::Value) -> String {
    match key {
        serde_yaml::Value::String(value) => format!("s:{value}"),
        _ => format!("o:{key:?}"),
    }
}

fn controller_effective_ids(controllers: &[serde_yaml::Value], prefix: &str) -> Vec<String> {
    use serde_yaml::Value;

    let id_key = Value::String("id".to_string());

    controllers
        .iter()
        .enumerate()
        .map(|(index, controller)| {
            if let Value::Mapping(map) = controller
                && let Some(Value::String(id)) = map.get(id_key.clone())
                && !id.trim().is_empty()
            {
                return id.to_string();
            }
            format!("{}{}", prefix, index)
        })
        .collect()
}

fn assign_drive_to_controller(
    drive: serde_yaml::Value,
    scsi_ids: &[String],
    sata_ids: &[String],
) -> Option<(&'static str, usize, serde_yaml::Value)> {
    use serde_yaml::Value;

    let interface_key = Value::String("interface".to_string());
    let controller_key = Value::String("controller".to_string());
    let bus_key = Value::String("bus".to_string());

    let Value::Mapping(mut drive_map) = drive else {
        return None;
    };

    let interface = drive_map
        .get(interface_key.clone())
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let explicit_controller = drive_map
        .get(controller_key.clone())
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let bus = drive_map
        .get(bus_key)
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let target = match interface.as_str() {
        "scsi" => {
            let controller_name = explicit_controller
                .clone()
                .or_else(|| (scsi_ids.len() == 1).then(|| scsi_ids[0].clone()))?;
            scsi_ids
                .iter()
                .position(|id| id == &controller_name)
                .map(|idx| ("scsi", idx))
        }
        "sata" => {
            let controller_name = explicit_controller
                .clone()
                .or_else(|| {
                    bus.as_deref()
                        .and_then(|value| value.split_once('.'))
                        .map(|(prefix, _)| prefix.to_string())
                })
                .or_else(|| (sata_ids.len() == 1).then(|| sata_ids[0].clone()))?;
            sata_ids
                .iter()
                .position(|id| id == &controller_name)
                .map(|idx| ("sata", idx))
        }
        _ => None,
    }?;

    drive_map.remove(controller_key);
    let expected_interface = target.0;
    if interface == expected_interface {
        drive_map.remove(interface_key);
    }

    Some((target.0, target.1, Value::Mapping(drive_map)))
}

fn build_debug_source_comments(parsed: &ProxmoxVmConfig, warnings: &[MappingWarning]) -> String {
    let mut lines = vec![
        "# debug-canonical export".to_string(),
        "# source: Proxmox config".to_string(),
    ];

    for key in [
        "name", "ostype", "machine", "bios", "cpu", "memory", "cores", "sockets", "agent", "args",
    ] {
        if let Some(value) = parsed.scalars.get(key) {
            lines.push(format!("# from Proxmox {key}: {value}"));
        }
    }

    if !warnings.is_empty() {
        lines.push(format!("# mapper warnings: {}", warnings.len()));
        for warning in warnings {
            lines.push(format!(
                "# warning {}: {}",
                warning.source_field, warning.message
            ));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        ImportRunOptions, RuntimeTarget, omit_reconstructable_hostpci_bus_addr,
        run_import_from_files,
    };
    use crate::test_support::env_lock;
    use serde_yaml::Value;
    use std::path::PathBuf;

    fn with_repo_profiles<T>(run: impl FnOnce() -> T) -> T {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let old = std::env::var_os("EZKVM_CONFIG");
        let central_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("etc/ezkvm.yaml");

        unsafe {
            std::env::set_var("EZKVM_CONFIG", &central_path);
        }

        let result = run();

        unsafe {
            match old {
                Some(value) => std::env::set_var("EZKVM_CONFIG", value),
                None => std::env::remove_var("EZKVM_CONFIG"),
            }
        }

        result
    }

    fn create_temp_test_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        dir.push(format!("ezkvm-{}-{}", name, nanos));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    #[test]
    fn dry_run_does_not_write_output_file() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-dry-run");
            let input_path = dir.join("100.conf");
            std::fs::write(
                &input_path,
                "name: vm-dry\nmemory: 2048\ncores: 2\nscsi0: local-lvm:disk0,size=10G\n",
            )
            .expect("write input");

            let output_path = dir.join("result.yaml");
            let options = ImportRunOptions {
                output_path: Some(output_path.to_string_lossy().to_string()),
                storage_path: None,
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: super::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(result.yaml.contains("name: vm-dry"));
            assert!(!output_path.exists());
        });
    }

    #[test]
    fn strict_mode_fails_when_warnings_exist() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-strict");
            let input_path = dir.join("101.conf");
            std::fs::write(
                &input_path,
                "name: vm-strict\narch: sparc\nmemory: 2048\ncores: 2\n",
            )
            .expect("write input");

            let options = ImportRunOptions {
                output_path: None,
                storage_path: None,
                strict: true,
                dry_run: true,
                compact_lists: false,
                output_mode: super::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            };

            let err = run_import_from_files(&input_path.to_string_lossy(), &options)
                .expect_err("must fail");
            assert!(err.to_string().contains("strict import failed"));
            assert!(err.to_string().contains("arch"));
        });
    }

    #[test]
    fn writes_output_file_when_not_dry_run() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-write");
            let input_path = dir.join("102.conf");
            std::fs::write(
                &input_path,
                "name: vm-write\nmemory: 2048\ncores: 2\nscsi0: local-lvm:disk0,size=10G\n",
            )
            .expect("write input");

            let output_path = dir.join("custom.yaml");
            let options = ImportRunOptions {
                output_path: Some(output_path.to_string_lossy().to_string()),
                storage_path: None,
                strict: false,
                dry_run: false,
                compact_lists: false,
                output_mode: super::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert_eq!(result.output_path, output_path.to_string_lossy());
            assert!(output_path.exists());
        });
    }

    #[test]
    fn resolves_storage_references_when_storage_cfg_is_provided() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-storage-resolution");
            let input_path = dir.join("108.conf");
            let storage_path = dir.join("storage.cfg");

            std::fs::write(
                &input_path,
                "name: vm-storage\nmemory: 2048\ncores: 2\nscsi0: vm1-pool:vm-108-boot,discard=on\n",
            )
            .expect("write input");
            std::fs::write(
                &storage_path,
                "lvmthin: vm1-pool\n    thinpool pool\n    vgname vm1\n    content images,rootdir\n",
            )
            .expect("write storage");

            let options = ImportRunOptions {
                output_path: None,
                storage_path: Some(storage_path.to_string_lossy().to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: super::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(result.yaml.contains("path: /dev/vm1/vm-108-boot"));
        });
    }

    #[test]
    fn resolves_zfspool_references_when_storage_cfg_is_provided() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-zfspool-resolution");
            let input_path = dir.join("500.conf");
            let storage_path = dir.join("storage.cfg");

            std::fs::write(
                &input_path,
                "name: vm-zfs\nmemory: 2048\ncores: 2\nscsi0: local-zfs:vm-500-disk-0,discard=on\n",
            )
            .expect("write input");
            std::fs::write(
                &storage_path,
                "zfspool: local-zfs\n    pool rpool/data\n    content images,rootdir\n",
            )
            .expect("write storage");

            let options = ImportRunOptions {
                output_path: None,
                storage_path: Some(storage_path.to_string_lossy().to_string()),
                strict: false,
                dry_run: true,
                compact_lists: false,
                output_mode: super::ImportOutputMode::Compact,
                runtime_target: RuntimeTarget::PortableLinux,
            };

            let result =
                run_import_from_files(&input_path.to_string_lossy(), &options).expect("import");
            assert!(
                result
                    .yaml
                    .contains("path: /dev/zvol/rpool/data/vm-500-disk-0")
            );
        });
    }

    #[test]
    fn canonical_and_compact_modes_render_different_yaml_shapes() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-output-mode-shapes");
            let input_path = dir.join("700.conf");
            std::fs::write(
                &input_path,
                "name: vm-mode\nostype: win11\nbios: ovmf\nmemory: 4096\ncores: 4\nscsi0: /var/lib/vm/disk.raw,format=raw\n",
            )
            .expect("write input");

            let canonical = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Canonical,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("canonical import");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import");

            assert_ne!(canonical.yaml, compact.yaml);
            assert!(canonical.yaml.contains("architecture:"));
        });
    }

    #[test]
    fn compact_mode_nests_scsi_drives_under_controllers_by_default() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-b35-scsi-layout");
            let input_path = dir.join("702.conf");
            std::fs::write(
                &input_path,
                "name: vm-b35\nmemory: 2048\ncores: 2\nscsi0: /var/lib/vm/disk.raw,format=raw\n",
            )
            .expect("write input");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices_key = Value::String("devices".to_string());
            let controllers_key = Value::String("controllers".to_string());
            let scsi_key = Value::String("scsi".to_string());
            let drives_key = Value::String("drives".to_string());
            let devices = root
                .as_mapping()
                .and_then(|map| map.get(devices_key.clone()))
                .and_then(Value::as_mapping)
                .expect("devices should be a mapping");
            let nested_scsi = devices
                .get(controllers_key)
                .and_then(Value::as_mapping)
                .and_then(|controllers| controllers.get(scsi_key))
                .and_then(Value::as_sequence)
                .expect("devices.controllers.scsi should exist");
            let first_scsi = nested_scsi
                .first()
                .and_then(Value::as_mapping)
                .expect("scsi controller should be mapping");
            let nested_drives = first_scsi
                .get(drives_key.clone())
                .and_then(Value::as_sequence)
                .expect("nested drives should exist");
            assert_eq!(nested_drives.len(), 1);

            let devices_drives = devices.get(drives_key).and_then(Value::as_sequence);
            assert!(
                devices_drives.is_none_or(|items| items.is_empty()),
                "controller-owned scsi drives should not remain in devices.drives"
            );
        });
    }

    #[test]
    fn compact_mode_keeps_non_controller_drives_in_devices_drives() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-b35-virtio-layout");
            let input_path = dir.join("703.conf");
            std::fs::write(
                &input_path,
                "name: vm-b35-virtio\nmemory: 2048\ncores: 2\nvirtio0: /var/lib/vm/disk.raw,format=raw\n",
            )
            .expect("write input");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices_key = Value::String("devices".to_string());
            let drives_key = Value::String("drives".to_string());
            let devices_drives = root
                .as_mapping()
                .and_then(|map| map.get(devices_key))
                .and_then(Value::as_mapping)
                .and_then(|devices| devices.get(drives_key))
                .and_then(Value::as_sequence)
                .expect("devices.drives should still exist for virtio drives");
            assert_eq!(devices_drives.len(), 1);
        });
    }

    #[test]
    fn compact_mode_nests_ide_drives_under_controllers() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-b35-ide-layout");
            let input_path = dir.join("704.conf");
            std::fs::write(
                &input_path,
                "name: vm-b35-ide\nmemory: 2048\ncores: 2\nide2: none,media=cdrom\n",
            )
            .expect("write input");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices_key = Value::String("devices".to_string());
            let controllers_key = Value::String("controllers".to_string());
            let ide_key = Value::String("ide".to_string());
            let drives_key = Value::String("drives".to_string());
            let path_key = Value::String("path".to_string());
            let interface_key = Value::String("interface".to_string());
            let bus_key = Value::String("bus".to_string());
            let unit_key = Value::String("unit".to_string());
            let devices = root
                .as_mapping()
                .and_then(|map| map.get(devices_key))
                .and_then(Value::as_mapping)
                .expect("devices should be a mapping");
            let nested_ide = devices
                .get(controllers_key)
                .and_then(Value::as_mapping)
                .and_then(|c| c.get(ide_key))
                .and_then(Value::as_sequence)
                .expect("devices.controllers.ide should exist");
            let first_ide = nested_ide
                .first()
                .and_then(Value::as_mapping)
                .expect("ide controller entry should be a mapping");
            let nested_drives = first_ide
                .get(drives_key.clone())
                .and_then(Value::as_sequence)
                .expect("ide controller should have nested drives");
            assert_eq!(nested_drives.len(), 1);
            let first_drive = nested_drives
                .first()
                .and_then(Value::as_mapping)
                .expect("ide drive should be a mapping");
            assert!(
                !first_drive.contains_key(path_key),
                "ide cdrom should omit empty path"
            );
            assert!(
                !first_drive.contains_key(interface_key),
                "ide nested drive should omit interface because container implies it"
            );
            assert!(
                !first_drive.contains_key(bus_key),
                "ide nested drive should omit bus because container implies it"
            );
            assert!(
                !first_drive.contains_key(unit_key),
                "ide nested drive should omit unit because container implies it"
            );

            let devices_drives = devices.get(drives_key).and_then(Value::as_sequence);
            assert!(
                devices_drives.is_none_or(|items| items.is_empty()),
                "ide drives should not remain in devices.drives"
            );
        });
    }

    #[test]
    fn debug_mode_includes_source_comments_and_deterministic_ids() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-output-mode-debug");
            let input_path = dir.join("701.conf");
            std::fs::write(
                &input_path,
                "name: vm-debug\nostype: win11\nbios: ovmf\nmemory: 4096\ncores: 4\nusb0: host=1-2.2\n",
            )
            .expect("write input");

            let canonical = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Canonical,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("canonical import");

            let debug = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::DebugCanonical,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("debug import");

            assert!(debug.yaml.contains("# from Proxmox ostype: win11"));
            // USB device is present in both modes and references xhci.0
            assert!(debug.yaml.contains("bus: xhci.0"));
            assert!(canonical.yaml.contains("bus: xhci.0"));
            // XHCI controller is not synthesized by mapper; comes from profiles/runtime
            assert!(!debug.yaml.contains("xhci:"));
            assert!(!canonical.yaml.contains("xhci:"));
        });
    }

    #[test]
    fn compact_mode_omits_reconstructable_hostpci_bus_and_addr() {
        with_repo_profiles(|| {
            let compact = run_import_from_files(
                "input/felucia/108.conf",
                &ImportRunOptions {
                    output_path: None,
                    storage_path: Some("input/felucia/storage.cfg".to_string()),
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import should succeed");

            let root: serde_yaml::Value =
                serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let host_pci = root
                .as_mapping()
                .and_then(|root| root.get(serde_yaml::Value::String("host".to_string())))
                .and_then(serde_yaml::Value::as_mapping)
                .and_then(|host| host.get(serde_yaml::Value::String("pci".to_string())))
                .and_then(serde_yaml::Value::as_sequence)
                .expect("host.pci should exist");

            let first = host_pci
                .first()
                .and_then(serde_yaml::Value::as_mapping)
                .expect("first hostpci should be mapping");
            let second = host_pci
                .get(1)
                .and_then(serde_yaml::Value::as_mapping)
                .expect("second hostpci should be mapping");
            assert!(!first.contains_key(serde_yaml::Value::String("bus".to_string())));
            assert!(!first.contains_key(serde_yaml::Value::String("addr".to_string())));
            assert!(!second.contains_key(serde_yaml::Value::String("bus".to_string())));
            assert!(!second.contains_key(serde_yaml::Value::String("addr".to_string())));
        });
    }

    #[test]
    fn compact_mode_keeps_nondefault_hostpci_bus_and_addr() {
        let input = r#"
name: vm
backend: qemu
system:
  machine: q35
  readconfig:
    - /usr/share/ezkvm/ezkvm-q35.cfg
host:
  pci:
    - id: hostpci0
      device: 0000:03:00.0
      pcie: true
      bus: ich9-pcie-port-3
      addr: 0x2.0
"#;

        let compacted = omit_reconstructable_hostpci_bus_addr(input, RuntimeTarget::PortableLinux)
            .expect("compact omission should succeed");
        let root: serde_yaml::Value = serde_yaml::from_str(&compacted).expect("yaml should parse");
        let first = root
            .as_mapping()
            .and_then(|root| root.get(serde_yaml::Value::String("host".to_string())))
            .and_then(serde_yaml::Value::as_mapping)
            .and_then(|host| host.get(serde_yaml::Value::String("pci".to_string())))
            .and_then(serde_yaml::Value::as_sequence)
            .and_then(|seq| seq.first())
            .and_then(serde_yaml::Value::as_mapping)
            .expect("first hostpci should exist");

        assert_eq!(
            first
                .get(serde_yaml::Value::String("bus".to_string()))
                .and_then(serde_yaml::Value::as_str),
            Some("ich9-pcie-port-3")
        );
        assert_eq!(
            first
                .get(serde_yaml::Value::String("addr".to_string()))
                .and_then(serde_yaml::Value::as_str),
            Some("0x2.0")
        );
    }

    #[test]
    fn compact_mode_omits_default_spice_addr() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-spice-default-addr");
            let input_path = dir.join("705.conf");
            std::fs::write(
                &input_path,
                "name: vm-spice-default\nmemory: 2048\ncores: 2\naudio0: device=ich9-intel-hda,driver=spice\n",
            )
            .expect("write input");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import should succeed");

            let root: serde_yaml::Value =
                serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let spice = root
                .as_mapping()
                .and_then(|map| map.get(serde_yaml::Value::String("spice".to_string())))
                .and_then(serde_yaml::Value::as_mapping)
                .expect("spice mapping should exist");

            assert!(
                !spice.contains_key(serde_yaml::Value::String("addr".to_string())),
                "compact YAML should omit default spice.addr"
            );
        });
    }

    #[test]
    fn compact_mode_keeps_nondefault_spice_addr() {
        with_repo_profiles(|| {
            let dir = create_temp_test_dir("import-spice-nondefault-addr");
            let input_path = dir.join("706.conf");
            std::fs::write(
                &input_path,
                "name: vm-spice-nondefault\nmemory: 2048\ncores: 2\nargs: -spice port=5903,addr=0.0.0.0,disable-ticketing=on\n",
            )
            .expect("write input");

            let compact = run_import_from_files(
                &input_path.to_string_lossy(),
                &ImportRunOptions {
                    output_path: None,
                    storage_path: None,
                    strict: false,
                    dry_run: true,
                    compact_lists: false,
                    output_mode: super::ImportOutputMode::Compact,
                    runtime_target: RuntimeTarget::PortableLinux,
                },
            )
            .expect("compact import should succeed");

            let root: serde_yaml::Value =
                serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let spice = root
                .as_mapping()
                .and_then(|map| map.get(serde_yaml::Value::String("spice".to_string())))
                .and_then(serde_yaml::Value::as_mapping)
                .expect("spice mapping should exist");

            assert_eq!(
                spice
                    .get(serde_yaml::Value::String("addr".to_string()))
                    .and_then(serde_yaml::Value::as_str),
                Some("0.0.0.0"),
                "compact YAML should keep non-default spice.addr"
            );
        });
    }
}
