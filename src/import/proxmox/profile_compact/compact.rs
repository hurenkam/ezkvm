use serde_yaml::{Mapping, Value};
use std::collections::HashMap;

pub(super) fn compact_overlay_against_base(
    base: &Value,
    overlay: &Value,
    path: &[String],
) -> Option<Value> {
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
        if super::paths::is_id_merge_list_path(path) {
            return compact_id_merge_sequence(base, overlay, path);
        }

        if super::paths::is_append_unique_list_path(path) {
            return compact_append_unique_sequence(base, overlay);
        }

        if super::paths::is_append_all_list_path(path) {
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

    if base_seq.iter().any(|item| super::yaml::yaml_mapping_id(item).is_none())
        || overlay_seq
            .iter()
            .any(|item| super::yaml::yaml_mapping_id(item).is_none())
    {
        return if base == overlay {
            None
        } else {
            Some(overlay.clone())
        };
    }

    let mut base_by_id: HashMap<String, &Value> = HashMap::new();
    for item in base_seq {
        let id = super::yaml::yaml_mapping_id(item).expect("id exists after guard");
        base_by_id.insert(id, item);
    }

    let mut kept = Vec::new();
    for overlay_item in overlay_seq {
        let id = super::yaml::yaml_mapping_id(overlay_item).expect("id exists after guard");

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
            if let Some(overlay_name) = super::yaml::yaml_mapping_name(overlay_item) {
                base_seq.iter().any(|existing| {
                    super::yaml::yaml_mapping_name(existing)
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
