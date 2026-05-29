use std::collections::HashSet;
use std::path::Path;

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
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConformanceError {
    #[error("invalid canonical yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("validation failed with {0} issue(s)")]
    Validation(usize, Vec<ValidationIssue>),
}

impl ConformanceError {
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            Self::Validation(_, issues) => issues,
            Self::Yaml(_) => &[],
        }
    }
}

pub fn validate_canonical_yaml(
    yaml: &str,
    filename: &Path,
) -> Result<CanonicalDocument, ConformanceError> {
    let root: Value = serde_yaml::from_str(yaml)?;
    let doc = parse_canonical_document(&root)?;
    validate_canonical_document(&doc, filename)?;
    Ok(doc)
}

fn parse_canonical_document(root: &Value) -> Result<CanonicalDocument, ConformanceError> {
    let mut issues = Vec::new();
    let Some(root_map) = expect_mapping(root, "$", &mut issues) else {
        return Err(ConformanceError::Validation(issues.len(), issues));
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
        return Err(ConformanceError::Validation(issues.len(), issues));
    }

    let Some(schema_version) = schema_version else {
        return Err(ConformanceError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(vm_name) = vm_name else {
        return Err(ConformanceError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(family) = family else {
        return Err(ConformanceError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(chipset) = chipset else {
        return Err(ConformanceError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(cpu_model) = cpu_model else {
        return Err(ConformanceError::Validation(
            1,
            vec![ValidationIssue::new(
                "$",
                "internal assembly failure after schema validation",
            )],
        ));
    };
    let Some(memory_min) = memory_min else {
        return Err(ConformanceError::Validation(
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

impl From<String> for StorageEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for NetworkEntry {
    fn from(id: String) -> Self {
        Self { id }
    }
}

impl From<String> for ResourceRef {
    fn from(id: String) -> Self {
        Self { id }
    }
}

pub fn validate_canonical_document(
    doc: &CanonicalDocument,
    filename: &Path,
) -> Result<(), ConformanceError> {
    let mut issues = Vec::new();

    check_required_strings(
        &mut issues,
        "metadata.schema_version",
        &doc.metadata.schema_version,
    );
    check_required_strings(&mut issues, "metadata.vm_name", &doc.metadata.vm_name);
    check_required_strings(
        &mut issues,
        "virtual_machine.system.machine.family",
        &doc.virtual_machine.system.machine.family,
    );
    check_required_strings(
        &mut issues,
        "virtual_machine.system.machine.chipset",
        &doc.virtual_machine.system.machine.chipset,
    );
    check_required_strings(
        &mut issues,
        "virtual_machine.system.cpu.model",
        &doc.virtual_machine.system.cpu.model,
    );

    if doc.virtual_machine.system.memory.min < 0 {
        issues.push(ValidationIssue::new(
            "virtual_machine.system.memory.min",
            "must be an integer >= 0",
        ));
    }

    validate_vm_name_filename_match(&mut issues, &doc.metadata.vm_name, filename);
    validate_machine_consistency(
        &mut issues,
        &doc.virtual_machine.system.machine.family,
        &doc.virtual_machine.system.machine.chipset,
    );
    validate_unique_ids_storage(&mut issues, &doc.virtual_machine.storage);
    validate_unique_ids_network(&mut issues, &doc.virtual_machine.network);
    validate_unique_ids_resources(&mut issues, &doc.virtual_machine.resources);

    if issues.is_empty() {
        Ok(())
    } else {
        Err(ConformanceError::Validation(issues.len(), issues))
    }
}

fn check_required_strings(issues: &mut Vec<ValidationIssue>, path: &str, value: &str) {
    if value.trim().is_empty() {
        issues.push(ValidationIssue::new(
            path,
            "is required and must be a non-empty string",
        ));
    }
}

fn validate_vm_name_filename_match(
    issues: &mut Vec<ValidationIssue>,
    vm_name: &str,
    filename: &Path,
) {
    let expected = filename.file_stem().and_then(|stem| stem.to_str());
    match expected {
        Some(stem) if !stem.is_empty() => {
            if vm_name != stem {
                issues.push(ValidationIssue::new(
                    "metadata.vm_name",
                    format!("must match filename stem '{stem}'"),
                ));
            }
        }
        _ => {
            issues.push(ValidationIssue::new(
                "metadata.vm_name",
                "cannot validate vm_name because filename stem is unavailable",
            ));
        }
    }
}

fn validate_machine_consistency(issues: &mut Vec<ValidationIssue>, family: &str, chipset: &str) {
    if family == "pc" {
        let allowed = ["q35", "i440fx"];
        if !allowed.contains(&chipset) {
            issues.push(ValidationIssue::new(
                "virtual_machine.system.machine.chipset",
                "must be one of [q35, i440fx] when machine family is 'pc'",
            ));
        }
    }
}

fn validate_unique_ids_storage(issues: &mut Vec<ValidationIssue>, entries: &[StorageEntry]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.storage[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(ValidationIssue::new(
                path,
                "is required and must be a non-empty string",
            ));
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(ValidationIssue::new(
                path,
                format!("duplicate id '{}'", entry.id),
            ));
        }
    }
}

fn validate_unique_ids_network(issues: &mut Vec<ValidationIssue>, entries: &[NetworkEntry]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.network[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(ValidationIssue::new(
                path,
                "is required and must be a non-empty string",
            ));
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(ValidationIssue::new(
                path,
                format!("duplicate id '{}'", entry.id),
            ));
        }
    }
}

fn validate_unique_ids_resources(issues: &mut Vec<ValidationIssue>, entries: &[ResourceRef]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.resources[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(ValidationIssue::new(
                path,
                "is required and must be a non-empty string",
            ));
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(ValidationIssue::new(
                path,
                format!("duplicate id '{}'", entry.id),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ConformanceError, validate_canonical_yaml};

    fn valid_yaml() -> &'static str {
        r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
  storage:
    - id: "disk0"
  network:
    - id: "net0"
  resources:
    - id: "gpu0"
"#
    }

    fn expect_validation_issues(err: ConformanceError) -> Vec<(String, String)> {
        match err {
            ConformanceError::Validation(_, issues) => {
                issues.into_iter().map(|i| (i.path, i.reason)).collect()
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn passing_canonical_example() {
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(valid_yaml(), filename);
        assert!(result.is_ok());
    }

    #[test]
    fn missing_required_field() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu: {}
    memory:
      min: 8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(
            issues
                .iter()
                .any(|(path, reason)| path == "virtual_machine.system.cpu.model"
                    && reason == "is required")
        );
    }

    #[test]
    fn vm_name_mismatch() {
        let filename = Path::new("/tmp/another-name.yaml");
        let err =
            validate_canonical_yaml(valid_yaml(), filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.vm_name" && reason.contains("filename stem 'another-name'")
        }));
    }

    #[test]
    fn duplicate_ids() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
  storage:
    - id: "disk0"
    - id: "disk0"
  network:
    - id: "net0"
    - id: "net0"
  resources:
    - id: "gpu0"
    - id: "gpu0"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(
            issues
                .iter()
                .any(|(path, reason)| path == "virtual_machine.storage[1].id"
                    && reason.contains("duplicate id"))
        );
        assert!(
            issues
                .iter()
                .any(|(path, reason)| path == "virtual_machine.network[1].id"
                    && reason.contains("duplicate id"))
        );
        assert!(
            issues
                .iter()
                .any(|(path, reason)| path == "virtual_machine.resources[1].id"
                    && reason.contains("duplicate id"))
        );
    }

    #[test]
    fn invalid_chipset_family_combination() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "arm-virt"
    cpu:
      model: "host"
    memory:
      min: 8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.system.machine.chipset" && reason.contains("[q35, i440fx]")
        }));
    }

    #[test]
    fn invalid_type_reports_field_path() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "pc"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: "8192"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(
            issues
                .iter()
                .any(|(path, reason)| path == "virtual_machine.system.memory.min"
                    && reason == "must be an integer")
        );
    }

    #[test]
    fn empty_ids_report_precise_field_paths() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "pc"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: 8192
    storage:
        - id: ""
    network:
        - id: ""
    resources:
        - id: ""
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);

        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.storage[0].id"
                && reason == "is required and must be a non-empty string"
        }));
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.network[0].id"
                && reason == "is required and must be a non-empty string"
        }));
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.resources[0].id"
                && reason == "is required and must be a non-empty string"
        }));
    }

    #[test]
    fn duplicate_storage_id_reports_offending_entry_index() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "pc"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: 8192
    storage:
        - id: "disk0"
        - id: "disk1"
        - id: "disk0"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);

        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.storage[2].id" && reason.contains("duplicate id 'disk0'")
        }));
        assert!(
            !issues
                .iter()
                .any(|(path, _)| path == "virtual_machine.storage[1].id")
        );
    }

    #[test]
    fn same_id_across_scopes_is_allowed() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "pc"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: 8192
    storage:
        - id: "shared0"
    network:
        - id: "shared0"
    resources:
        - id: "shared0"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(yaml, filename);
        assert!(result.is_ok());
    }
}
