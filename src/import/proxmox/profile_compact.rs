use super::ImportError;
use crate::config::CentralConfig;
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::path::Path;

const DEFAULT_PROFILE_DIR_FALLBACK: &str = "/etc/ezkvm/profiles.d";

pub fn compact_profile_owned_fields(yaml: &str) -> Result<String, ImportError> {
    let vm_value: Value = serde_yaml::from_str(yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse generated YAML for profile compaction: {e}"
        ))
    })?;

    let profile_names = extract_profile_names(&vm_value)?;
    if profile_names.is_empty() {
        return Ok(yaml.to_string());
    }

    let profile_dir = resolve_profile_dir();
    let base_profiles = load_merged_profiles(&profile_dir, &profile_names)?;

    let compacted_value = compact_overlay_against_base(&base_profiles, &vm_value, &[])
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));

    serde_yaml::to_string(&compacted_value).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to serialize compacted profile-aware YAML: {e}"
        ))
    })
}

fn extract_profile_names(vm_value: &Value) -> Result<Vec<String>, ImportError> {
    let Value::Mapping(vm_map) = vm_value else {
        return Ok(Vec::new());
    };

    let profiles_key = Value::String("profiles".to_string());
    let Some(profiles_value) = vm_map.get(&profiles_key) else {
        return Ok(Vec::new());
    };

    match profiles_value {
        Value::Sequence(items) => {
            let mut profile_names = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(name) if !name.trim().is_empty() => {
                        profile_names.push(name.to_string());
                    }
                    _ => {
                        return Err(ImportError::ParseError(
                            "generated YAML field 'profiles' must contain non-empty string names"
                                .to_string(),
                        ));
                    }
                }
            }
            Ok(profile_names)
        }
        Value::Null => Ok(Vec::new()),
        _ => Err(ImportError::ParseError(
            "generated YAML field 'profiles' must be a list of profile names".to_string(),
        )),
    }
}

fn resolve_profile_dir() -> String {
    CentralConfig::load()
        .ok()
        .and_then(|cfg| cfg.locations.profile_dir)
        .unwrap_or_else(|| DEFAULT_PROFILE_DIR_FALLBACK.to_string())
}

fn load_merged_profiles(profile_dir: &str, profile_names: &[String]) -> Result<Value, ImportError> {
    let mut merged = Value::Mapping(Mapping::new());

    for profile_name in profile_names {
        let profile_value = load_profile_value(profile_dir, profile_name)?;
        merge_yaml_values(&mut merged, profile_value);
    }

    Ok(merged)
}

fn load_profile_value(profile_dir: &str, profile_name: &str) -> Result<Value, ImportError> {
    let profile_path = Path::new(profile_dir).join(format!("{}.yaml", profile_name));
    let profile_content = std::fs::read_to_string(&profile_path).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to read profile '{}' from '{}': {}",
            profile_name,
            profile_path.display(),
            e
        ))
    })?;

    serde_yaml::from_str(&profile_content).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse profile '{}' from '{}': {}",
            profile_name,
            profile_path.display(),
            e
        ))
    })
}

fn compact_overlay_against_base(base: &Value, overlay: &Value, path: &[String]) -> Option<Value> {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            let mut kept = Mapping::new();

            for (key, overlay_value) in overlay_map {
                let key_name = match key {
                    Value::String(name) => Some(name.clone()),
                    _ => None,
                };
                let child_path = if let Some(name) = key_name {
                    let mut next = path.to_vec();
                    next.push(name);
                    next
                } else {
                    path.to_vec()
                };

                let keep_child = match base_map.get(key) {
                    Some(base_value) => compact_child_value(base_value, overlay_value, &child_path),
                    None => Some(overlay_value.clone()),
                };

                if let Some(value) = keep_child {
                    kept.insert(key.clone(), value);
                }
            }

            if kept.is_empty() {
                None
            } else {
                Some(Value::Mapping(kept))
            }
        }
        _ if base == overlay => None,
        _ => Some(overlay.clone()),
    }
}

fn compact_child_value(base: &Value, overlay: &Value, path: &[String]) -> Option<Value> {
    if matches!(base, Value::Sequence(_)) && matches!(overlay, Value::Sequence(_)) {
        if is_id_merge_list_path(path) {
            return compact_id_merge_sequence(base, overlay, path);
        }

        if is_append_unique_list_path(path) {
            return compact_append_unique_sequence(base, overlay);
        }

        if is_append_all_list_path(path) {
            return if base == overlay {
                None
            } else {
                Some(overlay.clone())
            };
        }
    }

    compact_overlay_against_base(base, overlay, path)
}

