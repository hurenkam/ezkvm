use serde_yaml::Value;

pub(super) fn yaml_mapping_id(value: &Value) -> Option<String> {
    let Value::Mapping(map) = value else {
        return None;
    };
    let id_key = Value::String("id".to_string());
    match map.get(&id_key) {
        Some(Value::String(id)) if !id.trim().is_empty() => Some(id.clone()),
        _ => None,
    }
}

pub(super) fn yaml_mapping_name(value: &Value) -> Option<String> {
    let Value::Mapping(map) = value else {
        return None;
    };
    let name_key = Value::String("name".to_string());
    match map.get(&name_key) {
        Some(Value::String(name)) if !name.trim().is_empty() => Some(name.clone()),
        _ => None,
    }
}
