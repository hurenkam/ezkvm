# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Validation before execution and dry-run preview behavior are explicit requirements.
- Determinism and actionable error reporting are quality gates.
- 2026-05-28: Team converged to prioritize execution baseline and traceability activation before further architecture writing, with next prepared work focused on contract-first validation for FR-003/004/005/006/007.
