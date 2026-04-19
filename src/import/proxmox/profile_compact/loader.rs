use super::super::ImportError;
use crate::config::CentralConfig;
use serde_yaml::{Mapping, Value};
use std::path::Path;

const DEFAULT_PROFILE_DIR_FALLBACK: &str = "/etc/ezkvm/profiles.d";

pub(super) fn extract_profile_names(vm_value: &Value) -> Result<Vec<String>, ImportError> {
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

pub(super) fn resolve_profile_dir() -> String {
    CentralConfig::load()
        .ok()
        .and_then(|cfg| cfg.profile_dir().map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_PROFILE_DIR_FALLBACK.to_string())
}

pub(super) fn load_merged_profiles(
    profile_dir: &str,
    profile_names: &[String],
) -> Result<Value, ImportError> {
    let mut merged = Value::Mapping(Mapping::new());

    for profile_name in profile_names {
        let profile_value = load_profile_value(profile_dir, profile_name)?;
        super::merge::merge_yaml_values(&mut merged, profile_value);
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
