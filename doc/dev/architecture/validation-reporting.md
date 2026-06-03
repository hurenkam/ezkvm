# Validation Report Structure and Formatting

**Status:** Active | **Owner:** Hicks (Backend Dev) | **Date:** 2026-05-29

This document describes the validation reporting baseline currently implemented in `src/runtime_config/validation.rs`, including severity classification, remediation guidance, and machine-readable output formats.

## Overview

Validation issues are reported with rich context to support both human operators and automated tools:

- **Severity Levels**: Error (fatal), Warning (advisory), Info (informational)
- **Context**: Optional line numbers, optional source snippets, remediation hints
- **Formats**: Human-readable console output and JSON structured reports

Context extraction is best-effort and is derived from canonical YAML field paths. When the exact leaf is absent, the report falls back to the closest parent mapping or list item so the operator still gets actionable nearby context.

The canonical YAML parser and conformance validator currently populate severity and remediation consistently. `line_number` and `source_snippet` are supported by the shared type and formatter, but are not yet derived from YAML source text in the core validation path.

---

## Severity Levels

### Error
**When to use:** Validation failures that prevent runtime execution.

**Examples:**
- Missing required fields (e.g., `vm_name`)
- Type violations (string expected, got integer)
- Duplicate IDs within a scope (storage, network, resources)
- Machine chipset inconsistent with family (pc family requires q35 or i440fx)
- Constraint violations (e.g., memory minimum < 0)

**Impact:** Stops validation, must be fixed before proceeding.

```json
{
  "path": "virtual_machine.system.memory.min",
  "reason": "must be an integer >= 0",
  "severity": "ERROR",
  "remediation": "Change memory.min to a non-negative value"
}
```

### Warning
**When to use:** Issues that are valid but potentially problematic or deprecated.

**Examples:**
- Unable to validate vm_name against filename (filename unavailable)
- Deprecated field usage (reserved for future validation rules)
- Optional fields with non-standard values

**Impact:** Does not block execution; operator should review.

```json
{
  "path": "metadata.vm_name",
  "reason": "cannot validate vm_name because filename stem is unavailable",
  "severity": "WARNING",
  "remediation": "Ensure the YAML file has a valid filename stem"
}
```

### Info
**When to use:** Informational notes that do not affect validation correctness.

**Examples:**
- Formal validation milestones or checkpoints
- Informational annotations from import adapters
- Notes about optional fields being used

**Impact:** Visible in reports but does not affect correctness.

**Current implementation note:** The shared `Severity` enum supports `Info`, but the canonical YAML validation path currently emits `Error` and `Warning` severities only.

---

## ValidationIssue Structure

### Core Fields
```rust
pub struct ValidationIssue {
    pub path: String,                           // Field path (e.g., "metadata.vm_name")
    pub reason: String,                         // Error description
    pub severity: Severity,                     // Error | Warning | Info
    pub line_number: Option<usize>,             // Source line (if available)
    pub source_snippet: Option<String>,         // Code snippet for context
    pub remediation: Option<String>,            // Guidance to fix the issue
}
```

### Field Semantics

| Field | Purpose | Example |
|-------|---------|---------|
| `path` | Navigation to the offending field | `virtual_machine.storage[1].id` |
| `reason` | What went wrong | `duplicate id 'disk0'` |
| `severity` | Classification | `Error`, `Warning`, `Info` |
| `line_number` | YAML source line | `5` |
| `source_snippet` | Raw code context | `id: ""` |
| `remediation` | How to fix | `Change id to a unique value` |

### Context Extraction Rules

- Semantic validation (`validate_canonical_yaml`) enriches issues after parsing, using the original YAML source.
- Structural validation (`ParseError::Validation`) enriches issues before the parse error is returned.
- Missing-field errors resolve to the nearest present parent block.
- Sequence item paths like `virtual_machine.storage[1].id` resolve to the offending list item.

---

## Report Formats

Syntax-invalid YAML is returned as `ParseError::Yaml` before `ValidationIssue` formatting applies. The formatter examples below cover issue-based parse/conformance failures and warning-only outcomes.

### Human-Readable Report

Designed for console output with severity grouping and indentation.

```
Validation Report: 3 issue(s)
════════════════════════════════════════════════════════

ERRORS (2):
  • virtual_machine.system.memory.min
    [ERROR] must be an integer >= 0
    Context:
         8 |     memory:
         9 |       min: -1
    Fix: Change memory.min to a non-negative value

  • virtual_machine.storage[1].id
    [ERROR] duplicate id 'disk0'
    Line: 12
    Context:
        11 |   storage:
        12 |     - id: "disk0"
        13 |     - id: "disk0"
    Fix: Change the id to a unique value; 'disk0' is already used in storage

WARNINGS (1):
  • metadata.vm_name
    [WARNING] cannot validate vm_name because filename stem is unavailable
    Fix: Ensure the YAML file has a valid filename stem
```

