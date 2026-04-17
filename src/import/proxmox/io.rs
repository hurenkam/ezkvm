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

#[derive(Debug, Clone)]
pub struct ImportRunOptions {
    pub output_path: Option<String>,
    pub storage_path: Option<String>,
    pub strict: bool,
    pub dry_run: bool,
    pub compact_lists: bool,
    pub output_mode: ImportOutputMode,
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
        Some(storage_config) => {
            map_proxmox_to_canonical_yaml_with_storage(&parsed, Some(storage_config))?
        }
        None => map_proxmox_to_canonical_yaml(&parsed)?,
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
    let rendered = match options.output_mode {
        ImportOutputMode::Canonical => canonical_yaml.to_string(),
        ImportOutputMode::Compact => compact_profile_owned_fields(canonical_yaml)?,
        ImportOutputMode::DebugCanonical => render_debug_canonical_yaml(parsed, canonical_yaml)?,
    };

    Ok(rendered)
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
        ImportError::ParseError(format!(
            "failed to serialize debug-canonical YAML: {e}"
        ))
    })
}

fn rewrite_drives_under_storage_controllers(input_yaml: &str) -> Result<String, ImportError> {
    use serde_yaml::{Mapping, Value};

    let parsed: Value = serde_yaml::from_str(input_yaml).map_err(|e| {
        ImportError::ParseError(format!("failed to parse rendered YAML for B-35 reshape: {e}"))
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
        None => {
            root.insert(devices_key, Value::Mapping(devices_map));
            return Ok(input_yaml.to_string());
        }
    };

    let Some(Value::Sequence(drives)) = devices_map.remove(&drives_key) else {
        root.insert(devices_key, Value::Mapping(devices_map));
        root.insert(controllers_key, Value::Mapping(controllers_map));
        return serde_yaml::to_string(&Value::Mapping(root)).map_err(|e| {
            ImportError::ParseError(format!("failed to serialize rendered YAML for B-35 reshape: {e}"))
        });
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
    let mut ide_drives = Vec::new();
    let mut other_leftovers = Vec::new();
    for drive in leftovers {
        let is_ide = if let Value::Mapping(ref map) = drive {
            map.get(&Value::String("interface".to_string()))
                .and_then(Value::as_str)
                .map_or(false, |iface| iface == "ide")
        } else {
            false
        };
        if is_ide {
            if let Value::Mapping(mut map) = drive {
                map.remove(&Value::String("interface".to_string()));
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
        devices_map.insert(nested_controllers_key, Value::Mapping(nested_devices_controllers));
    }

    if !devices_map.is_empty() {
        root.insert(devices_key, Value::Mapping(devices_map));
    }
    if !controllers_map.is_empty() {
        root.insert(controllers_key, Value::Mapping(controllers_map));
    }

    serde_yaml::to_string(&Value::Mapping(root)).map_err(|e| {
        ImportError::ParseError(format!("failed to serialize rendered YAML for B-35 reshape: {e}"))
    })
}

fn controller_effective_ids(controllers: &[serde_yaml::Value], prefix: &str) -> Vec<String> {
    use serde_yaml::Value;

    controllers
        .iter()
        .enumerate()
        .map(|(index, controller)| {
            if let Value::Mapping(map) = controller
                && let Some(Value::String(id)) = map.get(&Value::String("id".to_string()))
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

    let Value::Mapping(mut drive_map) = drive else {
        return None;
    };

    let interface = drive_map
        .get(&Value::String("interface".to_string()))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let explicit_controller = drive_map
        .get(&Value::String("controller".to_string()))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let bus = drive_map
        .get(&Value::String("bus".to_string()))
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

    drive_map.remove(&Value::String("controller".to_string()));
    let expected_interface = target.0;
    if interface == expected_interface {
        drive_map.remove(&Value::String("interface".to_string()));
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
    use super::{ImportRunOptions, run_import_from_files};
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
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices = root
                .as_mapping()
                .and_then(|map| map.get(&Value::String("devices".to_string())))
                .and_then(Value::as_mapping)
                .expect("devices should be a mapping");
            let nested_scsi = devices
                .get(&Value::String("controllers".to_string()))
                .and_then(Value::as_mapping)
                .and_then(|controllers| controllers.get(&Value::String("scsi".to_string())))
                .and_then(Value::as_sequence)
                .expect("devices.controllers.scsi should exist");
            let first_scsi = nested_scsi
                .first()
                .and_then(Value::as_mapping)
                .expect("scsi controller should be mapping");
            let nested_drives = first_scsi
                .get(&Value::String("drives".to_string()))
                .and_then(Value::as_sequence)
                .expect("nested drives should exist");
            assert_eq!(nested_drives.len(), 1);

            let devices_drives = devices
                .get(&Value::String("drives".to_string()))
                .and_then(Value::as_sequence);
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
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices_drives = root
                .as_mapping()
                .and_then(|map| map.get(&Value::String("devices".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|devices| devices.get(&Value::String("drives".to_string())))
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
                },
            )
            .expect("compact import");

            let root: Value = serde_yaml::from_str(&compact.yaml).expect("yaml should parse");
            let devices = root
                .as_mapping()
                .and_then(|map| map.get(&Value::String("devices".to_string())))
                .and_then(Value::as_mapping)
                .expect("devices should be a mapping");
            let nested_ide = devices
                .get(&Value::String("controllers".to_string()))
                .and_then(Value::as_mapping)
                .and_then(|c| c.get(&Value::String("ide".to_string())))
                .and_then(Value::as_sequence)
                .expect("devices.controllers.ide should exist");
            let first_ide = nested_ide
                .first()
                .and_then(Value::as_mapping)
                .expect("ide controller entry should be a mapping");
            let nested_drives = first_ide
                .get(&Value::String("drives".to_string()))
                .and_then(Value::as_sequence)
                .expect("ide controller should have nested drives");
            assert_eq!(nested_drives.len(), 1);
            let first_drive = nested_drives
                .first()
                .and_then(Value::as_mapping)
                .expect("ide drive should be a mapping");
            assert!(
                !first_drive.contains_key(&Value::String("path".to_string())),
                "ide cdrom should omit empty path"
            );
            assert!(
                !first_drive.contains_key(&Value::String("interface".to_string())),
                "ide nested drive should omit interface because container implies it"
            );

            let devices_drives = devices
                .get(&Value::String("drives".to_string()))
                .and_then(Value::as_sequence);
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
                },
            )
            .expect("debug import");

            assert!(debug.yaml.contains("# from Proxmox ostype: win11"));
            assert!(debug.yaml.contains("id: xhci"));
            assert!(!canonical.yaml.contains("id: xhci"));
        });
    }
}
