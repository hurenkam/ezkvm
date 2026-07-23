---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 6
current_phase_name: YAML↔Runtime
status: planning
last_updated: "2026-07-23T21:10:23.640Z"
last_activity: 2026-07-23
last_activity_desc: Phase 05 complete, transitioned to Phase 6
progress:
  total_phases: 5
  completed_phases: 2
  total_plans: 5
  completed_plans: 2
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2025-07-15)

**Core value:** Import Proxmox VM configurations into a typed Runtime model, save/load as ezkvm YAML, and generate valid QEMU commandlines — with full round-trip fidelity for real-world configs.
**Current focus:** Phase 03 — proxmox-parser

## Current Position

Phase: 6 — YAML↔Runtime
Plan: Not started
Status: Ready to plan
Last activity: 2026-07-23 — Phase 05 complete, transitioned to Phase 6

Progress: [██░░░░░░░░] 22%

## Performance Metrics

**Velocity:**

- Total plans completed: 3
- Average duration: —
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 03 | 1 | - | - |
| 04 | 1 | - | - |
| 05 | 1 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*

## Accumulated Context

### Decisions

- **saphyr over serde_yaml**: serde_yaml is officially deprecated; saphyr is the maintained successor and is already used in the codebase. Do not switch.
- **args field is opaque**: Proxmox `args:` sub-key must be stored and emitted verbatim as `RawArgs(String)`. Never attempt to parse its contents.
- **Snapshot sections must be split first**: Proxmox `.conf` parser MUST detect and discard `[snapshot_name]` sections before processing any key-value pairs. Bleeding snapshot fields into the active config produces silently wrong VMs.
- **ProxmoxImporter as named struct**: Use `ProxmoxImporter` struct (not bare `TryFrom`) so context (storage pool map, host name) can be passed through conversion without tuple hackery.
- **thiserror from day one**: Both `proxmox.rs` and `qemu.rs` currently use `type Error = ()`. Fix in Phase 1 before any new conversion code is written.
- **device_kind() replaces downcast_ref()**: 54 existing `downcast_ref()` sites exist. Add `device_kind()` to all device traits in Phase 1; no new `downcast_ref()` after that.
- **drive-before-device ordering**: QEMU requires `-drive id=X` before `-device ...,drive=X`. Use a segment-based `QemuCommandLine` builder struct (Phase 7) not a flat `Vec<String>`.
- **storage.cfg is first-class input**: Pool references in `.conf` values (e.g., `vm1-pool:vm-108-boot`) cannot be resolved without `storage.cfg`. It is a required input to the importer, not optional.

### Patterns

- Conversion via `TryFrom` impls between layer types (Runtime ↔ Schema ↔ Proxmox ↔ QEMU)
- `Arc<dyn Trait>` for device storage in Runtime (existing pattern — keep)
- Indexed keys (`scsi0`, `hostpci0`, …) parsed by stripping trailing digits then dispatching on base name
- Sub-option values: split on first comma for positional, then `k=v` pairs; handle colons inside values by per-field tokenization (not generic split)

### Watch Out For

- **Colon in sub-option values**: MAC addresses (`BC:24:11`), PCI BDFs (`0000:03:00`), storage volumes (`pool:vm-108-boot`) all contain colons — generic `split(':')` will corrupt them. Use per-field tokenizers.
- **URL-encoded comments**: `##args%3A` in some `.conf` files must be detected and discarded as comments, not decoded as live config.
- **Multi-function GPU**: `hostpci0: 0000:03:00,pcie=1,x-vga=1` maps to two `vfio-pci` devices (`.0` audio + `.1` GPU). Model as `functions: Vec<u8>` in `HostPci`.
- **EfiDisk dual sizes**: `efidisk0` has a logical size field AND a block device size in bytes — both must be preserved.
- **RuntimeBuilder Mutex**: Current builder wraps device Vec in Mutex; remove it (builders are single-threaded, Mutex can poison on panic).

## Next Steps

1. Run `/gsd-plan-phase 3` to plan Phase 3 (Proxmox importer)
2. Execute Phase 3 plan
3. Continue phases 4–8 in order

---
*State initialized: 2025-07-15*
*Last updated: 2025-07-15 — Project onboarding complete*
