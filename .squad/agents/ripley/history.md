# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Core model is three-part: import-host data, fixed machine layout, runtime-host data.
- Deterministic command generation and preflight validation are baseline constraints.
- 2026-05-28: Assigned requirements readiness risk review as part of documentation audit coverage.
- 2026-05-28: Added planning traceability baseline with required requirement-ID linkage policy and a lightweight matrix template for backlog/features.
- 2026-05-28: Team converged to prioritize execution baseline plus traceability activation before additional architecture writing, with a prepared contract-first validation baseline targeted at FR-003/004/005/006/007.
- 2026-05-29: Reviewed prepared feature docs (canonical-schema-conformance-matrix, schema-host-resource-boundary-slice) and all architecture contracts. Finding: existing contracts are normatively sufficient to start implementation — full Workstream 2 Architecture & Design pass is NOT blocking. Three micro-gaps must be resolved inline at the start of the first implementation ticket: (1) Rust module layout (src/canonical/, src/validation/, src/import/, src/render/), (2) YAML library choice (serde_yaml + serde::Deserialize with typed structs), (3) error strategy (thiserror for structured errors with field-path reporting). Recommended first action: start canonical-schema-conformance-matrix implementation with these three decisions made as part of opening the ticket. schema-host-resource-boundary-slice follows after conformance matrix gates are green. Full ADR set can be written incrementally during implementation, not as a blocking pre-step.
- 2026-05-29: Completed focused module naming consistency pass after `vm_spec` rename; scaffolded sibling stage modules as `src/import_stage/`, `src/runtime_resolution/`, and `src/render_stage/`, kept behavior unchanged with module stubs only, and synced architecture docs with the explicit naming map.
- 2026-05-29: Converted `import_stage` and `render_stage` placeholders to minimal traits plus one default implementation each (`CanonicalYamlImportStage`, `DeterministicRenderStage`), keeping contracts synchronous and object-safe with associated error types to enable multiple implementations without premature framework complexity.
