use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashMap};

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

    #[serde(default)]
    placement: DrivePlacementPolicy,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DrivePlacementPolicy {
    #[serde(default)]
    scsi_id: Option<ScsiIdPlacementConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScsiIdPlacementConfig {
    scope: String,
    start: u32,
    #[serde(default = "default_step")]
    step: u32,
    #[serde(default)]
    controller: Option<String>,
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

    #[serde(default)]
    placement: NetworkPlacementPolicy,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct NetworkPlacementPolicy {
    #[serde(default)]
    addr: Option<AddrPlacementConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct AddrPlacementConfig {
    scope: String,
    start: String,
    #[serde(default = "default_step")]
    step: u32,
    #[serde(default)]
    bus: Option<String>,
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
    apply_drive_placement_policies(root, &policies.drives)?;
    apply_network_placement_policies(root, &policies.networks)?;
    Ok(())
}

fn default_step() -> u32 {
    1
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

fn apply_drive_placement_policies(root: &mut Value, policies: &[DrivePolicy]) -> Result<()> {
    let Some(drives) = get_nested_sequence_mut(root, &["devices", "drives"])? else {
        return Ok(());
    };

    let snapshot = drives.clone();
    let mut used_by_scope: HashMap<String, BTreeSet<u32>> = HashMap::new();

    for drive in drives {
        let Value::Mapping(drive_map) = drive else {
            continue;
        };

        if drive_map.contains_key(Value::String("scsi_id".to_string())) {
            continue;
        }

        let Some(placement) = policies.iter().rev().find_map(|policy| {
            if drive_matches_policy(drive_map, &policy.selector) {
                policy.placement.scsi_id.as_ref()
            } else {
                None
            }
        }) else {
            continue;
        };

        let scope_key = resolve_scsi_scope_key(drive_map, placement)?;

        if placement.step == 0 {
            return Err(anyhow!(
                "policies.drives[].placement.scsi_id.step must be greater than 0"
            ));
        }

        let used = used_by_scope
            .entry(scope_key.clone())
            .or_insert_with(|| collect_used_scsi_ids(&snapshot, &scope_key));

        let mut candidate = placement.start;
        while used.contains(&candidate) {
            candidate = candidate.saturating_add(placement.step);
        }

        if candidate > 255 {
            return Err(anyhow!(
                "No free scsi_id available for scope '{}' within 0..=255",
                scope_key
            ));
        }

        drive_map.insert(
            Value::String("scsi_id".to_string()),
            Value::Number((candidate as u64).into()),
        );
        used.insert(candidate);
    }

    Ok(())
}

fn apply_network_placement_policies(root: &mut Value, policies: &[NetworkPolicy]) -> Result<()> {
    let Some(networks) = get_nested_sequence_mut(root, &["devices", "networks"])? else {
        return Ok(());
    };

    let snapshot = networks.clone();
    let mut used_by_scope: HashMap<String, BTreeSet<u32>> = HashMap::new();

    for network in networks {
        let Value::Mapping(network_map) = network else {
            continue;
        };

        if network_map.contains_key(Value::String("addr".to_string())) {
            continue;
        }

        let Some(placement) = policies.iter().rev().find_map(|policy| {
            if network_matches_policy(network_map, &policy.selector) {
                policy.placement.addr.as_ref()
            } else {
                None
            }
        }) else {
            continue;
        };

        if placement.step == 0 {
            return Err(anyhow!(
                "policies.networks[].placement.addr.step must be greater than 0"
            ));
        }

        let start = parse_numeric_addr(&placement.start)
            .with_context(|| format!("Invalid policies.networks[].placement.addr.start '{}': expected decimal or 0x-prefixed hex", placement.start))?;

        let scope_key = resolve_addr_scope_key(network_map, placement)?;
        let used = used_by_scope
            .entry(scope_key.clone())
            .or_insert_with(|| collect_used_addrs(&snapshot, &scope_key));

        let mut candidate = start;
        while used.contains(&candidate) {
            candidate = candidate.saturating_add(placement.step);
        }

        network_map.insert(
            Value::String("addr".to_string()),
            Value::String(format!("0x{:x}", candidate)),
        );
        used.insert(candidate);
    }

    Ok(())
}

fn resolve_scsi_scope_key(
    drive_map: &Mapping,
    placement: &ScsiIdPlacementConfig,
) -> Result<String> {
    match placement.scope.as_str() {
        "global" => Ok("scsi:global".to_string()),
        "controller" => {
            let controller = placement
                .controller
                .clone()
                .or_else(|| get_string_field(drive_map, "controller"))
                .ok_or_else(|| anyhow!("scsi_id placement scope 'controller' requires drive.controller or placement.controller"))?;
            Ok(format!("scsi:controller:{}", controller))
        }
        other => Err(anyhow!(
            "Unsupported scsi_id placement scope '{}': supported scopes are 'controller' and 'global'",
            other
        )),
    }
}

fn resolve_addr_scope_key(
    network_map: &Mapping,
    placement: &AddrPlacementConfig,
) -> Result<String> {
    match placement.scope.as_str() {
        "global" => Ok("addr:global".to_string()),
        "bus" => {
            let bus = placement
                .bus
                .clone()
                .or_else(|| get_string_field(network_map, "bus"))
                .ok_or_else(|| {
                    anyhow!("addr placement scope 'bus' requires network.bus or placement.bus")
                })?;
            Ok(format!("addr:bus:{}", bus))
        }
        other => Err(anyhow!(
            "Unsupported addr placement scope '{}': supported scopes are 'bus' and 'global'",
            other
        )),
    }
}

fn collect_used_scsi_ids(snapshot: &[Value], scope_key: &str) -> BTreeSet<u32> {
    let mut used = BTreeSet::new();

    for drive in snapshot {
        let Value::Mapping(drive_map) = drive else {
            continue;
        };

        let Some(scsi_id) = get_u32_field(drive_map, "scsi_id") else {
            continue;
        };

        let current_scope = if let Some(controller) = get_string_field(drive_map, "controller") {
            format!("scsi:controller:{}", controller)
        } else {
            "scsi:global".to_string()
        };

        if current_scope == scope_key || scope_key == "scsi:global" {
            used.insert(scsi_id);
        }
    }

    used
}

fn collect_used_addrs(snapshot: &[Value], scope_key: &str) -> BTreeSet<u32> {
    let mut used = BTreeSet::new();

    for network in snapshot {
        let Value::Mapping(network_map) = network else {
            continue;
        };

        let Some(addr) = get_string_field(network_map, "addr") else {
            continue;
        };

        let Ok(addr_num) = parse_numeric_addr(&addr) else {
            continue;
        };

        let current_scope = if let Some(bus) = get_string_field(network_map, "bus") {
            format!("addr:bus:{}", bus)
        } else {
            "addr:global".to_string()
        };

        if current_scope == scope_key || scope_key == "addr:global" {
            used.insert(addr_num);
        }
    }

    used
}

fn get_string_field(map: &Mapping, field: &str) -> Option<String> {
    map.get(Value::String(field.to_string()))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn get_u32_field(map: &Mapping, field: &str) -> Option<u32> {
    map.get(Value::String(field.to_string()))
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn parse_numeric_addr(value: &str) -> Result<u32> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u32::from_str_radix(hex, 16)
            .map_err(|_| anyhow!("invalid hexadecimal address '{}'", value));
    }

    trimmed
        .parse::<u32>()
        .map_err(|_| anyhow!("invalid numeric address '{}'", value))
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
