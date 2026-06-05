---
name: "rustdoc-writing"
description: "Use when documenting Rust code with rustdoc comments, module headers, struct and enum docs, and function or method documentation"
domain: "documentation"
confidence: "high"
source: "manual"
---

# Rustdoc Writing

## Context

Use this skill when adding or revising Rust documentation comments in `src/**/*.rs` or `tests/**/*.rs`.

The goal is to keep Rust code self-describing with rustdoc that is brief where it should be brief and explicit where public APIs need more context.

## Patterns

### File Header

Start each Rust file with a module-level `//!` header that explains:

- what the file is responsible for
- why it exists in the module layout
- which related design or requirement documents are relevant

Keep the header short for implementation files and a bit fuller for public-facing modules.

Example:

```rust
//! Runtime configuration helpers for validating and normalizing VM settings.
//!
//! Related documentation:
//! - doc/dev/architecture/model-separation-pipeline.md
//! - src/runtime_config/README.md
```

### Struct and Enum Docs

Document every struct and enum with `///` comments that explain its purpose.

- Public types should describe the concept they model and the constraints they enforce.
- Internal types should still explain the local role they play in the implementation.
- Prefer one short paragraph over a verbose list unless the type has important invariants.

Example:

```rust
/// Captures the validated runtime settings that downstream stages consume.
///
/// The type only contains values that have already passed importer and
/// validation checks.
pub struct RuntimeConfig {
```

### Function and Method Docs

Document each function and impl method with `///` comments that cover:

- what it does
- why it exists
- what arguments it takes
- what it returns

For local helpers and private methods, keep the comment concise and focus on the single behavior point.

For public methods, trait methods, and boundary-facing APIs, be more explicit about preconditions, side effects, and error behavior.

Example:

```rust
/// Converts a parsed source configuration into the canonical runtime model.
///
/// This keeps source-specific defaults isolated from the rest of the pipeline.
///
/// # Arguments
///
/// * `input` - Parsed source configuration to normalize.
///
/// # Returns
///
/// The canonical runtime representation, or an error if validation fails.
pub fn normalize(input: SourceConfig) -> Result<RuntimeConfig, NormalizeError> {
```

### Trait Docs

Document traits like public APIs:

- explain the contract the trait defines
- describe what implementors must guarantee
- mention any lifecycle or ownership expectations

If the trait has required methods, document those methods as part of the trait boundary.

### Consistency Rules

- Use `//!` for file/module headers and `///` for items.
- Keep wording direct and factual.
- Prefer present tense.
- Avoid repeating the code name in every sentence when the item name already makes it clear.
- Mention related docs by path when they help orient the reader.

## Anti-Patterns

- Leaving a Rust file without a module header
- Writing long prose for private helpers that only need one sentence
- Documenting public items with vague phrases like "does stuff" or "helper function"
- Omitting arguments or return behavior from public API docs
- Mixing implementation commentary into the rustdoc summary line