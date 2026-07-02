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

- `src/config_importer/` owns source-specific import adapters.
- `src/runtime_config/` owns the canonical VM specification model and validation.
- `src/runtime_resolution/` owns host-bound resolution of canonical intent.
- `src/render_stage/` owns deterministic command rendering.

## Validation Layer Implementation

The `src/runtime_config/` module implements a two-layer validation pattern that separates concerns between parsing and semantic validation:

**Layer 1: Parsing** (`src/runtime_config/parsing.rs`)
- Converts raw YAML text to `RuntimeConfig` via serde deserialization
- Structural constraints (required fields and scalar/collection types) are enforced by serde during decode
- Produces `RuntimeConfig` on success or `ParseError` on failure
- Returns `ParseError::Yaml` for syntax errors and serde structural/type decode errors
- Parse-layer failures short-circuit on the first decode error reported by serde

**Layer 2: Validation** (`src/runtime_config/validation.rs`)
- Accepts a parsed `RuntimeConfig` and performs semantic validation
- Enforces business rules: uniqueness constraints, consistency policies, filename matching
- Produces `ConformanceError` that wraps validation issues or parse errors
- Uses `ValidationIssue` for semantic diagnostics and reporting
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

**Device-to-Resource Binding:**

Devices that require storage or network resources (SATA, IDE, SCSI for storage; PCIe VirtioNet for network) use a builder pattern for runtime resolution:
- Schema layer (parsing): Devices hold resource IDs as strings (e.g., `resource: "disk0"`)
- Runtime layer: Device builders (`SataDeviceBuilder`, `IdeDeviceBuilder`, `ScsiDeviceBuilder`, `UsbDeviceBuilder`) resolve IDs to concrete resource objects during `RuntimeModel::try_from()`
- Validation layer: Conformance validation ensures all device resource references match top-level resource IDs before runtime instantiation

This pattern decouples resource definition from device usage while maintaining type safety and clear error diagnostics when resources are missing or duplicated.

## Stage Module Organization

Stage modules (config_importer, render_stage, etc.) follow a consistent internal structure:

**Module declaration and export rules:**
- Use `mod xxx;` declarations in `mod.rs` files only, with crate roots `lib.rs` and `main.rs` as allowed exceptions.
- Policy exception: inline `#[cfg(test)] mod tests { ... }` modules are allowed in non-`mod.rs` implementation files.
- Keep submodule declarations private; do not use `pub mod`, `pub(crate) mod`, or `pub(super) mod`.
- Re-export public surface from `mod.rs` with explicit `pub use ...`.
- Keep `impl` blocks out of `mod.rs`.
- When practical, keep `struct` and its `impl` in the same file.

**Type definitions and traits live in `mod.rs`:**
- Request and response types (e.g., `ConfigArgs`, `RenderRequest`)
- Trait definitions that define the stage interface (e.g., `ConfigImporter`, `RenderStage`)
- Error types specific to the stage

**Concrete implementations live in separate files:**
- Each adapter or renderer gets its own module boundary (e.g., `ezkvm/mod.rs`, `proxmox/mod.rs`, `deterministic.rs`)
- Module files contain the struct definition and `impl StageTrait` blocks
- Tests for the implementation live in its own module

**Re-export pattern in `mod.rs`:**
```rust
mod ezkvm;
pub use ezkvm::EzkvmConfigImporter;
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