fn compact_id_merge_sequence(base: &Value, overlay: &Value, path: &[String]) -> Option<Value> {
    let (Value::Sequence(base_seq), Value::Sequence(overlay_seq)) = (base, overlay) else {
        return Some(overlay.clone());
    };

    if base_seq.iter().any(|item| yaml_mapping_id(item).is_none())
        || overlay_seq
            .iter()
            .any(|item| yaml_mapping_id(item).is_none())
    {
        return if base == overlay {
            None
        } else {
            Some(overlay.clone())
        };
    }

    let mut base_by_id: HashMap<String, &Value> = HashMap::new();
    for item in base_seq {
        let id = yaml_mapping_id(item).expect("id exists after guard");
        base_by_id.insert(id, item);
    }

    let mut kept = Vec::new();
    for overlay_item in overlay_seq {
        let id = yaml_mapping_id(overlay_item).expect("id exists after guard");

        if let Some(base_item) = base_by_id.get(&id) {
            match compact_overlay_against_base(base_item, overlay_item, path) {
                None => {}
                Some(Value::Mapping(mut diff_map)) => {
                    diff_map.insert(Value::String("id".to_string()), Value::String(id));
                    kept.push(Value::Mapping(diff_map));
                }
                Some(_) => kept.push(overlay_item.clone()),
            }
        } else {
            kept.push(overlay_item.clone());
        }
    }

    if kept.is_empty() {
        None
    } else {
        Some(Value::Sequence(kept))
    }
}

fn compact_append_unique_sequence(base: &Value, overlay: &Value) -> Option<Value> {
    let (Value::Sequence(base_seq), Value::Sequence(overlay_seq)) = (base, overlay) else {
        return Some(overlay.clone());
    };

    let mut kept = Vec::new();
    for overlay_item in overlay_seq {
        if !contains_append_unique_equivalent(base_seq, overlay_item) {
            kept.push(overlay_item.clone());
        }
    }

    if kept.is_empty() {
        None
    } else {
        Some(Value::Sequence(kept))
    }
}

fn contains_append_unique_equivalent(base_seq: &[Value], overlay_item: &Value) -> bool {
    match overlay_item {
        Value::String(s) => base_seq
            .iter()
            .any(|existing| matches!(existing, Value::String(es) if es == s)),
        Value::Mapping(_) => {
            if let Some(overlay_name) = yaml_mapping_name(overlay_item) {
                base_seq.iter().any(|existing| {
                    yaml_mapping_name(existing)
                        .map(|name| name == overlay_name)
                        .unwrap_or(false)
                })
            } else {
                base_seq.iter().any(|existing| existing == overlay_item)
            }
        }
        _ => base_seq.iter().any(|existing| existing == overlay_item),
    }
}

fn merge_yaml_values(base: &mut Value, overlay: Value) {
    merge_yaml_values_at_path(base, overlay, &[]);
}

