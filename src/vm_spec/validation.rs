use std::collections::HashSet;
use std::path::Path;

use serde_json::json;
use thiserror::Error;

use super::model::{CanonicalDocument, NetworkEntry, ResourceRef, StorageEntry};
use super::parsing::{
    ParseError, Severity, ValidationIssue, enrich_validation_issues,
    parse_canonical_document_from_yaml,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationReportFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationSummary {
    pub total_issues: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl ValidationSummary {
    pub fn from_issues(issues: &[ValidationIssue]) -> Self {
        Self {
            total_issues: issues.len(),
            errors: issues
                .iter()
                .filter(|issue| issue.severity == Severity::Error)
                .count(),
            warnings: issues
                .iter()
                .filter(|issue| issue.severity == Severity::Warning)
                .count(),
            infos: issues
                .iter()
                .filter(|issue| issue.severity == Severity::Info)
                .count(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub summary: ValidationSummary,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn from_issues(issues: impl Into<Vec<ValidationIssue>>) -> Self {
        let issues = issues.into();
        let summary = ValidationSummary::from_issues(&issues);
        Self { summary, issues }
    }

    pub fn render_with<F>(&self, formatter: &F, format: ValidationReportFormat) -> String
    where
        F: ReportFormatter,
    {
        match format {
            ValidationReportFormat::Human => formatter.format_human(&self.issues),
            ValidationReportFormat::Json => formatter.format_json(&self.issues),
        }
    }
}

/// Trait for formatting validation issues into human-readable or machine-readable reports.
pub trait ReportFormatter {
    /// Generate a human-readable report.
    fn format_human(&self, issues: &[ValidationIssue]) -> String;

    /// Generate a JSON report.
    fn format_json(&self, issues: &[ValidationIssue]) -> String;
}

/// Default report formatter implementation.
pub struct DefaultReportFormatter;

impl DefaultReportFormatter {
    /// Create a new default formatter.
    pub fn new() -> Self {
        Self
    }

    /// Extract human-readable context from an issue for display.
    fn issue_context(&self, issue: &ValidationIssue) -> String {
        let mut lines = Vec::new();
        lines.push(format!("[{}] {}", issue.severity.as_str(), issue.reason));
        if let Some(line) = issue.line_number {
            lines.push(format!("  Line: {}", line));
        }
        if let Some(snippet) = &issue.source_snippet {
            lines.push("  Context:".to_string());
            for snippet_line in snippet.lines() {
                lines.push(format!("    {}", snippet_line));
            }
        }
        if let Some(hint) = &issue.remediation {
            lines.push(format!("  Fix: {}", hint));
        }
        lines.join("\n")
    }
}

impl Default for DefaultReportFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportFormatter for DefaultReportFormatter {
    fn format_human(&self, issues: &[ValidationIssue]) -> String {
        let summary = ValidationSummary::from_issues(issues);
        if issues.is_empty() {
            return "Validation passed (no issues).\n".to_string();
        }

        let mut output = format!("Validation Report: {} issue(s)\n", summary.total_issues);
        output.push_str(&"═".repeat(60));
        output.push('\n');

        // Group by severity
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect();
        let infos: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Info)
            .collect();

        if !errors.is_empty() {
            output.push_str(&format!("\nERRORS ({}):\n", errors.len()));
            for issue in errors {
                output.push_str(&format!("  • {}\n", issue.path));
                output.push_str(&format!("    {}\n", self.issue_context(issue)));
            }
        }

        if !warnings.is_empty() {
            output.push_str(&format!("\nWARNINGS ({}):\n", warnings.len()));
            for issue in warnings {
                output.push_str(&format!("  • {}\n", issue.path));
                output.push_str(&format!("    {}\n", self.issue_context(issue)));
            }
        }

        if !infos.is_empty() {
            output.push_str(&format!("\nINFO ({}):\n", infos.len()));
            for issue in infos {
                output.push_str(&format!("  • {}\n", issue.path));
                output.push_str(&format!("    {}\n", self.issue_context(issue)));
            }
        }

        output
    }

    fn format_json(&self, issues: &[ValidationIssue]) -> String {
        let summary = ValidationSummary::from_issues(issues);
        let issues_json: Vec<_> = issues
            .iter()
            .map(|issue| {
                let mut obj = json!({
                    "path": issue.path,
                    "reason": issue.reason,
                    "severity": issue.severity.as_str(),
                });
                if let Some(line) = issue.line_number {
                    obj["line_number"] = json!(line);
                }
                if let Some(snippet) = &issue.source_snippet {
                    obj["source_snippet"] = json!(snippet);
                }
                if let Some(hint) = &issue.remediation {
                    obj["remediation"] = json!(hint);
                }
                obj
            })
            .collect();

        let report = json!({
            "validation_report": {
                "summary": {
                    "total_issues": summary.total_issues,
                    "errors": summary.errors,
                    "warnings": summary.warnings,
                    "infos": summary.infos,
                },
                "total_issues": summary.total_issues,
                "errors": summary.errors,
                "warnings": summary.warnings,
                "infos": summary.infos,
                "issues": issues_json,
            }
        });

        serde_json::to_string_pretty(&report).unwrap_or_default()
    }
}

#[derive(Debug, Error)]
pub enum ConformanceError {
    #[error("invalid canonical yaml: {0}")]
    Parse(#[from] ParseError),
    #[error("validation failed with {0} issue(s)")]
    Validation(usize, Vec<ValidationIssue>),
}

impl ConformanceError {
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            Self::Validation(_, issues) => issues,
            Self::Parse(parse_error) => parse_error.issues(),
        }
    }

    pub fn report(&self) -> Option<ValidationReport> {
        let issues = self.issues();
        if issues.is_empty() {
            None
        } else {
            Some(ValidationReport::from_issues(issues.to_vec()))
        }
    }
}

pub fn validate_canonical_yaml(
    yaml: &str,
    filename: &Path,
) -> Result<CanonicalDocument, ConformanceError> {
    let doc = parse_canonical_document_from_yaml(yaml)?;
    match validate_canonical_document(&doc, filename) {
        Ok(()) => {}
        Err(ConformanceError::Validation(_, issues)) => {
            let issues = enrich_validation_issues(yaml, issues);
            return Err(ConformanceError::Validation(issues.len(), issues));
        }
        Err(other) => return Err(other),
    }
    Ok(doc)
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
        issues.push(
            ValidationIssue::new(
                "virtual_machine.system.memory.min",
                "must be an integer >= 0",
            )
            .with_remediation("Change memory.min to a non-negative value"),
        );
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
        issues.push(
            ValidationIssue::new(path, "is required and must be a non-empty string")
                .with_remediation(format!("Provide a non-empty string value for {}", path)),
        );
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
                issues.push(
                    ValidationIssue::new(
                        "metadata.vm_name",
                        format!("must match filename stem '{stem}'"),
                    )
                    .with_remediation(format!(
                        "Rename the file to {}.yaml or update vm_name to '{}'",
                        vm_name, stem
                    )),
                );
            }
        }
        _ => {
            issues.push(
                ValidationIssue::with_severity(
                    "metadata.vm_name",
                    "cannot validate vm_name because filename stem is unavailable",
                    Severity::Warning,
                )
                .with_remediation("Ensure the YAML file has a valid filename stem"),
            );
        }
    }
}

