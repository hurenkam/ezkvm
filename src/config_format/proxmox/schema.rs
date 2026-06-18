/// Define the Proxmox configuration schema.
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Parsed representation of a Proxmox VM configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxmoxConfigSchema {
    pub global: ProxmoxSection,
    pub snapshots: BTreeMap<String, ProxmoxSection>,
}

impl ProxmoxConfigSchema {
    /// Parses Proxmox `.conf` text into a typed schema.
    pub fn parse(source: &str) -> Result<Self, ProxmoxParseError> {
        let mut global = ProxmoxSection::default();
        let mut snapshots: BTreeMap<String, ProxmoxSection> = BTreeMap::new();
        let mut active_snapshot: Option<String> = None;

        for (index, raw_line) in source.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') {
                let name = parse_snapshot_header(line, line_number)?;
                if snapshots.contains_key(&name) {
                    return Err(ProxmoxParseError::DuplicateSnapshot {
                        line: line_number,
                        name,
                    });
                }
                snapshots.insert(name.clone(), ProxmoxSection::default());
                active_snapshot = Some(name);
                continue;
            }

            let entry = parse_entry(line, line_number)?;
            let section =
                match active_snapshot.as_ref() {
                    Some(snapshot_name) => snapshots.get_mut(snapshot_name).ok_or_else(|| {
                        ProxmoxParseError::Internal {
                            line: line_number,
                            reason: "active snapshot not found while parsing".to_string(),
                        }
                    })?,
                    None => &mut global,
                };

            section.insert_entry(entry, line_number)?;
        }

        Ok(Self { global, snapshots })
    }

    /// Renders the schema into a deterministic Proxmox `.conf` text representation.
    pub fn render(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.extend(self.global.render_lines());

        for (snapshot, section) in &self.snapshots {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push(format!("[{snapshot}]"));
            lines.extend(section.render_lines());
        }

        lines.join("\n")
    }
}

impl FromStr for ProxmoxConfigSchema {
    type Err = ProxmoxParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Display for ProxmoxConfigSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

/// One section in a Proxmox config file (global section or snapshot section).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxmoxSection {
    pub entries: BTreeMap<String, ProxmoxValue>,
}

impl ProxmoxSection {
    fn insert_entry(
        &mut self,
        entry: ProxmoxEntry,
        line_number: usize,
    ) -> Result<(), ProxmoxParseError> {
        let key = entry.key;
        if self.entries.insert(key.clone(), entry.value).is_some() {
            return Err(ProxmoxParseError::DuplicateKey {
                line: line_number,
                key,
            });
        }
        Ok(())
    }

    fn render_lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProxmoxEntry {
    key: String,
    value: ProxmoxValue,
}

/// Parsed right-hand-side Proxmox field value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProxmoxValue {
    Scalar { value: String },
    Compound(ProxmoxCompoundValue),
}

impl Display for ProxmoxValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar { value } => f.write_str(value),
            Self::Compound(compound) => Display::fmt(compound, f),
        }
    }
}

/// A comma-separated value with a head token and optional options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxmoxCompoundValue {
    pub head: String,
    pub options: Vec<ProxmoxOption>,
}

impl Display for ProxmoxCompoundValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.head)?;
        for option in &self.options {
            f.write_str(",")?;
            Display::fmt(option, f)?;
        }
        Ok(())
    }
}

/// Option entry inside a compound Proxmox value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProxmoxOption {
    Flag { value: String },
    KeyValue { key: String, value: String },
}

impl Display for ProxmoxOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flag { value } => f.write_str(value),
            Self::KeyValue { key, value } => write!(f, "{key}={value}"),
        }
    }
}

/// Parse failures for Proxmox schema text input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ProxmoxParseError {
    #[error("line {line}: malformed snapshot header '{header}'")]
    MalformedSnapshotHeader { line: usize, header: String },
    #[error("line {line}: duplicate snapshot section '{name}'")]
    DuplicateSnapshot { line: usize, name: String },
    #[error("line {line}: malformed key/value line '{line_text}'")]
    MalformedEntry { line: usize, line_text: String },
    #[error("line {line}: empty key in line '{line_text}'")]
    EmptyKey { line: usize, line_text: String },
    #[error("line {line}: empty value for key '{key}'")]
    EmptyValue { line: usize, key: String },
    #[error("line {line}: duplicate key '{key}' in the same section")]
    DuplicateKey { line: usize, key: String },
    #[error("line {line}: internal parse state error: {reason}")]
    Internal { line: usize, reason: String },
}