fn merge_yaml_values_at_path(base: &mut Value, overlay: Value, path: &[String]) {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                let key_name = match &key {
                    Value::String(name) => Some(name.clone()),
                    _ => None,
                };

                if let Some(base_value) = base_map.get_mut(&key) {
                    let child_path = if let Some(name) = key_name {
                        let mut p = path.to_vec();
                        p.push(name);
                        p
                    } else {
                        path.to_vec()
                    };

                    if is_id_merge_list_path(&child_path)
                        && matches!(base_value, Value::Sequence(_))
                        && matches!(overlay_value, Value::Sequence(_))
                    {
                        merge_sequence_of_mappings_by_id(base_value, overlay_value, &child_path);
                    } else if is_append_all_list_path(&child_path)
                        && matches!(base_value, Value::Sequence(_))
                        && matches!(overlay_value, Value::Sequence(_))
                    {
                        merge_sequence_append_all(base_value, overlay_value);
                    } else if is_append_unique_list_path(&child_path)
                        && matches!(base_value, Value::Sequence(_))
                        && matches!(overlay_value, Value::Sequence(_))
                    {
                        merge_sequence_append_unique(base_value, overlay_value);
                    } else {
                        merge_yaml_values_at_path(base_value, overlay_value, &child_path);
                    }
                } else {
                    base_map.insert(key, overlay_value);
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn is_id_merge_list_path(path: &[String]) -> bool {
    matches!(path, [first, second] if first == "host" && second == "pci")
        || matches!(path, [first, second] if first == "host" && second == "usb")
        || matches!(path, [first, second] if first == "controllers" && second == "scsi")
        || matches!(path, [first, second] if first == "controllers" && second == "xhci")
        || matches!(path, [first, second] if first == "devices" && second == "audio")
}

fn is_append_unique_list_path(path: &[String]) -> bool {
    matches!(path, [first, second, third] if first == "system" && second == "cpu" && third == "features")
        || matches!(path, [first, second] if first == "system" && second == "machine_options")
        || matches!(path, [first, second] if first == "options" && second == "global_options")
}

fn is_append_all_list_path(path: &[String]) -> bool {
    matches!(path, [first, second] if first == "devices" && second == "drives")
        || matches!(path, [first, second] if first == "devices" && second == "networks")
        || matches!(path, [first, second] if first == "policies" && second == "drives")
        || matches!(path, [first, second] if first == "policies" && second == "networks")
        || matches!(path, [first, second] if first == "policies" && second == "displays")
        || matches!(path, [first, second] if first == "policies" && second == "serials")
        || matches!(path, [first, second] if first == "policies" && second == "hostpci")
        || matches!(path, [first, second] if first == "policies" && second == "usb_devices")
        || matches!(path, [first, second] if first == "policies" && second == "xhci_controllers")
        || matches!(path, [first, second] if first == "policies" && second == "audio_devices")
        || matches!(path, [first, second] if first == "policies" && second == "scsi_controllers")
        || matches!(path, [first, second] if first == "policies" && second == "iscsi_disks")
}

fn merge_sequence_of_mappings_by_id(base: &mut Value, overlay: Value, path: &[String]) {
    let (Value::Sequence(base_seq), Value::Sequence(mut overlay_seq)) = (base, overlay) else {
        return;
    };

    if base_seq.iter().any(|item| yaml_mapping_id(item).is_none())
        || overlay_seq
            .iter()
            .any(|item| yaml_mapping_id(item).is_none())
    {
        *base_seq = overlay_seq;
        return;
    }

    let mut index_by_id: HashMap<String, usize> = HashMap::new();
    for (idx, item) in base_seq.iter().enumerate() {
        let id = yaml_mapping_id(item).expect("id exists after guard");
        index_by_id.insert(id, idx);
    }

    for overlay_item in overlay_seq.drain(..) {
        let id = yaml_mapping_id(&overlay_item).expect("id exists after guard");

        if let Some(base_idx) = index_by_id.get(&id).copied() {
            if let Some(base_item) = base_seq.get_mut(base_idx) {
                merge_yaml_values_at_path(base_item, overlay_item, path);
            }
        } else {
            let next_idx = base_seq.len();
            base_seq.push(overlay_item);
            index_by_id.insert(id, next_idx);
        }
    }
}

fn yaml_mapping_id(value: &Value) -> Option<String> {
    let Value::Mapping(map) = value else {
        return None;
    };
    let id_key = Value::String("id".to_string());
    match map.get(&id_key) {
        Some(Value::String(id)) if !id.trim().is_empty() => Some(id.clone()),
        _ => None,
    }
}

fn yaml_mapping_name(value: &Value) -> Option<String> {
    let Value::Mapping(map) = value else {
        return None;
    };
    let name_key = Value::String("name".to_string());
    match map.get(&name_key) {
        Some(Value::String(name)) if !name.trim().is_empty() => Some(name.clone()),
        _ => None,
    }
}

fn merge_sequence_append_unique(base: &mut Value, overlay: Value) {
    let (Value::Sequence(base_seq), Value::Sequence(overlay_seq)) = (base, overlay) else {
        return;
    };

    for overlay_item in overlay_seq {
        let already_present = match &overlay_item {
            Value::String(s) => base_seq
                .iter()
                .any(|existing| matches!(existing, Value::String(es) if es == s)),
            Value::Mapping(_) => {
                if let Some(overlay_name) = yaml_mapping_name(&overlay_item) {
                    base_seq.iter().any(|existing| {
                        yaml_mapping_name(existing)
                            .map(|name| name == overlay_name)
                            .unwrap_or(false)
                    })
                } else {
                    base_seq.iter().any(|existing| existing == &overlay_item)
                }
            }
            _ => base_seq.iter().any(|existing| existing == &overlay_item),
        };

        if !already_present {
            base_seq.push(overlay_item);
        }
    }
}

fn merge_sequence_append_all(base: &mut Value, overlay: Value) {
    let (Value::Sequence(base_seq), Value::Sequence(overlay_seq)) = (base, overlay) else {
        return;
    };

    base_seq.extend(overlay_seq);
}

#[cfg(test)]
mod tests {
    use super::compact_profile_owned_fields;
    use crate::test_support::env_lock;
    use serde_yaml::Value;
    use std::path::PathBuf;

    fn with_test_profiles<T>(run: impl FnOnce(PathBuf) -> T) -> T {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let root = std::env::temp_dir().join(format!(
            "ezkvm-profile-compact-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let profile_dir = root.join("profiles");
        std::fs::create_dir_all(&profile_dir).expect("create profile dir");

        let central = root.join("ezkvm.yaml");
        std::fs::write(
            &central,
            format!("locations:\n  profile_dir: {}\n", profile_dir.display()),
        )
        .expect("write central config");

        let old = std::env::var_os("EZKVM_CONFIG");
        unsafe { std::env::set_var("EZKVM_CONFIG", &central) };

        let result = run(profile_dir);

        unsafe {
            match old {
                Some(value) => std::env::set_var("EZKVM_CONFIG", value),
                None => std::env::remove_var("EZKVM_CONFIG"),
            }
        }

        result
    }

    #[test]
    fn removes_redundant_mapping_fields_owned_by_profiles() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "options:\n  rtc:\n    base: localtime\n    driftfix: slew\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
system:
  architecture: x86_64
options:
  rtc:
    base: localtime
    driftfix: slew
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let map = value.as_mapping().expect("root mapping");
            assert!(map.contains_key(Value::String("profiles".to_string())));
            assert!(map.contains_key(Value::String("system".to_string())));
            assert!(!map.contains_key(Value::String("options".to_string())));
        });
    }

    #[test]
    fn keeps_vm_specific_values_when_they_differ_from_profiles() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "options:\n  rtc:\n    base: localtime\n    driftfix: slew\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
options:
  rtc:
    base: utc
    driftfix: slew
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let rtc_base = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("options".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("rtc".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("base".to_string())))
                .and_then(Value::as_str);

            assert_eq!(rtc_base, Some("utc"));
        });
    }

    #[test]
    fn compacts_id_merge_lists_by_keeping_only_item_differences() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("gpu-passthrough.yaml"),
                "controllers:\n  xhci:\n    - id: xhci\n      p2: 15\n      p3: 15\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - gpu-passthrough