fn validate_machine_consistency(issues: &mut Vec<ValidationIssue>, family: &str, chipset: &str) {
    if family == "pc" {
        let allowed = ["q35", "i440fx"];
        if !allowed.contains(&chipset) {
            issues.push(
                ValidationIssue::new(
                    "virtual_machine.system.machine.chipset",
                    "must be one of [q35, i440fx] when machine family is 'pc'",
                )
                .with_remediation("Change chipset to 'q35' or 'i440fx' for pc family machines"),
            );
        }
    }
}

fn validate_unique_ids_storage(issues: &mut Vec<ValidationIssue>, entries: &[StorageEntry]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.storage[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(
                ValidationIssue::new(path, "is required and must be a non-empty string")
                    .with_remediation(format!(
                        "Provide a non-empty id string for storage entry at index {}",
                        idx
                    )),
            );
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(
                ValidationIssue::new(path, format!("duplicate id '{}'", entry.id))
                    .with_remediation(format!(
                        "Change the id to a unique value; '{}' is already used in storage",
                        entry.id
                    )),
            );
        }
    }
}

fn validate_unique_ids_network(issues: &mut Vec<ValidationIssue>, entries: &[NetworkEntry]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.network[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(
                ValidationIssue::new(path, "is required and must be a non-empty string")
                    .with_remediation(format!(
                        "Provide a non-empty id string for network entry at index {}",
                        idx
                    )),
            );
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(
                ValidationIssue::new(path, format!("duplicate id '{}'", entry.id))
                    .with_remediation(format!(
                        "Change the id to a unique value; '{}' is already used in network",
                        entry.id
                    )),
            );
        }
    }
}

