//! Marshaler stage: `EzkvmConfigSchema` → YAML text.

use serde_yaml::{Mapping, Value};

use crate::config_format::{ezkvm::EzkvmConfigSchema, stages::Marshaler};

/// Marshals an `EzkvmConfigSchema` into ezkvm YAML text.
pub struct EzkvmMarshaler;

impl Marshaler for EzkvmMarshaler {
    type Schema = EzkvmConfigSchema;
    type Error = String;

    fn marshal(&self, schema: &EzkvmConfigSchema) -> Result<String, String> {
        let value = serde_yaml::to_value(schema)
            .map_err(|e| format!("Failed to convert schema to YAML value: {e}"))?;
        render_compact_yaml(&value)
    }
}

fn render_compact_yaml(value: &Value) -> Result<String, String> {
    let rendered = match value {
        Value::Mapping(map) => render_mapping_block(map, 0, 0, None)?,
        _ => render_block(value, 0, 0)?,
    };

    let mut out = rendered.trim_end().to_string();
    out.push('\n');
    Ok(out)
}

fn render_block(value: &Value, indent: usize, depth: usize) -> Result<String, String> {
    match value {
        Value::Mapping(map) => render_mapping_block(map, indent, depth, None),
        Value::Sequence(seq) => render_sequence_block(seq, indent, depth, None),
        _ => render_scalar(value),
    }
}

