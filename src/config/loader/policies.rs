use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::config::NetworkBackendConfig;

#[derive(Debug, Clone, Default, Deserialize)]
struct ProfilePolicies {
    #[serde(default)]
    drives: Vec<DrivePolicy>,

    #[serde(default)]
    networks: Vec<NetworkPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct DrivePolicy {
    #[serde(default, rename = "match")]
    selector: DrivePolicyMatch,

    #[serde(default)]
    defaults: Mapping,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DrivePolicyMatch {
    #[serde(default)]
    interface: Option<String>,

    #[serde(default, rename = "type")]
    drive_type: Option<String>,

    #[serde(default)]
    controller: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NetworkPolicy {
    #[serde(default, rename = "match")]
    selector: NetworkPolicyMatch,

    #[serde(default)]
    defaults: Mapping,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct NetworkPolicyMatch {
    #[serde(default)]
    model: Option<String>,

    #[serde(default)]
    backend_type: Option<String>,
}

pub(crate) fn apply_profile_policies(root: &mut Value) -> Result<()> {
    let Some(policies_value) = remove_top_level_key(root, "policies")? else {
        return Ok(());
    };

    let policies: ProfilePolicies = serde_yaml::from_value(policies_value)
        .context("Failed to parse top-level 'policies' section")?;

    apply_drive_policies(root, &policies.drives)?;
    apply_network_policies(root, &policies.networks)?;
    Ok(())
}

fn remove_top_level_key(root: &mut Value, key: &str) -> Result<Option<Value>> {
    let Value::Mapping(root_map) = root else {
        return Err(anyhow!(
            "Merged VM config must be a YAML mapping/object at the root"
        ));
    };

    Ok(root_map.remove(Value::String(key.to_string())))
}

fn apply_drive_policies(root: &mut Value, policies: &[DrivePolicy]) -> Result<()> {
    let Some(drives) = get_nested_sequence_mut(root, &["devices", "drives"])? else {
        return Ok(());
    };

    for drive in drives {
        let Value::Mapping(drive_map) = drive else {
            continue;
        };

        for policy in policies.iter().rev() {
            if drive_matches_policy(drive_map, &policy.selector) {
                merge_missing_mapping(drive_map, &policy.defaults);
            }
        }
    }

    Ok(())
}

fn apply_network_policies(root: &mut Value, policies: &[NetworkPolicy]) -> Result<()> {
    let Some(networks) = get_nested_sequence_mut(root, &["devices", "networks"])? else {
        return Ok(());
    };

    for network in networks {
        let Value::Mapping(network_map) = network else {
            continue;
        };

        for policy in policies.iter().rev() {
            if network_matches_policy(network_map, &policy.selector) {
                merge_missing_mapping(network_map, &policy.defaults);
            }
        }
    }

    Ok(())
}

fn get_nested_sequence_mut<'a>(
    root: &'a mut Value,
    path: &[&str],
) -> Result<Option<&'a mut Vec<Value>>> {
    let mut current = root;
    for segment in path {
        let Value::Mapping(map) = current else {
            return Ok(None);
        };

        let Some(next) = map.get_mut(Value::String((*segment).to_string())) else {
            return Ok(None);
        };
        current = next;
    }

    match current {
        Value::Sequence(sequence) => Ok(Some(sequence)),
        _ => Err(anyhow!(
            "Expected '{}' to be a YAML list/sequence",
            path.join(".")
        )),
    }
}

fn drive_matches_policy(drive_map: &Mapping, selector: &DrivePolicyMatch) -> bool {
    string_selector_matches(drive_map, "interface", selector.interface.as_deref())
        && string_selector_matches(drive_map, "type", selector.drive_type.as_deref())
        && string_selector_matches(drive_map, "controller", selector.controller.as_deref())
}

fn network_matches_policy(network_map: &Mapping, selector: &NetworkPolicyMatch) -> bool {
    string_selector_matches(network_map, "model", selector.model.as_deref())
        && optional_string_matches(
            extract_network_backend_type(network_map).as_deref(),
            selector.backend_type.as_deref(),
        )
}

fn string_selector_matches(map: &Mapping, field: &str, expected: Option<&str>) -> bool {
    let actual = map
        .get(Value::String(field.to_string()))
        .and_then(Value::as_str);
    optional_string_matches(actual, expected)
}

fn optional_string_matches(actual: Option<&str>, expected: Option<&str>) -> bool {
    match expected {
        Some(expected_value) => actual == Some(expected_value),
        None => true,
    }
}

fn extract_network_backend_type(network_map: &Mapping) -> Option<String> {
    let backend_key = Value::String("backend".to_string());
    if let Some(Value::Mapping(backend_map)) = network_map.get(&backend_key)
        && let Some(Value::String(backend_type)) =
            backend_map.get(Value::String("type".to_string()))
    {
        return Some(backend_type.clone());
    }

    let mode_key = Value::String("mode".to_string());
    let mode = network_map.get(&mode_key).and_then(Value::as_str)?;
    NetworkBackendConfig::from_legacy_mode(mode)
        .ok()
        .map(|backend| backend.backend_type)
}

fn merge_missing_mapping(target: &mut Mapping, defaults: &Mapping) {
    for (key, default_value) in defaults {
        match target.get_mut(key) {
            Some(existing) => merge_missing_value(existing, default_value),
            None => {
                target.insert(key.clone(), default_value.clone());
            }
        }
    }
}

fn merge_missing_value(target: &mut Value, default_value: &Value) {
    if let (Value::Mapping(target_map), Value::Mapping(default_map)) = (target, default_value) {
        merge_missing_mapping(target_map, default_map);
    }
}
