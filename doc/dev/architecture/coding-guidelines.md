# ezkvm_v3 Coding Guidelines

These are repo-level boundaries for writing and reviewing Rust code.

## Principles

- Keep one responsibility per file or module.
- Prefer explicit data types and narrow interfaces.
- Keep helper modules small enough to review in one pass.
- Add tests alongside behavior changes.
- Keep naming specific to the stage or domain role.

## Size and Scope

- Prefer files that stay under roughly 350 lines.
- Split modules when a file starts mixing parsing, validation, and transformation logic.
- Avoid introducing shared abstractions until at least two implementations exist or are clearly imminent.

## Pipeline Structure

- `src/import_stage/` owns source-specific import adapters.
- `src/vm_spec/` owns the canonical VM specification model and validation.
- `src/runtime_resolution/` owns host-bound resolution of canonical intent.
- `src/render_stage/` owns deterministic command rendering.

## Validation Layer Implementation

The `src/vm_spec/` module implements a two-layer validation pattern that separates concerns between parsing and semantic validation:

**Layer 1: Parsing** (`src/vm_spec/parsing.rs`)
- Converts raw YAML text to a structured model
- Performs structural validation (required fields, type correctness, field path availability)
- Produces `CanonicalDocument` on success or `ParseError` on failure
- Returns `ParseError::Yaml` immediately for syntax-invalid YAML before structural validation runs
- Collects all parsing issues in one pass (does not short-circuit on first error)
- Uses `ValidationIssue` type with precise field paths (e.g., `virtual_machine.storage[1].id`)

**Layer 2: Validation** (`src/vm_spec/validation.rs`)
- Accepts a parsed `CanonicalDocument` and performs semantic validation
- Enforces business rules: uniqueness constraints, consistency policies, filename matching
- Produces `ConformanceError` that wraps validation issues or parse errors
- Uses the same `ValidationIssue` type for consistency with parsing layer
- Reports field paths including array indices for duplicate detection
- Owns report formatting via `ReportFormatter` and `DefaultReportFormatter`

**Key Types:**

- `ValidationIssue`: Contains `path`, `reason`, `severity`, and optional `line_number`, `source_snippet`, and `remediation`
- `ParseError`: Wraps either a raw YAML syntax error or a counted list of `ValidationIssue`
- `ConformanceError`: Wraps either `ParseError` or a count + list of `ValidationIssue`

**Testing Pattern:**

Tests live in the implementation module (`validation.rs`) and cover:
- Valid documents (happy path)
- YAML syntax rejection at the parse boundary
- Missing required fields
- Type mismatches
- ID uniqueness violations within each scope
- Filename/vm_name matching
- Chipset consistency with machine family
- Human-readable and JSON report formatter output

See [validation rules reference](../requirements/validation-rules.md) for detailed rule documentation and [validation examples](./validation-examples.md) for common failure scenarios.

## Stage Module Organization

Stage modules (import_stage, render_stage, etc.) follow a consistent internal structure:

**Type definitions and traits live in `mod.rs`:**
- Request and response types (e.g., `ImportRequest`, `RenderRequest`)
- Trait definitions that define the stage interface (e.g., `ImportStage`, `RenderStage`)
- Error types specific to the stage

**Concrete implementations live in separate files:**
- Each adapter or renderer gets its own module file (e.g., `canonical_yaml.rs`, `proxmox_conf.rs`, `deterministic.rs`)
- Module files contain the struct definition and `impl StageTrait` blocks
- Tests for the implementation live in its own module

**Re-export pattern in `mod.rs`:**
```rust
pub mod canonical_yaml;
pub use canonical_yaml::CanonicalYamlImportStage;
```

**Rationale:**
- Types and traits change less frequently; implementations can be added/replaced without touching the interface
- Clear separation makes stage capabilities and boundaries explicit
- Easier to add new adapters without modifying the stage definition
- Tests stay colocated with implementations

## Review Expectations

- Refactors should preserve behavior unless the change explicitly says otherwise.
- Add or update tests when a module boundary changes.
- Keep docs and helper instructions aligned with module naming or workflow changes.