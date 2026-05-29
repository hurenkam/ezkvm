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
