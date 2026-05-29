//! YAML parsing and document assembly for canonical VM specification.
//!
//! This module owns the extraction of raw YAML structure into the canonical model,
//! prior to semantic validation.

use std::ops::Range;

use serde_yaml::{Mapping, Value};
use thiserror::Error;

use super::model::{
    CanonicalDocument, Cpu, Machine, Memory, Metadata, NetworkEntry, ResourceRef, StorageEntry,
    System, VirtualMachine,
};

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

pub fn parse_canonical_document_from_yaml(yaml: &str) -> Result<CanonicalDocument, ParseError> {
    let root: Value = serde_yaml::from_str(yaml)?;
    parse_canonical_document(&root).map_err(|err| match err {
        ParseError::Validation(_, issues) => {
            let issues = enrich_validation_issues(yaml, issues);
            ParseError::Validation(issues.len(), issues)
        }
        other => other,
    })
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
            issues.push(
                ValidationIssue::with_severity(path, "must be a mapping", Severity::Error)
                    .with_remediation(format!("Ensure {} is a YAML mapping/object", path)),
            );
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
            issues.push(
                ValidationIssue::with_severity(path, "must be a mapping", Severity::Error)
                    .with_remediation(format!("Ensure {} is a YAML mapping/object", path)),
            );
            None
        }
        None => {
            issues.push(
                ValidationIssue::new(path, "is required")
                    .with_remediation(format!("Add the required mapping at {}", path)),
            );
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
            issues.push(
                ValidationIssue::with_severity(path, "must be a string", Severity::Error)
                    .with_remediation(format!("Change {} to a string value", path)),
            );
            None
        }
        None => {
            issues.push(
                ValidationIssue::new(path, "is required")
                    .with_remediation(format!("Add the required string field at {}", path)),
            );
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
                issues.push(
                    ValidationIssue::with_severity(path, "must be an integer", Severity::Error)
                        .with_remediation(format!(
                            "Change {} to an integer value (not a float)",
                            path
                        )),
                );
                None
            }
        }
        Some(_) => {
            issues.push(
                ValidationIssue::with_severity(path, "must be an integer", Severity::Error)
                    .with_remediation(format!("Change {} to an integer value", path)),
            );
            None
        }
        None => {
            issues.push(
                ValidationIssue::new(path, "is required")
                    .with_remediation(format!("Add the required integer field at {}", path)),
            );
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
        issues.push(
            ValidationIssue::with_severity(path, "must be a list", Severity::Error)
                .with_remediation(format!("Change {} to a YAML list/array", path)),
        );
        return Vec::new();
    };

    let mut out = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let item_path = format!("{path}[{idx}]");
        let Value::Mapping(entry_map) = item else {
            issues.push(
                ValidationIssue::with_severity(&item_path, "must be a mapping", Severity::Error)
                    .with_remediation(format!("Ensure each item in {} is a YAML mapping", path)),
            );
            continue;
        };

        if let Some(id) = required_string(entry_map, "id", &format!("{path}[{idx}].id"), issues) {
            out.push(T::from(id));
        }
    }

    out
}

pub(crate) fn enrich_validation_issues(
    yaml: &str,
    issues: Vec<ValidationIssue>,
) -> Vec<ValidationIssue> {
    let lines = YamlLines::from_source(yaml);

    issues
        .into_iter()
        .map(|issue| lines.attach_context(issue))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceContext {
    line_number: usize,
    snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct YamlLine<'a> {
    line_number: usize,
    indent: usize,
    content: &'a str,
    trimmed: &'a str,
}

#[derive(Debug, Clone)]
struct YamlLines<'a> {
    lines: Vec<YamlLine<'a>>,
}

impl<'a> YamlLines<'a> {
    fn from_source(yaml: &'a str) -> Self {
        let lines = yaml
            .lines()
            .enumerate()
            .map(|(index, raw_line)| {
                let indent = raw_line.chars().take_while(|ch| *ch == ' ').count();
                YamlLine {
                    line_number: index + 1,
                    indent,
                    content: raw_line,
                    trimmed: raw_line.trim(),
                }
            })
            .collect();

        Self { lines }
    }

    fn attach_context(&self, mut issue: ValidationIssue) -> ValidationIssue {
        if issue.line_number.is_some() && issue.source_snippet.is_some() {
            return issue;
        }

        let Some(context) = self.locate_path(&issue.path) else {
            return issue;
        };

        if issue.line_number.is_none() {
            issue = issue.with_line_number(context.line_number);
        }
        if issue.source_snippet.is_none() {
            issue = issue.with_source_snippet(context.snippet);
        }

        issue
    }

