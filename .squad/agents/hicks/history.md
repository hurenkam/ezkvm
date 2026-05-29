# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Runtime stage uses fixed machine layout plus runtime-host data to generate QEMU arguments.
- Deterministic command generation and preflight validation are baseline constraints.
- 2026-05-29: Participated in corpus-vs-schema review with Bishop, Hudson, and Vasquez; confirmed `input/` artifact mix (222 files, mostly `.conf`/`.qemu.cmd`/`.qemu.cmd.split`) and captured likely canonical schema updates for topology, machine identity, storage/network intent, firmware/TPM identity, and passthrough limits.
- 2026-05-29: Reviewed expanded input/ Proxmox and QEMU corpus against canonical schema contract; aligned on schema expansion for topology/identity/intent/passthrough and runtime-resolution of host-specific paths.
- 2026-05-29: Implemented first Rust conformance slice with typed canonical YAML parsing and deterministic validation for core required fields, vm_name filename matching, pc chipset consistency, and scoped ID uniqueness; field-path diagnostics are emitted through structured validation issues.
- 2026-05-29: Completed validation report enhancement by enriching parse/conformance issues with best-effort YAML line/snippet context, adding stable `ValidationReport`/`ValidationSummary` helpers, and keeping JSON plus human formatter output aligned under test.