fn validate_unique_ids_resources(issues: &mut Vec<ValidationIssue>, entries: &[ResourceRef]) {
    let mut seen = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let path = format!("virtual_machine.resources[{idx}].id");
        if entry.id.trim().is_empty() {
            issues.push(
                ValidationIssue::new(path, "is required and must be a non-empty string")
                    .with_remediation(format!(
                        "Provide a non-empty id string for resource entry at index {}",
                        idx
                    )),
            );
            continue;
        }
        if !seen.insert(entry.id.as_str()) {
            issues.push(
                ValidationIssue::new(path, format!("duplicate id '{}'", entry.id))
                    .with_remediation(format!(
                        "Change the id to a unique value; '{}' is already used in resources",
                        entry.id
                    )),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use super::super::parsing::{ParseError, Severity, ValidationIssue};
    use super::{
        ConformanceError, DefaultReportFormatter, ReportFormatter, ValidationReport,
        ValidationReportFormat, validate_canonical_yaml,
    };

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
            ConformanceError::Parse(ParseError::Validation(_, issues)) => {
                issues.into_iter().map(|i| (i.path, i.reason)).collect()
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    // CT-001: Valid canonical document with all required fields and valid machine model
    #[test]
    fn passing_canonical_example() {
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(valid_yaml(), filename);
        assert!(result.is_ok());
    }

    fn expect_issue<'a>(err: &'a ConformanceError, path: &str) -> &'a ValidationIssue {
        err.issues()
            .iter()
            .find(|issue| issue.path == path)
            .unwrap_or_else(|| panic!("expected issue for path {path}"))
    }

    // CT-002: Required field validation — rejects missing cpu.model
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
    // CT-004: vm_name must match filename stem

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
    // CT-003: Resource ID uniqueness enforcement within each scope (storage, network, resources)

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
    // CT-004: Machine policy enforces chipset consistency with machine family (pc → [q35, i440fx])

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
    // CT-002: Required field type validation; CT-005: Precise field path error reporting

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

    // CT-002-01: Collection field type mismatch reports the container path precisely
    #[test]
    fn collection_type_mismatch_reports_field_path() {
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
        id: "disk0"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);

        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.storage" && reason == "must be a list"
        }));
    }

    // CT-002-02: Malformed YAML is rejected before structural/conformance validation runs
    #[test]
    fn malformed_yaml_is_rejected_before_validation() {
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
            min: [8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail parsing");

        match err {
            ConformanceError::Parse(ParseError::Yaml(_)) => {}
            other => panic!("expected YAML parse error, got {other:?}"),
        }
    }
    // CT-003: Required ID validation; CT-005: Precise field path error reporting with index

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
    // CT-003: Duplicate detection targets offending entry; CT-005: Precise index error reporting

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
    // CT-003: ID uniqueness scoped to resource type; same ID allowed across storage/network/resources

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

    // CT-001-01: Optional sections completely omitted (storage, network, resources are optional)
    #[test]
    fn optional_sections_omitted_is_valid() {
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
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(yaml, filename);
        assert!(result.is_ok());
    }

    // CT-002-03: Multiple missing required fields reported with precise paths
    #[test]
    fn multiple_missing_required_fields() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "pc"
        cpu: {}
        memory:
            min: 8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        // Should report both missing cpu.model and missing machine.chipset
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.system.cpu.model" && reason == "is required"
        }));
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.system.machine.chipset" && reason == "is required"
        }));
        assert!(issues.len() >= 2);
    }

    // CT-002-04: Empty vm_name (boundary of required string field)
    #[test]
    fn empty_vm_name() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: ""
