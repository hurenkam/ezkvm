---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 7
current_phase_name: QEMU Cmdline
status: planning
last_updated: "2026-07-24T01:28:00+02:00"
last_activity: 2026-07-24
last_activity_desc: Phase 06 YAML runtime round-trip verified complete
progress:
  total_phases: 9
  completed_phases: 6
  total_plans: 4
  completed_plans: 4
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2025-07-15)

**Core value:** Import Proxmox VM configurations into a typed Runtime model, save/load as ezkvm YAML, and generate valid QEMU commandlines — with full round-trip fidelity for real-world configs.
**Current focus:** Phase 07 — QEMU Cmdline

## Current Position

Phase: 7 — QEMU Cmdline
Plan: Not started
Status: Ready to plan
Last activity: 2026-07-24 — Phase 06 verified complete

Progress: [██████░░░░] 66%

## Performance Metrics

**Velocity:**

- Total plans completed: 3
- Average duration: —
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | historical | - | - |
| 02 | historical | - | - |
| 03 | 1 | - | - |
| 04 | 1 | - | - |
| 05 | 1 | - | - |
| 06 | 1 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*

## Historical Phase Verification

- **Phase 01 — Foundation:** Complete. Typed conversion errors, `device_kind()` dispatch across
  device traits, and a mutex-free `RuntimeBuilder` are present; no `type Error = ()` or
  `downcast_ref()` calls remain under `src/`.
- **Phase 02 — Runtime Model:** Complete. `EfiDisk`, `TpmState`, `HostPci`, `Ivshmem`,
  `AudioDevice`, `SpiceDisplay`, and `RawArgs` are first-class Runtime types with builder and
  device-kind coverage in `tests/runtime_phase2.rs`.

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
- **Phase 6 YAML boundary**: The Felucia test now exercises `Runtime → ConfigSchema → YAML → ConfigSchema → Runtime`, while dedicated tests cover SPICE and empty collections.

## Next Steps

1. Plan Phase 7 QEMU commandline generation.
2. Implement the segmented commandline emitter and ordering tests.
3. Verify Phase 7 before transitioning to VM lifecycle management.

---
*State initialized: 2025-07-15*
*Last updated: 2026-07-24 — Phases 01 and 02 verified as historically complete*