    fn locate_path(&self, path: &str) -> Option<SourceContext> {
        let segments = parse_issue_path(path);
        if segments.is_empty() {
            return self
                .first_contentful_line()
                .map(|index| self.context_for_index(index));
        }

        let mut scope = 0..self.lines.len();
        let mut parent_indent: isize = -1;
        let mut anchor_index = self.first_contentful_line()?;

        for segment in segments {
            match segment {
                PathSegment::Key(key) => {
                    if let Some(index) = self.find_key_in_scope(&scope, parent_indent, &key) {
                        anchor_index = index;
                        parent_indent = self.lines[index].indent as isize;
                        scope = self.subtree_range(index);
                    } else {
                        return Some(self.context_for_index(anchor_index));
                    }
                }
                PathSegment::Index(target) => {
                    if let Some(index) = self.find_list_item_in_scope(&scope, parent_indent, target)
                    {
                        anchor_index = index;
                        parent_indent = self.lines[index].indent as isize;
                        scope = self.subtree_range(index);
                    } else {
                        return Some(self.context_for_index(anchor_index));
                    }
                }
            }
        }

        Some(self.context_for_index(anchor_index))
    }

    fn first_contentful_line(&self) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| !is_ignorable_line(line.trimmed))
    }

    fn find_key_in_scope(
        &self,
        scope: &Range<usize>,
        parent_indent: isize,
        key: &str,
    ) -> Option<usize> {
        scope
            .clone()
            .filter(|index| {
                let line = &self.lines[*index];
                !is_ignorable_line(line.trimmed) && line.indent as isize > parent_indent
            })
            .find(|index| line_key(self.lines[*index].trimmed) == Some(key))
    }

    fn find_list_item_in_scope(
        &self,
        scope: &Range<usize>,
        parent_indent: isize,
        target_index: usize,
    ) -> Option<usize> {
        let mut direct_item_indent = None;
        let mut seen = 0usize;

        for index in scope.clone() {
            let line = &self.lines[index];
            if is_ignorable_line(line.trimmed) || line.indent as isize <= parent_indent {
                continue;
            }
            if !line.trimmed.starts_with("-") {
                continue;
            }

            let item_indent = *direct_item_indent.get_or_insert(line.indent);
            if line.indent != item_indent {
                continue;
            }

            if seen == target_index {
                return Some(index);
            }
            seen += 1;
        }

        None
    }

    fn subtree_range(&self, start_index: usize) -> Range<usize> {
        let start_indent = self.lines[start_index].indent;
        let mut end_index = self.lines.len();

        for index in (start_index + 1)..self.lines.len() {
            let line = &self.lines[index];
            if is_ignorable_line(line.trimmed) {
                continue;
            }
            if line.indent <= start_indent {
                end_index = index;
                break;
            }
        }

        start_index..end_index
    }

    fn context_for_index(&self, index: usize) -> SourceContext {
        let mut snippet_indexes = Vec::new();

        if let Some(previous) = self.previous_contentful_line(index) {
            snippet_indexes.push(previous);
        }
        snippet_indexes.push(index);
        if let Some(next) = self.next_contentful_line(index) {
            snippet_indexes.push(next);
        }

        snippet_indexes.sort_unstable();
        snippet_indexes.dedup();

        let snippet = snippet_indexes
            .into_iter()
            .map(|line_index| {
                let line = &self.lines[line_index];
                format!("{:>4} | {}", line.line_number, line.content)
            })
            .collect::<Vec<_>>()
            .join("\n");

        SourceContext {
            line_number: self.lines[index].line_number,
            snippet,
        }
    }

    fn previous_contentful_line(&self, start_index: usize) -> Option<usize> {
        (0..start_index)
            .rev()
            .find(|index| !is_ignorable_line(self.lines[*index].trimmed))
    }

    fn next_contentful_line(&self, start_index: usize) -> Option<usize> {
        ((start_index + 1)..self.lines.len())
            .find(|index| !is_ignorable_line(self.lines[*index].trimmed))
    }
}

fn parse_issue_path(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();

    for raw_segment in path.split('.') {
        if raw_segment.is_empty() || raw_segment == "$" {
            continue;
        }

        let mut remainder = raw_segment;
        while let Some(open_bracket) = remainder.find('[') {
            let key = &remainder[..open_bracket];
            if !key.is_empty() {
                segments.push(PathSegment::Key(key.to_owned()));
            }

            let Some(close_bracket) = remainder[open_bracket + 1..].find(']') else {
                break;
            };
            let index_text = &remainder[open_bracket + 1..open_bracket + 1 + close_bracket];
            if let Ok(index) = index_text.parse::<usize>() {
                segments.push(PathSegment::Index(index));
            }

            remainder = &remainder[open_bracket + close_bracket + 2..];
        }

        if !remainder.is_empty() {
            segments.push(PathSegment::Key(remainder.to_owned()));
        }
    }

    segments
}

fn line_key(trimmed: &str) -> Option<&str> {
    if is_ignorable_line(trimmed) {
        return None;
    }

    let candidate = if let Some(rest) = trimmed.strip_prefix("-") {
        rest.trim_start()
    } else {
        trimmed
    };

    let (key, _) = candidate.split_once(':')?;
    let key = key.trim();
    if key.is_empty() { None } else { Some(key) }
}

fn is_ignorable_line(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed.starts_with('#')
}