virtual_machine:
    system:
        machine:
            family: "pc"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: 8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.vm_name" && reason == "is required and must be a non-empty string"
        }));
    }

    // CT-002-05: Memory minimum boundary value = 0 (valid edge case)
    #[test]
    fn memory_minimum_zero_is_valid() {
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
            min: 0
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(yaml, filename);
        assert!(result.is_ok());
    }

    // CT-003-01: Multiple duplicates within a single scope (3+ identical IDs)
    #[test]
    fn multiple_duplicates_in_storage_scope() {
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
        - id: "disk0"
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        // Should report duplicates at indices 1 and 2
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.storage[1].id" && reason.contains("duplicate id")
        }));
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.storage[2].id" && reason.contains("duplicate id")
        }));
    }

    // CT-005-01: Empty schema_version (boundary test of required string field)
    #[test]
    fn empty_schema_version() {
        let yaml = r#"
metadata:
    schema_version: ""
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
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.schema_version"
                && reason == "is required and must be a non-empty string"
        }));
    }

    // CT-004-01: Machine family outside known set (boundary of family enum)
    #[test]
    fn unknown_machine_family() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    system:
        machine:
            family: "unknown-family"
            chipset: "q35"
        cpu:
            model: "host"
        memory:
            min: 8192
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = validate_canonical_yaml(yaml, filename);
        // Unknown family should pass (not "pc", so chipset constraint doesn't apply)
        assert!(result.is_ok());
    }

    // Reporter-001: Human-readable report formatting
    #[test]
    fn human_readable_report_formatting() {
        let issues = vec![
            ValidationIssue::new(
                "virtual_machine.system.memory.min",
                "must be an integer >= 0",
            )
            .with_source_snippet("   9 |     min: -1")
            .with_remediation("Change memory.min to a non-negative value"),
            ValidationIssue::with_severity(
                "metadata.vm_name",
                "must match filename stem",
                Severity::Warning,
            )
            .with_line_number(5),
        ];

        let formatter = DefaultReportFormatter::new();
        let report = formatter.format_human(&issues);

        assert!(report.contains("Validation Report: 2 issue(s)"));
        assert!(report.contains("ERRORS (1)"));
        assert!(report.contains("WARNINGS (1)"));
        assert!(report.contains("virtual_machine.system.memory.min"));
        assert!(report.contains("Change memory.min to a non-negative value"));
        assert!(report.contains("Line: 5"));
        assert!(report.contains("Context:"));
    }

    // Reporter-002: JSON report formatting preserves structure
    #[test]
    fn json_report_formatting() {
        let issues = vec![
            ValidationIssue::new(
                "metadata.vm_name",
                "is required and must be a non-empty string",
            )
            .with_remediation("Provide a non-empty string value for metadata.vm_name")
            .with_line_number(3),
        ];

        let formatter = DefaultReportFormatter::new();
        let json_report = formatter.format_json(&issues);

        let parsed: Value = serde_json::from_str(&json_report).expect("report must be valid json");
        assert_eq!(parsed["validation_report"]["summary"]["total_issues"], 1);
        assert_eq!(parsed["validation_report"]["summary"]["errors"], 1);
        assert_eq!(
            parsed["validation_report"]["issues"][0]["path"],
            "metadata.vm_name"
        );
        assert_eq!(
            parsed["validation_report"]["issues"][0]["reason"],
            "is required and must be a non-empty string"
        );
        assert_eq!(parsed["validation_report"]["issues"][0]["line_number"], 3);
        assert!(parsed["validation_report"]["issues"][0]["remediation"].is_string());
    }

    // Reporter-003: Empty issue list produces empty report
    #[test]
    fn empty_issue_list_formatting() {
        let issues = vec![];
        let formatter = DefaultReportFormatter::new();
        let report = formatter.format_human(&issues);
        assert!(report.contains("Validation passed (no issues)"));

        let json_report = formatter.format_json(&issues);
        let parsed: Value = serde_json::from_str(&json_report).expect("report must be valid json");
        assert_eq!(parsed["validation_report"]["summary"]["total_issues"], 0);
    }

    #[test]
    fn parse_validation_issue_includes_context_and_remediation() {
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
        let issue = expect_issue(&err, "virtual_machine.system.cpu.model");

        assert_eq!(
            issue.remediation.as_deref(),
            Some("Add the required string field at virtual_machine.system.cpu.model")
        );
        assert!(issue.line_number.is_some());
        let snippet = issue.source_snippet.as_deref().unwrap_or("");
        assert!(snippet.contains("cpu: {}"));
    }

    #[test]
    fn conformance_validation_issue_includes_context_and_report_helpers() {
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
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = validate_canonical_yaml(yaml, filename).expect_err("should fail validation");
        let issue = expect_issue(&err, "virtual_machine.storage[1].id");

        assert!(issue.line_number.is_some());
        let snippet = issue.source_snippet.as_deref().unwrap_or("");
        assert!(snippet.contains("- id: \"disk0\""));

        let report = err
            .report()
            .expect("validation issues should produce a report");
        let formatter = DefaultReportFormatter::new();
        let human = report.render_with(&formatter, ValidationReportFormat::Human);
        let json = report.render_with(&formatter, ValidationReportFormat::Json);

        assert!(human.contains("virtual_machine.storage[1].id"));
        assert!(human.contains("Context:"));

        let parsed: Value = serde_json::from_str(&json).expect("report must be valid json");
        assert_eq!(parsed["validation_report"]["summary"]["errors"], 1);
    }

    #[test]
    fn validation_report_summary_counts_by_severity() {
        let report = ValidationReport::from_issues(vec![
            ValidationIssue::new("metadata.vm_name", "is required"),
            ValidationIssue::with_severity(
                "metadata.schema_version",
                "deprecated",
                Severity::Warning,
            ),
            ValidationIssue::with_severity("metadata", "checked", Severity::Info),
        ]);

        assert_eq!(report.summary.total_issues, 3);
        assert_eq!(report.summary.errors, 1);
        assert_eq!(report.summary.warnings, 1);
        assert_eq!(report.summary.infos, 1);
    }
}
