use super::ImportError;

pub fn compact_sequence_mappings(yaml: &str) -> Result<String, ImportError> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|e| {
        ImportError::ParseError(format!(
            "failed to parse generated YAML for compaction: {e}"
        ))
    })?;

    let mut out = String::new();
    render_block(&value, 0, &mut out);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn render_block(value: &serde_yaml::Value, indent: usize, out: &mut String) {
    match value {
        serde_yaml::Value::Mapping(map) => render_mapping_block(map, indent, out),
        serde_yaml::Value::Sequence(seq) => render_sequence_block(seq, indent, out),
        _ => {
            push_indent(out, indent);
            out.push_str(&scalar_inline(value));
            out.push('\n');
        }
    }
}

fn render_mapping_block(map: &serde_yaml::Mapping, indent: usize, out: &mut String) {
    for (key, value) in map {
        push_indent(out, indent);
        out.push_str(&key_inline(key));

        match value {
            serde_yaml::Value::Mapping(child) => {
                if child.is_empty() {
                    out.push_str(": {}\n");
                } else if should_inline_nested_mapping(indent) {
                    out.push_str(": ");
                    out.push_str(&flow_inline(value));
                    out.push('\n');
                } else {
                    out.push_str(":\n");
                    render_mapping_block(child, indent + 2, out);
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                if seq.is_empty() {
                    out.push_str(": []\n");
                } else {
                    out.push_str(":\n");
                    render_sequence_block(seq, indent + 2, out);
                }
            }
            _ => {
                out.push_str(": ");
                out.push_str(&scalar_inline(value));
                out.push('\n');
            }
        }
    }
}

fn should_inline_nested_mapping(indent: usize) -> bool {
    // Inline mapping values for deeper sections such as system.memory.ballooning and ivshmem.
    indent >= 4
}

fn render_sequence_block(seq: &[serde_yaml::Value], indent: usize, out: &mut String) {
    for item in seq {
        push_indent(out, indent);
        out.push_str("- ");

        match item {
            serde_yaml::Value::Mapping(_) => {
                out.push_str(&flow_inline(item));
                out.push('\n');
            }
            serde_yaml::Value::Sequence(_) => {
                out.push_str(&flow_inline(item));
                out.push('\n');
            }
            _ => {
                out.push_str(&scalar_inline(item));
                out.push('\n');
            }
        }
    }
}

fn flow_inline(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Mapping(map) => {
            let mut parts = Vec::with_capacity(map.len());
            for (key, val) in map {
                parts.push(format!("{}: {}", key_inline(key), flow_inline(val)));
            }
            format!("{{{}}}", parts.join(", "))
        }
        serde_yaml::Value::Sequence(seq) => {
            let parts = seq.iter().map(flow_inline).collect::<Vec<_>>();
            format!("[{}]", parts.join(", "))
        }
        _ => scalar_inline(value),
    }
}

fn scalar_inline(value: &serde_yaml::Value) -> String {
    let mut text = serde_yaml::to_string(value).unwrap_or_else(|_| "null\n".to_string());
    if let Some(stripped) = text.strip_prefix("---\n") {
        text = stripped.to_string();
    }
    text.trim_end().to_string()
}

fn key_inline(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => s.clone(),
        _ => scalar_inline(value),
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.push_str(&" ".repeat(indent));
}

#[cfg(test)]
mod tests {
    use super::compact_sequence_mappings;

    #[test]
    fn compacts_sequence_mapping_items_to_flow_style() {
        let input = "name: vm\ndevices:\n  drives:\n    - id: scsi0\n      path: /dev/disk0\n      interface: scsi\n";

        let compacted = compact_sequence_mappings(input).expect("compaction should succeed");

        assert!(compacted.contains("- {id: scsi0, path: /dev/disk0, interface: scsi}"));
    }

    #[test]
    fn compacted_yaml_roundtrips_to_same_value() {
        let input = "system:\n  architecture: x86_64\n  machine: q35\n  memory:\n    size: 2048\ndevices:\n  networks:\n    - id: net0\n      model: virtio-net\n      backend:\n        type: tap\n        bridge: vmbr0\n      boot_index: 100\n";

        let compacted = compact_sequence_mappings(input).expect("compaction should succeed");
        let original_value: serde_yaml::Value = serde_yaml::from_str(input).expect("parse input");
        let compacted_value: serde_yaml::Value =
            serde_yaml::from_str(&compacted).expect("parse compacted");

        assert_eq!(compacted_value, original_value);
    }

    #[test]
    fn compacts_deep_nested_mapping_values_to_flow_style() {
        let input = "system:\n  memory:\n    size: 2048\n    ballooning:\n      model: virtio-balloon-pci\n    ivshmem:\n      enabled: true\n      size: 128\n      id: ivshmem0\n";

        let compacted = compact_sequence_mappings(input).expect("compaction should succeed");

        assert!(compacted.contains("ballooning: {model: virtio-balloon-pci}"));
        assert!(compacted.contains("ivshmem: {enabled: true, size: 128, id: ivshmem0}"));
    }
}