controllers:
  xhci:
    - id: xhci
      p2: 15
      p3: 15
      bus: pci.1
      addr: "0x1b"
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let xhci = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("controllers".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("xhci".to_string())))
                .and_then(Value::as_sequence)
                .expect("xhci sequence should be present");

            let item = xhci
                .first()
                .and_then(Value::as_mapping)
                .expect("xhci item mapping");

            assert_eq!(
                item.get(Value::String("id".to_string()))
                    .and_then(Value::as_str),
                Some("xhci")
            );
            assert_eq!(
                item.get(Value::String("bus".to_string()))
                    .and_then(Value::as_str),
                Some("pci.1")
            );
            assert_eq!(
                item.get(Value::String("addr".to_string()))
                    .and_then(Value::as_str),
                Some("0x1b")
            );
            assert!(!item.contains_key(Value::String("p2".to_string())));
            assert!(!item.contains_key(Value::String("p3".to_string())));
        });
    }

    #[test]
    fn compacts_append_unique_lists_to_only_new_entries() {
        with_test_profiles(|profile_dir| {
            std::fs::write(
                profile_dir.join("windows-common.yaml"),
                "system:\n  cpu:\n    features:\n      - +kvm_pv_eoi\n      - +kvm_pv_unhalt\n",
            )
            .expect("write profile");

            let input = r#"
name: vm
backend: qemu
profiles:
  - windows-common
system:
  cpu:
    features:
      - +kvm_pv_eoi
      - +kvm_pv_unhalt
      - +svm
"#;

            let compacted = compact_profile_owned_fields(input).expect("compact");
            let value: Value = serde_yaml::from_str(&compacted).expect("parse compacted");

            let features = value
                .as_mapping()
                .and_then(|m| m.get(Value::String("system".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("cpu".to_string())))
                .and_then(Value::as_mapping)
                .and_then(|m| m.get(Value::String("features".to_string())))
                .and_then(Value::as_sequence)
                .expect("features sequence should remain with new entries");

            assert_eq!(features.len(), 1);
            assert_eq!(features[0].as_str(), Some("+svm"));
        });
    }
}
