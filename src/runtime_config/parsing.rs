//! Shared parsing/validation issue types.
//!
//! Concrete ezkvm YAML parsing lives in src/config_importer/ezkvm/parsing.rs.

use thiserror::Error;

/// Severity level for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warning => "WARNING",
            Self::Info => "INFO",
        }
    }
}

/// Rich validation issue with severity, context, and remediation guidance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub reason: String,
    pub severity: Severity,
    pub line_number: Option<usize>,
    pub source_snippet: Option<String>,
    pub remediation: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue with Error severity.
    pub fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
            severity: Severity::Error,
            line_number: None,
            source_snippet: None,
            remediation: None,
        }
    }

    /// Create a validation issue with specified severity.
    pub fn with_severity(
        path: impl Into<String>,
        reason: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
            severity,
            line_number: None,
            source_snippet: None,
            remediation: None,
        }
    }

    /// Set the line number.
    pub fn with_line_number(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    /// Set the source snippet.
    pub fn with_source_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.source_snippet = Some(snippet.into());
        self
    }

    /// Set remediation guidance.
    pub fn with_remediation(mut self, hint: impl Into<String>) -> Self {
        self.remediation = Some(hint.into());
        self
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("validation failed with {0} issue(s)")]
    Validation(usize, Vec<ValidationIssue>),
}

impl ParseError {
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            Self::Validation(_, issues) => issues,
            Self::Yaml(_) => &[],
        }
    }
}
