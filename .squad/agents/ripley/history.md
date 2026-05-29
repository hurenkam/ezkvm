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
