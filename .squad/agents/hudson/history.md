# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM machine modeling on Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Canonical machine layout is the fixed model between import and runtime stages.
- Q35 and i440fx domain constraints are key to compatibility and deterministic behavior.
- 2026-05-28: Assigned chipset documentation coverage audit focused on q35/i440fx/qemu material.
- 2026-05-28: Added ezkvm Q35/i440fx machine-model policy that fixes canonical-vs-runtime boundaries, deterministic placement rules, and chipset-sensitive validation minimums aligned with FR-004/FR-005.
- 2026-05-29: Participated in corpus-vs-schema review with Bishop, Hicks, and Vasquez; input corpus sizing/artifact scan (`input/` = 222 files) reinforced likely schema adjustments around topology modeling, machine identity, storage/network intent, firmware/TPM identity, and passthrough boundaries.
- 2026-05-29: Reviewed expanded input/ Proxmox and QEMU corpus against canonical schema contract; aligned on schema expansion for topology/identity/intent/passthrough and runtime-resolution of host-specific paths.
