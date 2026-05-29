# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux documentation set
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Requirement IDs in product requirements should anchor planning and implementation docs.
- Docs must stay synchronized when behavior, defaults, or runtime workflow changes.
- 2026-05-28: Assigned documentation readiness audit for doc structure and coverage checks.
- 2026-05-28: Core dev contracts are clearer when split into adapter boundary, pipeline boundary, schema shape, and determinism guarantees with FR/NFR traceability.
- 2026-05-28: Team converged to prioritize execution baseline and traceability activation before additional architecture writing, with a proposed prepared contract-first validation baseline for FR-003/004/005/006/007.
- 2026-05-29: Reusable helper skills live under `.copilot/skills/` in this repo; prompt helpers live under `.github/prompts/`.
- 2026-05-29: Module/stage design guidance should center on trait-based stage boundaries, source-specific adapter scaffolds, deterministic rendering, and validation before execution.
