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
- 2026-05-29: Participated in corpus-vs-schema review with Bishop, Hicks, and Hudson; `input/` corpus inventory (222 files, dominated by `.conf`/`.qemu.cmd`/`.qemu.cmd.split`) supports likely canonical schema updates for topology, machine identity, storage/network intent, firmware/TPM identity, and passthrough boundary handling.
- 2026-05-29: Reviewed expanded input/ Proxmox and QEMU corpus against canonical schema contract; aligned on schema expansion for topology/identity/intent/passthrough and runtime-resolution of host-specific paths.
- 2026-05-29: QA conformance slice review added regression tests for precise validation field paths on empty IDs and uniqueness behavior (non-consecutive duplicate index targeting and cross-scope ID allowance); full Rust gates (`fmt`, `clippy -D warnings`, `test`) passed.
- 2026-05-29: Item 2 edge-case review showed optional-section and scalar type checks already existed; the real coverage gap was explicit malformed-YAML proof and container-shape mismatch evidence with precise field paths.