**Usage:**
```rust
let formatter = DefaultReportFormatter::new();
let report = formatter.format_human(&issues);
println!("{}", report);
```

### JSON Report

Machine-readable structured format for programmatic processing.

```json
{
  "validation_report": {
    "summary": {
      "total_issues": 3,
      "errors": 2,
      "warnings": 1,
      "infos": 0
    },
    "total_issues": 3,
    "errors": 2,
    "warnings": 1,
    "infos": 0,
    "issues": [
      {
        "path": "virtual_machine.system.memory.min",
        "reason": "must be an integer >= 0",
        "severity": "ERROR",
        "line_number": 9,
        "remediation": "Change memory.min to a non-negative value"
      },
      {
        "path": "virtual_machine.storage[1].id",
        "reason": "duplicate id 'disk0'",
        "severity": "ERROR",
        "line_number": 12,
        "source_snippet": "- id: \"disk0\"",
        "remediation": "Change the id to a unique value; 'disk0' is already used in storage"
      },
      {
        "path": "metadata.vm_name",
        "reason": "cannot validate vm_name because filename stem is unavailable",
        "severity": "WARNING",
        "remediation": "Ensure the YAML file has a valid filename stem"
      }
    ]
  }
}
```

**Usage:**
```rust
let formatter = DefaultReportFormatter::new();
let json_report = formatter.format_json(&issues);
// Pipe to file or log system
```

---

## Remediation Hint Strategies

Remediation hints guide operators toward fixes without prescribing implementation.

### Strategy 1: Direct Action
Specify what to change and how.

```
"remediation": "Change chipset to 'q35' or 'i440fx' for pc family machines"
```

### Strategy 2: Field-Dependent Context
Tailor hints based on field constraints.

```
"remediation": "Provide a non-empty string value for metadata.vm_name"
```

### Strategy 3: Scoped Conflict Resolution
Explain scope rules for duplicate detection.

```
"remediation": "Change the id to a unique value; 'gpu0' is already used in resources"
```

### Strategy 4: Validation Prerequisite
Explain dependencies.

```
"remediation": "Add the required 'memory.min' field to virtual_machine.system"
```

---

## Implementation Patterns

### Creating Issues with Severity
```rust
// Error with remediation
issues.push(
    ValidationIssue::new(path, "field is required")
        .with_remediation("Provide a non-empty value")
);

// Warning with context
issues.push(
    ValidationIssue::with_severity(path, "field is deprecated", Severity::Warning)
        .with_line_number(5)
        .with_remediation("Use the new field instead")
);
```

### Implementing Custom Formatters
```rust
pub trait ReportFormatter {
    fn format_human(&self, issues: &[ValidationIssue]) -> String;
    fn format_json(&self, issues: &[ValidationIssue]) -> String;
}
```

```rust
pub struct ValidationReport {
  pub summary: ValidationSummary,
  pub issues: Vec<ValidationIssue>,
}
```

`ValidationReport` is the stable container for callers that want severity counts alongside the issue list before rendering.

Implementers may:
- Add color / styling to human output
- Filter issues by severity
- Transform JSON structure for specific consumers
- Add custom metadata (timestamps, validation duration, etc.)

---

## Testing Validation Reports

### Test Checklist
- [x] Human report grouping covered by `human_readable_report_formatting`
- [x] JSON structure covered by `json_report_formatting`
- [x] Empty issue list covered by `empty_issue_list_formatting`
- [ ] Parser-derived line numbers and source snippets
- [ ] Additional warning/info producers beyond current canonical validation cases

### Example Test
```rust
#[test]
fn human_readable_report_formatting() {
    let issues = vec![
        ValidationIssue::new("field", "is required")
            .with_remediation("Provide a value"),
    ];
    let formatter = DefaultReportFormatter::new();
    let report = formatter.format_human(&issues);
    
    assert!(report.contains("ERRORS (1)"));
    assert!(report.contains("Provide a value"));
}
```

---

## Related Documentation

- [Validation Examples](validation-examples.md) — Common errors and fixes
- [Ezkvm Machine Model Policy](ezkvm-machine-model-policy.md) — Machine constraints
- [Coding Guidelines](coding-guidelines.md) — Module organization
