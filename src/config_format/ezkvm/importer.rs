//! ezkvm YAML importer for loading and validating runtime configurations.
//!
//! Related documentation:
//! - src/README.md
//! - doc/dev/architecture/design/vm-spec-parsing-validation.md

use serde_yaml::from_str;
use std::path::Path;

use crate::config_format::{EzkvmImporter, ImportError, ImportOptions, Importer, RuntimeConfig};
use crate::runtime_config::{ConformanceError, ParseError};

use super::diagnostics::enrich_validation_issues;

/// Arguments required to import an ezkvm YAML configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EzkvmInputArgs {
    /// Path to the host configuration file used during import.
    #[serde(rename = "input.host")]
    pub input_host: String,
    /// Path to the ezkvm VM configuration file.
    #[serde(rename = "input.vm")]
    pub input_vm: String,
}

impl EzkvmImporter {
    /// Validates ezkvm YAML and returns a canonical runtime configuration.
    ///
    /// # Arguments
    ///
    /// * `yaml` - Source YAML text to parse and validate.
    /// * `filename` - Source filename used for validation context.
    ///
    /// # Returns
    ///
    /// A validated runtime configuration or a conformance error with context.
    fn validate(yaml: &str, filename: &Path) -> Result<RuntimeConfig, ConformanceError> {
        let doc = from_str::<RuntimeConfig>(yaml).map_err(ParseError::from)?;
        match doc.validate_runtime_config(filename) {
            Ok(()) => {}
            Err(ConformanceError::Validation(_, issues)) => {
                let issues = enrich_validation_issues(yaml, issues);
                return Err(ConformanceError::Validation(issues.len(), issues));
            }
            Err(other) => return Err(other),
        }
        Ok(doc)
    }
}

impl Importer for EzkvmImporter {
    /// Imports an ezkvm YAML configuration into the canonical runtime model.
    fn import(&self, args: ImportOptions) -> Result<RuntimeConfig, ImportError> {
        let (_host_path, vm_path) = match args {
            ImportOptions::Ezkvm { host, vm } => (host, vm),
            _ => return Err(ImportError::InvalidFormat),
        };
        let source_text = std::fs::read_to_string(&vm_path)
            .map_err(|e| ImportError::ImportFailed(format!("{}: {}", vm_path, e)))?;
        Self::validate(&source_text, Path::new(&vm_path))
            .map_err(|e| ImportError::ImportFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::Value;

    use crate::{
        config_format::EzkvmImporter,
        runtime_config::{
            ConformanceError, DefaultReportFormatter, ParseError, ReportFormatter, Severity,
            ValidationIssue, ValidationReport, ValidationReportFormat,
        },
    };

    /// Returns a valid ezkvm YAML document used by multiple tests.
    fn valid_yaml() -> &'static str {
        r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 8589934592
    devices: []
resources: []
"#
    }

    /// Collects validation issue paths and reasons from a failed validation result.
    fn expect_validation_issues(err: ConformanceError) -> Vec<(String, String)> {
        match err {
            ConformanceError::Validation(_, issues) => issues
                .into_iter()
                .map(|issue| (issue.path, issue.reason))
                .collect(),
            ConformanceError::Parse(ParseError::Validation(_, issues)) => issues
                .into_iter()
                .map(|issue| (issue.path, issue.reason))
                .collect(),
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    /// Extracts the YAML parse error message from a conformance error.
    fn expect_yaml_parse_error(err: ConformanceError) -> String {
        match err {
            ConformanceError::Parse(ParseError::Yaml(parse_error)) => parse_error.to_string(),
            other => panic!("expected YAML parse error, got {other:?}"),
        }
    }

    /// Returns a single validation issue for the requested path.
    fn expect_issue<'a>(err: &'a ConformanceError, path: &str) -> &'a ValidationIssue {
        err.issues()
            .iter()
            .find(|issue| issue.path == path)
            .unwrap_or_else(|| panic!("expected issue for path {path}"))
    }

    #[test]
    /// Accepts a valid ezkvm YAML document.
    fn passing_runtime_config_example() {
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = EzkvmImporter::validate(valid_yaml(), filename);
        assert!(result.is_ok());
    }

    #[test]
    /// Reports missing required top-level virtual machine fields.
    fn missing_required_field() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");
        let message = expect_yaml_parse_error(err);
        assert!(message.contains("virtual_machine"));
        assert!(message.contains("missing field `machine`"));
    }

