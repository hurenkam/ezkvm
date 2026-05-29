# VM Spec, Parsing, And Validation

Back to index: [Current Implementation Architecture](./current-implementation-architecture.md)

## Scope

This note covers canonical model shape plus parse and validation logic in:

- `src/vm_spec/model.rs`
- `src/vm_spec/parsing.rs`
- `src/vm_spec/validation.rs`
- `src/vm_spec/mod.rs`

## Canonical Model

`CANONICAL_SCHEMA_VERSION` is currently `1.0.0`.
Top-level schema expects `metadata` and `virtual_machine`. `storage`, `network`, and `resources` default to empty vectors.

```plantuml
@startuml
skinparam classAttributeIconSize 0

class CanonicalDocument << (S,#98FB98) >> {
  +metadata: Metadata
  +virtual_machine: VirtualMachine
}

class Metadata << (S,#98FB98) >> {
  +schema_version: String
  +vm_name: String
}

class VirtualMachine << (S,#98FB98) >> {
  +system: System
  +storage: Vec<StorageEntry>
  +network: Vec<NetworkEntry>
  +resources: Vec<ResourceRef>
}

class System << (S,#98FB98) >> {
  +machine: Machine
  +cpu: Cpu
  +memory: Memory
}

class Machine << (S,#98FB98) >> {
  +family: String
  +chipset: String
}

class Cpu << (S,#98FB98) >> {
  +model: String
}

class Memory << (S,#98FB98) >> {
  +min: i64
}

class StorageEntry << (S,#98FB98) >> {
  +id: String
}

class NetworkEntry << (S,#98FB98) >> {
  +id: String
}

class ResourceRef << (S,#98FB98) >> {
  +id: String
}

CanonicalDocument *-- Metadata
CanonicalDocument *-- VirtualMachine
VirtualMachine *-- System
VirtualMachine o-- StorageEntry
VirtualMachine o-- NetworkEntry
VirtualMachine o-- ResourceRef
System *-- Machine
System *-- Cpu
System *-- Memory
@enduml
```

## Parsing Surface

`parse_canonical_document_from_yaml()` parses YAML text to `serde_yaml::Value` and then feeds structural extraction through `parse_canonical_document()`, collecting `ValidationIssue` values where possible.

```plantuml
@startuml
skinparam classAttributeIconSize 0

enum Severity << (S,#98FB98) >> {
  Error
  Warning
  Info
}

class ValidationIssue << (S,#98FB98) >> {
  +path: String
  +reason: String
  +severity: Severity
  +line_number: Option<usize>
  +source_snippet: Option<String>
  +remediation: Option<String>
}

class ParseError << (S,#98FB98) >> {
}

class CanonicalDocument << (S,#98FB98) >>

class "parse_canonical_document_from_yaml()" as ParseYaml << (F,#DDA0DD) >>
class "parse_canonical_document()" as ParseValue << (F,#DDA0DD) >>
class "enrich_validation_issues()" as Enrich << (F,#DDA0DD) >>

ParseYaml ..> ParseValue
ParseValue ..> CanonicalDocument
ParseValue ..> ValidationIssue
ParseValue ..> ParseError
Enrich ..> ValidationIssue
ValidationIssue --> Severity
@enduml
```

## Semantic Validation

`validate_canonical_document()` applies whole-document checks including:

- `metadata.vm_name` matching filename stem when available
- consistency between `machine.family` and `machine.chipset`
- per-scope uniqueness for `storage`, `network`, `resources` IDs
- non-empty required string fields
- non-negative minimum memory

`validate_canonical_yaml()` is the high-level path that parses first, validates second, and enriches issue context before returning `ConformanceError` when needed.

```plantuml
@startuml
skinparam classAttributeIconSize 0

class ValidationSummary << (S,#98FB98) >> {
  +total_issues: usize
  +errors: usize
  +warnings: usize
  +infos: usize
  +from_issues(issues)
}

class ValidationReport << (S,#98FB98) >> {
  +summary: ValidationSummary
  +issues: Vec<ValidationIssue>
  +from_issues(issues)
  +render_with(formatter, format)
}

interface ReportFormatter << (T,#FFB347) >> {
  +format_human(issues) -> String
  +format_json(issues) -> String
}

class DefaultReportFormatter << (S,#98FB98) >> {
  +new() -> Self
  +format_human(issues) -> String
  +format_json(issues) -> String
}

class ConformanceError << (S,#98FB98) >> {
  +issues() -> &[ValidationIssue]
  +report() -> Option<ValidationReport>
}

class ValidationIssue << (S,#98FB98) >>
class ParseError << (S,#98FB98) >>

ReportFormatter <|.. DefaultReportFormatter
ConformanceError --> ParseError : wraps
ConformanceError --> ValidationIssue : exposes issues
ValidationReport *-- ValidationSummary
ValidationReport o-- ValidationIssue
DefaultReportFormatter ..> ValidationIssue
DefaultReportFormatter ..> ValidationSummary
@enduml
```

## Validation Failure States

```plantuml
@startuml
[*] --> RawYAML
RawYAML --> SyntaxRejected : serde_yaml::from_str fails
RawYAML --> ParsedValue : YAML parses
ParsedValue --> StructuralIssues : required mapping/type/path missing
ParsedValue --> CanonicalDocument : structure ok
CanonicalDocument --> SemanticIssues : validate_canonical_document() fails
CanonicalDocument --> ValidatedDocument : passes
SyntaxRejected --> ReportedError
StructuralIssues --> ReportedError
SemanticIssues --> ReportedError
ValidatedDocument --> [*]
ReportedError --> [*]
@enduml
```

## Related Docs

- [Validation Reporting](../validation-reporting.md)
- [Validation Examples](../validation-examples.md)
- [Runtime Resolution And Render Stage](./runtime-resolution-and-render-stage.md)