fn render_mapping_block(
    map: &Mapping,
    indent: usize,
    depth: usize,
    parent_key: Option<&str>,
) -> Result<String, String> {
    if map.is_empty() {
        return Ok("{}".to_string());
    }

    let mut out = String::new();
    let in_top_level = depth == 0 && indent == 0;
    let mut emitted = 0usize;

    for (key, value) in map {
        let key_text = render_key(key)?;
        let force_block_container = key_text == "resources" || key_text == "devices";
        let force_block_top_level_section = depth == 0 && key_text != "metadata";

        if in_top_level && emitted > 0 {
            out.push('\n');
        }

        out.push_str(&" ".repeat(indent));
        out.push_str(&key_text);
        out.push(':');

        if !force_block_container
            && !force_block_top_level_section
            && (should_inline_field(parent_key, &key_text, value, depth)
                || is_inline_candidate(value, depth + 1))
        {
            out.push(' ');
            out.push_str(&render_inline(value)?);
            out.push('\n');
            emitted += 1;
            continue;
        }

        match value {
            Value::Mapping(_) | Value::Sequence(_) => {
                out.push('\n');
                match value {
                    Value::Mapping(child) => {
                        out.push_str(&render_mapping_block(
                            child,
                            indent + 2,
                            depth + 1,
                            Some(&key_text),
                        )?);
                    }
                    Value::Sequence(child) => {
                        out.push_str(&render_sequence_block(
                            child,
                            indent + 2,
                            depth + 1,
                            Some(&key_text),
                        )?);
                    }
                    _ => {}
                }
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {
                out.push(' ');
                out.push_str(&render_scalar(value)?);
                out.push('\n');
            }
        }

        emitted += 1;
    }

    Ok(out)
}

fn render_sequence_block(
    seq: &[Value],
    indent: usize,
    depth: usize,
    parent_key: Option<&str>,
) -> Result<String, String> {
    if seq.is_empty() {
        return Ok("[]".to_string());
    }

    let mut out = String::new();
    for item in seq {
        out.push_str(&" ".repeat(indent));
        out.push_str("- ");

        if should_inline_sequence_item(parent_key, item, depth)
            || is_inline_candidate(item, depth + 1)
        {
            out.push_str(&render_inline(item)?);
            out.push('\n');
            continue;
        }

        match item {
            Value::Mapping(_) | Value::Sequence(_) => {
                out.push('\n');
                match item {
                    Value::Mapping(child) => {
                        out.push_str(&render_mapping_block(
                            child,
                            indent + 2,
                            depth + 1,
                            parent_key,
                        )?);
                    }
                    Value::Sequence(child) => {
                        out.push_str(&render_sequence_block(
                            child,
                            indent + 2,
                            depth + 1,
                            parent_key,
                        )?);
                    }
                    _ => {}
                }
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {
                out.push_str(&render_scalar(item)?);
                out.push('\n');
            }
        }
    }

    Ok(out)
}

fn should_inline_field(
    parent_key: Option<&str>,
    key_text: &str,
    value: &Value,
    depth: usize,
) -> bool {
    if depth == 0 && key_text == "metadata" {
        return matches!(value, Value::Mapping(_));
    }

    matches!(parent_key, Some("resources") | Some("devices"))
}

fn should_inline_sequence_item(parent_key: Option<&str>, item: &Value, _depth: usize) -> bool {
    if matches!(parent_key, Some("resources") | Some("devices")) {
        return matches!(item, Value::Mapping(_));
    }
    false
}

fn render_inline(value: &Value) -> Result<String, String> {
    match value {
        Value::Mapping(map) => {
            let mut parts = Vec::with_capacity(map.len());
            for (key, value) in map {
                let key_text = render_key(key)?;
                let value_text = render_inline(value)?;
                parts.push(format!("{key_text}: {value_text}"));
            }
            Ok(format!("{{ {} }}", parts.join(", ")))
        }
        Value::Sequence(seq) => {
            let mut parts = Vec::with_capacity(seq.len());
            for item in seq {
                parts.push(render_inline(item)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        _ => render_scalar(value),
    }
}

fn render_key(key: &Value) -> Result<String, String> {
    match key {
        Value::String(s) if is_plain_key(s) => Ok(s.clone()),
        _ => render_scalar(key),
    }
}

fn render_scalar(value: &Value) -> Result<String, String> {
    let mut text = serde_yaml::to_string(value)
        .map_err(|e| format!("Failed to render scalar YAML value: {e}"))?;

    if let Some(stripped) = text.strip_prefix("---\n") {
        text = stripped.to_string();
    }

    Ok(text.trim().to_string())
}

fn is_inline_candidate(value: &Value, depth: usize) -> bool {
    if depth > 3 {
        return false;
    }

    match value {
        Value::Mapping(map) => {
            if map.is_empty() || map.len() > 6 {
                return false;
            }

            map.values().all(|v| match v {
                Value::Mapping(inner) => inner.len() <= 4 && is_inline_candidate(v, depth + 1),
                Value::Sequence(inner) => inner.len() <= 4 && is_inline_candidate(v, depth + 1),
                _ => true,
            })
        }
        Value::Sequence(seq) => {
            if seq.is_empty() || seq.len() > 4 {
                return false;
            }

            seq.iter().all(|v| match v {
                Value::Mapping(inner) => inner.len() <= 4 && is_inline_candidate(v, depth + 1),
                Value::Sequence(inner) => inner.len() <= 4 && is_inline_candidate(v, depth + 1),
                _ => true,
            })
        }
        _ => true,
    }
}

fn is_plain_key(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::render_compact_yaml;
    use serde_yaml::Value;

    fn parse_yaml(input: &str) -> Value {
        serde_yaml::from_str::<Value>(input).expect("test YAML should parse")
    }

    #[test]
    fn compacts_simple_nested_mappings() {
        let input = r#"
metadata:
  schema_version: 1.0.0
  vm_name: bakura
host:
  spice:
    port: 5900
    listen: 0.0.0.0
    disable_ticketing: true
virtual_machine:
    machine:
        family: pc
        chipset: q35
"#;

        let value = parse_yaml(input);
        let rendered = render_compact_yaml(&value).expect("compact render should succeed");

        assert!(rendered.contains("metadata: {"));
        assert!(rendered.contains("schema_version:"));
        assert!(rendered.contains("vm_name: bakura"));
        assert!(
            rendered.contains("spice: { port: 5900, listen: 0.0.0.0, disable_ticketing: true }")
        );
        assert!(rendered.contains("\n\nhost:\n"));
        assert!(rendered.contains("\n\nvirtual_machine:\n"));
    }

    #[test]
    fn keeps_yaml_semantics_equal_after_compaction() {
        let input = r#"
virtual_machine:
  devices:
    - pcie:
        bus: 0
        device: 0
        function: 0
        type: pv_scsi
    - scsi:
        bus: 0
        address:
          target: 0
          lun: 0
        type: hdd
        resource: storage0
"#;

        let original = parse_yaml(input);
        let rendered = render_compact_yaml(&original).expect("compact render should succeed");
        let reparsed = parse_yaml(&rendered);

        assert_eq!(original, reparsed);
    }

    #[test]
    fn keeps_resources_and_devices_items_on_single_line() {
        let input = r#"
metadata:
  schema_version: 1.0.0
  vm_name: bakura
host:
  resources:
  - id: storage0
    storage:
      block_device: /dev/vm0/vm-202-disk-0
virtual_machine:
  machine:
    family: pc
    chipset: q35
  devices:
  - pcie:
      bus: 0
      device: 0
      function: 0
      type: pv_scsi
"#;

        let value = parse_yaml(input);
        let rendered = render_compact_yaml(&value).expect("compact render should succeed");

        assert!(
            rendered
                .contains("- { id: storage0, storage: { block_device: /dev/vm0/vm-202-disk-0 } }")
        );
        assert!(rendered.contains("- { pcie: { bus: 0, device: 0, function: 0, type: pv_scsi } }"));
        assert!(rendered.contains("resources:\n    - {"));
        assert!(rendered.contains("devices:\n    - {"));
    }
}
