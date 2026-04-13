use std::collections::HashMap;

pub(crate) fn merge_yaml_values(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
    merge_yaml_values_at_path(base, overlay, &[]);
}

fn merge_yaml_values_at_path(
    base: &mut serde_yaml::Value,
    overlay: serde_yaml::Value,
    path: &[String],
) {
    match (base, overlay) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                let key_name = match &key {
                    serde_yaml::Value::String(name) => Some(name.clone()),
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
                        && matches!(base_value, serde_yaml::Value::Sequence(_))
                        && matches!(overlay_value, serde_yaml::Value::Sequence(_))
                    {
                        merge_sequence_of_mappings_by_id(base_value, overlay_value, &child_path);
                    } else if is_append_all_list_path(&child_path)
                        && matches!(base_value, serde_yaml::Value::Sequence(_))
                        && matches!(overlay_value, serde_yaml::Value::Sequence(_))
                    {
                        merge_sequence_append_all(base_value, overlay_value);
                    } else if is_append_unique_list_path(&child_path)
                        && matches!(base_value, serde_yaml::Value::Sequence(_))
                        && matches!(overlay_value, serde_yaml::Value::Sequence(_))
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

fn merge_sequence_of_mappings_by_id(
    base: &mut serde_yaml::Value,
    overlay: serde_yaml::Value,
    path: &[String],
) {
    let (serde_yaml::Value::Sequence(base_seq), serde_yaml::Value::Sequence(mut overlay_seq)) =
        (base, overlay)
    else {
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
        let id = yaml_mapping_id(item).unwrap();
        index_by_id.insert(id, idx);
    }

    for overlay_item in overlay_seq.drain(..) {
        let id = yaml_mapping_id(&overlay_item).unwrap();

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

fn yaml_mapping_id(value: &serde_yaml::Value) -> Option<String> {
    let serde_yaml::Value::Mapping(map) = value else {
        return None;
    };
    let id_key = serde_yaml::Value::String("id".to_string());
    match map.get(&id_key) {
        Some(serde_yaml::Value::String(id)) if !id.trim().is_empty() => Some(id.clone()),
        _ => None,
    }
}

fn yaml_mapping_name(value: &serde_yaml::Value) -> Option<String> {
    let serde_yaml::Value::Mapping(map) = value else {
        return None;
    };
    let name_key = serde_yaml::Value::String("name".to_string());
    match map.get(&name_key) {
        Some(serde_yaml::Value::String(name)) if !name.trim().is_empty() => Some(name.clone()),
        _ => None,
    }
}

fn merge_sequence_append_unique(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
    let (serde_yaml::Value::Sequence(base_seq), serde_yaml::Value::Sequence(overlay_seq)) =
        (base, overlay)
    else {
        return;
    };

    for overlay_item in overlay_seq {
        let already_present = match &overlay_item {
            serde_yaml::Value::String(s) => base_seq
                .iter()
                .any(|existing| matches!(existing, serde_yaml::Value::String(es) if es == s)),
            serde_yaml::Value::Mapping(_) => {
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

fn merge_sequence_append_all(base: &mut serde_yaml::Value, overlay: serde_yaml::Value) {
    let (serde_yaml::Value::Sequence(base_seq), serde_yaml::Value::Sequence(overlay_seq)) =
        (base, overlay)
    else {
        return;
    };

    base_seq.extend(overlay_seq);
}
