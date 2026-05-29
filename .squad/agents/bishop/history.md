# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM, Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Import source families in scope include Proxmox config, raw QEMU command lines, and Libvirt definitions.
- Import adapters must normalize into a canonical YAML model without redesigning core runtime.
- 2026-05-29: Participated in corpus-vs-schema review with Hicks, Hudson, and Vasquez; reviewed `input/` inventory (222 files: `.conf`/`.qemu.cmd`/`.qemu.cmd.split` dominant) and aligned on likely schema gaps in topology, machine identity, intent modeling, firmware/TPM identity, and passthrough boundaries.
- 2026-05-29: Reviewed expanded input/ Proxmox and QEMU corpus against canonical schema contract; aligned on schema expansion for topology/identity/intent/passthrough and runtime-resolution of host-specific paths.
- 2026-05-29: Renamed Rust module directory from `src/canonical` to `src/vm_spec` to reflect typed VM intent schema plus validation responsibilities; kept type/function names stable and validated with fmt, clippy, and tests.
- 2026-05-29: Added a Proxmox `.conf` import-stage skeleton (`ProxmoxConfImportStage`) beside canonical YAML import to prove the `ImportStage` trait boundary can host multiple adapters.
