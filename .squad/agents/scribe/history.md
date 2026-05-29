# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux documentation set
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Requirements baseline is maintained under doc/dev/requirements and should anchor future decision entries.
- Scribe tracks decision and session continuity as team memory source of truth.
- 2026-05-29: Logged input corpus vs canonical schema review for `input/`: 222 files total with primary artifacts `.qemu.cmd` (59), `.qemu.cmd.split` (58), `.conf` (58), plus `.yaml` (26) and smaller supporting text/config artifacts.
- 2026-05-29: Reviewers engaged: Bishop, Hicks, Hudson, Vasquez.
- 2026-05-29: Outcome indicates canonical schema likely needs updates for topology expression, machine identity, storage/network intent modeling, firmware/TPM identity, and passthrough boundary representation.
