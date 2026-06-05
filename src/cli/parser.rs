//! CLI argument parsing utilities for turning raw flags into `CliCommand`.
//!
//! Related documentation:
//! - src/README.md
//! - src/cli/README.md

use serde_json::{Map, Value};

use super::CliCommand;

/// Parses the provided raw arguments into a structured CLI command.
///
/// This keeps dotted flag handling and command decoding in one place so the
/// rest of the application receives a typed request.
///
/// # Arguments
///
/// * `args` - Raw CLI arguments after the executable name.
///
/// # Returns
///
/// The decoded command, or an error message if parsing fails.
pub fn parse_cli_options(args: &[String]) -> Result<CliCommand, String> {
    let structured_json = args_to_structured_json(args)?;
    let yaml_payload = serde_json::to_string(&structured_json)
        .map_err(|error| format!("failed to encode CLI payload: {error}"))?;

    serde_yaml::from_str(&yaml_payload)
        .map_err(|error| format!("failed to parse CLI payload: {error}"))
}

/// Converts raw CLI arguments into a JSON object shaped like the command model.
///
/// # Arguments
///
/// * `args` - Raw CLI arguments after the executable name.
///
/// # Returns
///
/// A structured JSON value containing the command name and option map.
fn args_to_structured_json(args: &[String]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("missing subcommand".to_string());
    }

    let command = args[0].clone();
    let options = parse_options_as_nested_json(&args[1..])?;

    Ok(serde_json::json!({
        "command": command,
        "options": options,
    }))
}

/// Parses CLI flags into a nested JSON object keyed by dotted option paths.
///
/// # Arguments
///
/// * `args` - Raw flag and value tokens.
///
/// # Returns
///
/// The structured option map, or an error if the flags are malformed.
fn parse_options_as_nested_json(args: &[String]) -> Result<Map<String, Value>, String> {
    let mut options = Map::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with("--") {
            return Err(format!("unexpected positional argument '{}'", arg));
        }

        let key = arg.trim_start_matches("--");
        if key.is_empty() {
            return Err("empty flag name is not allowed".to_string());
        }

        if index + 1 < args.len() && !args[index + 1].starts_with("--") {
            insert_dotted_key(&mut options, key, Value::String(args[index + 1].clone()))?;
            index += 2;
            continue;
        }

        insert_dotted_key(&mut options, key, Value::Bool(true))?;
        index += 1;
    }

    Ok(options)
}

/// Inserts a value under a dotted option path.
///
/// # Arguments
///
/// * `map` - The object to update.
/// * `key` - The dotted option path.
/// * `value` - The value to insert at that path.
///
/// # Returns
///
/// `Ok(())` when the value has been inserted successfully.
fn insert_dotted_key(map: &mut Map<String, Value>, key: &str, value: Value) -> Result<(), String> {
    let segments: Vec<&str> = key.split('.').collect();
    insert_segments(map, &segments, value)
}

/// Inserts a value into a nested JSON object using path segments.
///
/// # Arguments
///
/// * `current` - The current object being updated.
/// * `segments` - The remaining path segments to traverse.
/// * `value` - The value to insert at the leaf node.
///
/// # Returns
///
/// `Ok(())` when insertion succeeds, or an error when the path is invalid.
fn insert_segments(
    current: &mut Map<String, Value>,
    segments: &[&str],
    value: Value,
) -> Result<(), String> {
    if segments.is_empty() {
        return Err("empty flag path is not allowed".to_string());
    }

    if segments.len() == 1 {
        current.insert(segments[0].to_string(), value);
        return Ok(());
    }

    let head = segments[0].to_string();
    let entry = current
        .entry(head)
        .or_insert_with(|| Value::Object(Map::new()));

    let Some(child) = entry.as_object_mut() else {
        return Err(format!(
            "cannot nest property under non-object flag '--{}'",
            segments[0]
        ));
    };

    insert_segments(child, &segments[1..], value)
}
