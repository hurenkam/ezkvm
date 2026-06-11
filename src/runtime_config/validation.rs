use std::path::{Path, PathBuf};

use serde_json::json;
use thiserror::Error;

use super::model::RuntimeConfig;
use super::parsing::{ParseError, Severity, ValidationIssue};

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

impl RuntimeConfig {
    pub fn validate_runtime(&self, source_path: Option<&Path>) -> Result<(), String> {
        let fallback = PathBuf::from(format!("{}.yaml", self.metadata.vm_name));
        let path = source_path.unwrap_or(&fallback);

        self.validate_runtime_config(path)
            .map_err(|error| match error.report() {
                Some(report) => {
                    let formatter = DefaultReportFormatter::new();
                    report.render_with(&formatter, ValidationReportFormat::Human)
                }
                None => error.to_string(),
            })
    }

    pub fn validate_runtime_config(&self, filename: &Path) -> Result<(), ConformanceError> {
        let mut issues = Vec::new();

        check_required_strings(
            &mut issues,
            "metadata.schema_version",
            &self.metadata.schema_version,
        );
        check_required_strings(&mut issues, "metadata.vm_name", &self.metadata.vm_name);
        check_required_strings(
            &mut issues,
            "virtual_machine.machine.family",
            &self.virtual_machine.machine.family,
        );
        check_required_strings(
            &mut issues,
            "virtual_machine.machine.chipset",
            &self.virtual_machine.machine.chipset,
        );

        validate_vm_name_filename_match(&mut issues, &self.metadata.vm_name, filename);
        validate_machine_consistency(
            &mut issues,
            &self.virtual_machine.machine.family,
            &self.virtual_machine.machine.chipset,
        );

        if issues.is_empty() {
            Ok(())
        } else {
            Err(ConformanceError::Validation(issues.len(), issues))
        }
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
                    "virtual_machine.machine.chipset",
                    "must be one of [q35, i440fx] when machine family is 'pc'",
                )
                .with_remediation("Change chipset to 'q35' or 'i440fx' for pc family machines"),
            );
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
    #[error("invalid ezkvm config yaml: {0}")]
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

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::super::parsing::{Severity, ValidationIssue};
    use super::{DefaultReportFormatter, ReportFormatter, ValidationReport};

    // Reporter-001: Human-readable report formatting
    #[test]
    fn human_readable_report_formatting() {
        let issues = vec![
            ValidationIssue::new("virtual_machine.memory.size", "must be an integer >= 0")
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