    #[test]
    /// Flags a vm name mismatch against the source filename stem.
    fn vm_name_mismatch() {
        let filename = Path::new("/tmp/another-name.yaml");
        let err =
            EzkvmImporter::validate(valid_yaml(), filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.vm_name" && reason.contains("filename stem 'another-name'")
        }));
    }

    #[test]
    /// Rejects invalid machine and chipset combinations.
    fn invalid_chipset_family_combination() {
        let yaml = r#"
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "arm-virt"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "virtual_machine.machine.chipset" && reason.contains("[q35, i440fx]")
        }));
    }

    #[test]
    /// Reports numeric type mismatches with the exact field path.
    fn invalid_type_reports_field_path() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: "8589934592"
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");
        let message = expect_yaml_parse_error(err);
        assert!(message.contains("virtual_machine.memory.size"));
        assert!(message.contains("expected usize"));
    }

    #[test]
    /// Reports sequence type mismatches with the exact field path.
    fn collection_type_mismatch_reports_field_path() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 8589934592
    devices:
        id: "dev0"
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");
        let message = expect_yaml_parse_error(err);
        assert!(message.contains("virtual_machine.devices"));
        assert!(message.contains("expected a sequence"));
    }

    #[test]
    /// Rejects malformed YAML before validation runs.
    fn malformed_yaml_is_rejected_before_validation() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: [8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");

        match err {
            ConformanceError::Parse(ParseError::Yaml(_)) => {}
            other => panic!("expected YAML parse error, got {other:?}"),
        }
    }

    #[test]
    fn optional_cpu_omitted_is_valid() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = EzkvmImporter::validate(yaml, filename);
        assert!(result.is_ok());
    }

    #[test]
    fn multiple_missing_required_fields() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");
        let message = expect_yaml_parse_error(err);
        assert!(message.contains("virtual_machine.machine"));
        assert!(message.contains("missing field `chipset`"));
    }

    #[test]
    fn empty_vm_name() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: ""
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.vm_name" && reason == "is required and must be a non-empty string"
        }));
    }

    #[test]
    fn memory_size_zero_is_valid() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 0
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = EzkvmImporter::validate(yaml, filename);
        assert!(result.is_ok());
    }

    #[test]
    fn empty_schema_version() {
        let yaml = r#"
metadata:
    schema_version: ""
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail validation");
        let issues = expect_validation_issues(err);
        assert!(issues.iter().any(|(path, reason)| {
            path == "metadata.schema_version"
                && reason == "is required and must be a non-empty string"
        }));
    }

    #[test]
    fn unknown_machine_family() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "win11-dev"
virtual_machine:
    machine:
        family: "unknown-family"
        chipset: "q35"
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let result = EzkvmImporter::validate(yaml, filename);
        assert!(result.is_ok());
    }

    #[test]
    fn keyed_untagged_enum_yaml_is_accepted() {
        let yaml = r#"
metadata:
    schema_version: "1.0.0"
    vm_name: "workstation-01"
virtual_machine:
    machine:
        family: "pc"
        chipset: "q35"
    cpu:
        model: "Host"
        cores: 1
        threads: 1
        sockets: 1
    memory:
        size: 17179869184
    devices:
        - sata:
            type: ssd
            resource: "disk0"
        - pcie:
            bus: 0
            device: 1
            function: 2
            type: virtio_net
            resource: "net0"
resources:
    - storage:
        block_device: "/dev/vm/disk0"
      id: "disk0"
    - network:
        bridge: "vmbr0"
      id: "net0"
"#;
        let filename = Path::new("/tmp/workstation-01.yaml");
        let result = EzkvmImporter::validate(yaml, filename);
        if let Err(ref err) = result {
            panic!("expected keyed untagged yaml to parse, got: {err:?}");
        }
        assert!(result.is_ok());
    }

    #[test]
    fn human_readable_report_formatting() {
        let issues = vec![
            ValidationIssue::new(
                "virtual_machine.memory.size",
                "must be an integer >= 0 bytes",
            )
            .with_source_snippet("   9 |     size: -1")
            .with_remediation("Change memory.size to a non-negative value"),
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
        assert!(report.contains("virtual_machine.memory.size"));
        assert!(report.contains("Change memory.size to a non-negative value"));
        assert!(report.contains("Line: 5"));
        assert!(report.contains("Context:"));
    }

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
    memory:
        size: 8589934592
    devices: []
resources: []
"#;
        let filename = Path::new("/tmp/win11-dev.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail parsing");
        let message = expect_yaml_parse_error(err);

        assert!(message.contains("virtual_machine"));
        assert!(message.contains("missing field `machine`"));
    }

    #[test]
    fn conformance_validation_issue_includes_context_and_report_helpers() {
        let yaml = valid_yaml();
        let filename = Path::new("/tmp/other-name.yaml");
        let err = EzkvmImporter::validate(yaml, filename).expect_err("should fail validation");
        let issue = expect_issue(&err, "metadata.vm_name");

        let snippet = issue.source_snippet.as_deref().unwrap_or("");
        if !snippet.is_empty() {
            assert!(snippet.contains("vm_name"));
        }

        let report = err
            .report()
            .expect("validation issues should produce a report");
        let formatter = DefaultReportFormatter::new();
        let human = report.render_with(&formatter, ValidationReportFormat::Human);
        let json = report.render_with(&formatter, ValidationReportFormat::Json);

        assert!(human.contains("metadata.vm_name"));

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
