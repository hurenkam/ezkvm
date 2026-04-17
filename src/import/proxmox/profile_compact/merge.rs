use serde_yaml::Value;
use std::collections::HashMap;

// B-34 ownership boundaries (merge side):
// - id-merge lists: host.pci, host.usb, controllers.{scsi,sata,xhci}, devices.audio
//   Example: controllers.scsi items merge by id, preserving profile defaults per controller id.
// - append-unique lists: system.cpu.features, system.machine_options, devices.input,
//   options.global_options.
// - append-all lists: devices.{drives,networks} and policies.* list families.
//
// These boundaries are intentionally mirrored by compaction rules so profile-stack
// merge remains the inverse of compact output.

pub(super) fn merge_yaml_values(base: &mut Value, overlay: Value) {
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

                    if super::paths::is_id_merge_list_path(&child_path)
                        && matches!(base_value, Value::Sequence(_))
                        && matches!(overlay_value, Value::Sequence(_))
                    {
                        merge_sequence_of_mappings_by_id(base_value, overlay_value, &child_path);
                    } else if super::paths::is_append_all_list_path(&child_path)
                        && matches!(base_value, Value::Sequence(_))
                        && matches!(overlay_value, Value::Sequence(_))
                    {
                        merge_sequence_append_all(base_value, overlay_value);
                    } else if super::paths::is_append_unique_list_path(&child_path)
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

fn merge_sequence_of_mappings_by_id(base: &mut Value, overlay: Value, path: &[String]) {
    let (Value::Sequence(base_seq), Value::Sequence(mut overlay_seq)) = (base, overlay) else {
        return;
    };

    if base_seq
        .iter()
        .any(|item| super::yaml::yaml_mapping_id(item).is_none())
        || overlay_seq
            .iter()
            .any(|item| super::yaml::yaml_mapping_id(item).is_none())
    {
        *base_seq = overlay_seq;
        return;
    }

    let mut index_by_id: HashMap<String, usize> = HashMap::new();
    for (idx, item) in base_seq.iter().enumerate() {
        let id = super::yaml::yaml_mapping_id(item).expect("id exists after guard");
        index_by_id.insert(id, idx);
    }

    for overlay_item in overlay_seq.drain(..) {
        let id = super::yaml::yaml_mapping_id(&overlay_item).expect("id exists after guard");

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
                if let Some(overlay_name) = super::yaml::yaml_mapping_name(&overlay_item) {
                    base_seq.iter().any(|existing| {
                        super::yaml::yaml_mapping_name(existing)
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