fn parse_snapshot_header(line: &str, line_number: usize) -> Result<String, ProxmoxParseError> {
    if !line.ends_with(']') {
        return Err(ProxmoxParseError::MalformedSnapshotHeader {
            line: line_number,
            header: line.to_string(),
        });
    }

    let Some(inner) = line
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return Err(ProxmoxParseError::MalformedSnapshotHeader {
            line: line_number,
            header: line.to_string(),
        });
    };

    let name = inner.trim();
    if name.is_empty() {
        return Err(ProxmoxParseError::MalformedSnapshotHeader {
            line: line_number,
            header: line.to_string(),
        });
    }

    Ok(name.to_string())
}

fn parse_entry(line: &str, line_number: usize) -> Result<ProxmoxEntry, ProxmoxParseError> {
    let Some((raw_key, raw_value)) = line.split_once(':') else {
        return Err(ProxmoxParseError::MalformedEntry {
            line: line_number,
            line_text: line.to_string(),
        });
    };

    let key = raw_key.trim();
    if key.is_empty() {
        return Err(ProxmoxParseError::EmptyKey {
            line: line_number,
            line_text: line.to_string(),
        });
    }

    let value = raw_value.trim();
    if value.is_empty() {
        return Err(ProxmoxParseError::EmptyValue {
            line: line_number,
            key: key.to_string(),
        });
    }

    Ok(ProxmoxEntry {
        key: key.to_string(),
        value: parse_value(value),
    })
}

fn parse_value(value: &str) -> ProxmoxValue {
    if !value.contains(',') {
        return ProxmoxValue::Scalar {
            value: value.to_string(),
        };
    }

    let mut parts = value.split(',').map(str::trim);
    let head = parts.next().unwrap_or_default().to_string();
    let options = parts
        .filter(|token| !token.is_empty())
        .map(parse_option)
        .collect::<Vec<_>>();

    ProxmoxValue::Compound(ProxmoxCompoundValue { head, options })
}

fn parse_option(token: &str) -> ProxmoxOption {
    let Some((key, value)) = token.split_once('=') else {
        return ProxmoxOption::Flag {
            value: token.to_string(),
        };
    };

    ProxmoxOption::KeyValue {
        key: key.trim().to_string(),
        value: value.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ProxmoxConfigSchema, ProxmoxParseError};

    #[test]
    fn parses_global_and_snapshot_sections() {
        let source = r#"
name: endor
machine: q35
memory: 16384
scsi0: vm1-pool:vm-101-disk-1,cache=writeback,size=64G

[initial_snapshot]
name: endor
machine: q35
memory: 8192
"#;

        let parsed = ProxmoxConfigSchema::parse(source).expect("source should parse");
        assert_eq!(parsed.global.entries.len(), 4);
        assert_eq!(parsed.snapshots.len(), 1);
        assert!(parsed.snapshots.contains_key("initial_snapshot"));
    }

    #[test]
    fn parse_render_parse_roundtrip_is_stable() {
        let source = r#"
agent: 1
boot: order=scsi0;ide2;net0
machine: q35
memory: 65536
name: gyndine
net0: virtio=DE:AD:BE:EF:00:42,bridge=vmbr0
scsi0: vm1:vm-3100-disk-1,cache=writeback,discard=on,iothread=1,size=32G,ssd=1

[clean_state]
memory: 32768
name: gyndine
"#;

        let first = ProxmoxConfigSchema::parse(source).expect("source should parse");
        let rendered = first.render();
        let second = ProxmoxConfigSchema::parse(&rendered).expect("rendered source should parse");

        assert_eq!(first, second);
    }

    #[test]
    fn rejects_malformed_lines() {
        let source = "name endor";
        let error = ProxmoxConfigSchema::parse(source).expect_err("source must fail");

        assert!(matches!(
            error,
            ProxmoxParseError::MalformedEntry { line: 1, .. }
        ));
    }

    #[test]
    fn rejects_duplicate_keys_in_same_section() {
        let source = r#"
name: endor
name: duplicate
"#;

        let error = ProxmoxConfigSchema::parse(source).expect_err("source must fail");
        assert_eq!(
            error,
            ProxmoxParseError::DuplicateKey {
                line: 3,
                key: "name".to_string(),
            }
        );
    }
}
