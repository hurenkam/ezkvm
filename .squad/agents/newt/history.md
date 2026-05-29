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
