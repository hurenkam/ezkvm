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

## Review Expectations

- Refactors should preserve behavior unless the change explicitly says otherwise.
- Add or update tests when a module boundary changes.
- Keep docs and helper instructions aligned with module naming or workflow changes.