//! YAML parsing and document assembly for canonical VM specification.
//!
//! This module owns the extraction of raw YAML structure into the canonical model,
//! prior to semantic validation.

use serde_yaml::{Mapping, Value};
use thiserror::Error;

use super::model::{
    CanonicalDocument, Cpu, Machine, Memory, Metadata, NetworkEntry, ResourceRef, StorageEntry,
    System, VirtualMachine,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub reason: String,
}

impl ValidationIssue {
    pub fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("validation failed with {0} issue(s)")]
    Validation(usize, Vec<ValidationIssue>),
}

pub fn parse_canonical_document_from_yaml(
    yaml: &str,
) -> Result<CanonicalDocument, ParseError> {
    let root: Value = serde_yaml::from_str(yaml)?;
    parse_canonical_document(&root)
}

pub fn parse_canonical_document(root: &Value) -> Result<CanonicalDocument, ParseError> {
    let mut issues = Vec::new();
    let Some(root_map) = expect_mapping(root, "$", &mut issues) else {
        return Err(ParseError::Validation(issues.len(), issues));
    };

    let metadata_map = required_mapping(root_map, "metadata", "metadata", &mut issues);
    let vm_map = required_mapping(root_map, "virtual_machine", "virtual_machine", &mut issues);

    let schema_version = metadata_map
        .and_then(|m| required_string(m, "schema_version", "metadata.schema_version", &mut issues));
    let vm_name =
        metadata_map.and_then(|m| required_string(m, "vm_name", "metadata.vm_name", &mut issues));

    let system_map =
        vm_map.and_then(|m| required_mapping(m, "system", "virtual_machine.system", &mut issues));
    let machine_map = system_map.and_then(|m| {
        required_mapping(m, "machine", "virtual_machine.system.machine", &mut issues)
    });
    let cpu_map = system_map
        .and_then(|m| required_mapping(m, "cpu", "virtual_machine.system.cpu", &mut issues));
    let memory_map = system_map
        .and_then(|m| required_mapping(m, "memory", "virtual_machine.system.memory", &mut issues));

    let family = machine_map.and_then(|m| {
        required_string(
            m,
            "family",
            "virtual_machine.system.machine.family",
            &mut issues,
        )
    });
    let chipset = machine_map.and_then(|m| {
        required_string(
            m,
            "chipset",
            "virtual_machine.system.machine.chipset",
            &mut issues,
        )
    });
    let cpu_model = cpu_map
        .and_then(|m| required_string(m, "model", "virtual_machine.system.cpu.model", &mut issues));
    let memory_min = memory_map
        .and_then(|m| required_i64(m, "min", "virtual_machine.system.memory.min", &mut issues));

    let storage = vm_map
        .map(|m| {
            parse_id_scope::<StorageEntry>(m, "storage", "virtual_machine.storage", &mut issues)
        })
        .unwrap_or_default();
    let network = vm_map
        .map(|m| {
            parse_id_scope::<NetworkEntry>(m, "network", "virtual_machine.network", &mut issues)
        })
        .unwrap_or_default();
    let resources = vm_map
        .map(|m| {
            parse_id_scope::<ResourceRef>(m, "resources", "virtual_machine.resources", &mut issues)
        })
        .unwrap_or_default();

    if !issues.is_empty() {
        return Err(ParseError::Validation(issues.len(), issues));
    }

    let Some(schema_version) = schema_version else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(vm_name) = vm_name else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(family) = family else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(chipset) = chipset else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(cpu_model) = cpu_model else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(memory_min) = memory_min else {
        return Err(ParseError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };

    Ok(CanonicalDocument {
        metadata: Metadata {
            schema_version,
            vm_name,
        },
        virtual_machine: VirtualMachine {
            system: System {
                machine: Machine { family, chipset },
                cpu: Cpu { model: cpu_model },
                memory: Memory { min: memory_min },
            },
            storage,
            network,
            resources,
        },
    })
}

fn expect_mapping<'a>(
    value: &'a Value,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a Mapping> {
    match value {
        Value::Mapping(map) => Some(map),
        _ => {
            issues.push(ValidationIssue::new(path, "must be a mapping"));
            None
        }
    }
}

fn required_mapping<'a>(
    map: &'a Mapping,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a Mapping> {
    let value = map.get(Value::String(key.to_owned()));
    match value {
        Some(Value::Mapping(inner)) => Some(inner),
        Some(_) => {
            issues.push(ValidationIssue::new(path, "must be a mapping"));
            None
        }
        None => {
            issues.push(ValidationIssue::new(path, "is required"));
            None
        }
    }
}

fn required_string(
    map: &Mapping,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<String> {
    let value = map.get(Value::String(key.to_owned()));
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            issues.push(ValidationIssue::new(path, "must be a string"));
            None
        }
        None => {
            issues.push(ValidationIssue::new(path, "is required"));
            None
        }
    }
}

fn required_i64(
    map: &Mapping,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<i64> {
    let value = map.get(Value::String(key.to_owned()));
    match value {
        Some(Value::Number(number)) => {
            if let Some(int_val) = number.as_i64() {
                Some(int_val)
            } else {
                issues.push(ValidationIssue::new(path, "must be an integer"));
                None
            }
        }
        Some(_) => {
            issues.push(ValidationIssue::new(path, "must be an integer"));
            None
        }
        None => {
            issues.push(ValidationIssue::new(path, "is required"));
            None
        }
    }
}

fn parse_id_scope<T>(
    vm_map: &Mapping,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<T>
where
    T: From<String>,
{
    let Some(value) = vm_map.get(Value::String(key.to_owned())) else {
        return Vec::new();
    };

    let Value::Sequence(items) = value else {
        issues.push(ValidationIssue::new(path, "must be a list"));
        return Vec::new();
    };

    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{idx}]");
        let Value::Mapping(entry_map) = item else {
            issues.push(ValidationIssue::new(item_path, "must be a mapping"));
            continue;
        };

        if let Some(id) = required_string(entry_map, "id", &format!("{path}[{idx}].id"), issues) {
            out.push(T::from(id));
        }
    }

    out
}
