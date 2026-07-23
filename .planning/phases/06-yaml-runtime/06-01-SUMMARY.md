---
phase: 06-yaml-runtime
plan: 01
subsystem: config
tags: [yaml, serde, thiserror, runtime, conversion, round-trip]

# Dependency graph
requires:
  - phase: 01-05
    provides: Runtime domain types (EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs, HostPci, Ivshmem, storage devices) and the ezkvm YAML schema types (ConfigSchema, DeviceSchema, ResourceSchema, etc.)
provides:
  - "YamlRuntimeError typed error enum (11 variants) as the single Error type for both TryFrom<Runtime> for ConfigSchema and TryFrom<ConfigSchema> for Runtime"
  - "Lossless round-trip for all v1 root devices: Memory, Chipset, EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs"
  - "Lossless round-trip for all v1 PCIe devices: HostPci (multi-function), Ivshmem"
  - "Storage resource path preservation (SCSI/SATA/IDE) — the synthetic-empty-string bug is fixed for Ssd, Hdd, and Cdrom"
  - "Deterministic ResourceNotFound typed error when a device references a missing resource ID"
affects: [07-yaml-runtime-cli, yaml-serialization-consumers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Typed conversion-boundary errors via thiserror::Error (YamlRuntimeError) replacing bare String errors"
    - "Resource-ID indirection: parser.rs collects ResourceSchema entries (Storage/PcieDevice/Memory) into a Vec before constructing HostSchema; builder.rs resolves every resource ID through host.resources() before constructing storage/HostPci/Ivshmem devices"
    - "Deterministic synthetic resource IDs for HostPci (hostpci0, hostpci1, ... via an address-sorted counter); Ivshmem reuses its own stable id() as the resource ID"

key-files:
  created:
    - src/config/ezkvm/runtime/error.rs
    - tests/yaml_round_trip.rs
  modified:
    - src/config/ezkvm/runtime/parser.rs
    - src/config/ezkvm/runtime/builder.rs
    - src/config/ezkvm/runtime.rs
    - src/runtime/ide.rs
    - src/runtime/storage.rs

key-decisions:
  - "IdeDevice trait gained an as_any() method (mirroring ScsiDevice/SataDevice) so IDE storage devices (Cdrom) can be downcast for resource-path extraction during parsing — required for storage resource fidelity, not explicitly listed in the plan's files_modified but necessary to satisfy the plan's own storage-resource-path truths"
  - "A handful of pre-existing, defensive bare-String error sites that don't map 1:1 onto any of the plan's 11 named YamlRuntimeError variants (legacy PCI bus != 0, non-generic USB device kind — currently unreachable via the public API, VirtioNet on a non-zero PCIe bus) were mapped onto YamlRuntimeError::UnsupportedPcieDevice { device_type: <descriptive message> } rather than inventing new unplanned variants"
  - "missing_resource_id_returns_typed_error was implemented as an internal unit test in builder.rs (not in tests/yaml_round_trip.rs) because constructing a minimal ConfigSchema requires the private crate::config::ezkvm::schema module, which is not part of the crate's public API surface (config.rs only re-exports ConfigSchema itself as EzkvmConfigSchema, not its schema submodule) — extending public visibility across the schema tree was judged out of scope for this plan"

requirements-completed: [YAML-01, YAML-02]

coverage:
  - id: D1
    description: "TryFrom<Runtime> for ConfigSchema and TryFrom<ConfigSchema> for Runtime both use YamlRuntimeError as their associated Error type; zero bare String errors remain in src/config/ezkvm/runtime/"
    requirement: "YAML-01"
    verification:
      - kind: unit
        ref: "grep -r 'type Error = String' src/config/ezkvm/runtime/ | wc -l -> 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "EfiDisk, TpmState, AudioDevice, RawArgs, SpiceDisplay root devices round-trip through the YAML schema with all fields byte-identical"
    requirement: "YAML-02"
    verification:
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_efidisk_fields_preserved"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_tpmstate_version_and_volume"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_audio_device_fields"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_rawargs_verbatim"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_spice_display"
        status: pass
    human_judgment: false
  - id: D3
    description: "HostPci (multi-function) and Ivshmem PCIe devices round-trip with base_bdf, functions, x_vga, rombar, romfile, mem_path, and size preserved"
    requirement: "YAML-02"
    verification:
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_hostpci_base_bdf_and_functions"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_ivshmem_id_path_size"
        status: pass
    human_judgment: false
  - id: D4
    description: "Ssd, Hdd, and Cdrom resource paths (SCSI, SATA, IDE respectively) are non-empty and preserved after a schema round trip — the prior synthetic-empty-string bug is fixed"
    requirement: "YAML-02"
    verification:
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_scsi_storage_resource_path"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_sata_hdd_storage_resource_path"
        status: pass
      - kind: unit
        ref: "src/config/ezkvm/runtime/parser.rs#round_trip_ide_cdrom_storage_resource_path"
        status: pass
      - kind: integration
        ref: "tests/yaml_round_trip.rs#felucia_108_runtime_round_trips_yaml"
        status: pass
    human_judgment: false
  - id: D5
    description: "A schema referencing a resource ID absent from host.resources() returns Err(YamlRuntimeError::ResourceNotFound { id }) deterministically instead of panicking or silently substituting an empty path"
    requirement: "YAML-01"
    verification:
      - kind: unit
        ref: "src/config/ezkvm/runtime/builder.rs#missing_resource_id_returns_typed_error"
        status: pass
    human_judgment: false
  - id: D6
    description: "End-to-end Felucia-108 fixture round-trips through the real Proxmox importer -> Runtime -> ezkvm YAML -> Runtime pipeline with root device count, EfiDisk, TpmState, AudioDevice, RawArgs, HostPci, and Ssd resource all verified"
    requirement: "YAML-02"
    verification:
      - kind: integration
        ref: "tests/yaml_round_trip.rs#felucia_108_runtime_round_trips_yaml"
        status: pass
    human_judgment: false
  - id: D7
    description: "Full cargo test -p ezkvm suite passes with zero regressions across lib, bin, and all three integration test files"
    verification:
      - kind: unit
        ref: "cargo test -p ezkvm (all targets)"
        status: pass
    human_judgment: false

# Metrics
duration: not precisely tracked (multi-session execution across a context compaction; resumed-portion work, from integration-test creation through final verification, took roughly 20-25 min)
completed: 2025-06-XX
status: complete
---

# Phase 6 Plan 1: YAML Runtime Conversion Boundary Summary

**Typed `YamlRuntimeError` conversion boundary (thiserror, 11 variants) plus full v1 field-fidelity round-trips for EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs, HostPci, Ivshmem, and SCSI/SATA/IDE storage resource paths between Runtime and ezkvm YAML.**

**⚠️ Per explicit user instruction, no `git commit` was run at any point in this execution. All code, test, and documentation changes listed below remain uncommitted in the working tree for manual review.**

## Performance

- **Started:** not recorded (session began before this compaction boundary)
- **Completed:** see file mtime / `git status` timestamp at hand-off
- **Tasks:** 3/3 (tracer + 2 expansion tasks, per plan frontmatter)
- **Files modified:** 5 modified, 2 created (see below)

## Accomplishments
- Replaced every bare-`String` error at the Runtime ↔ ezkvm YAML conversion boundary with a single typed `YamlRuntimeError` enum (11 variants, `thiserror`-derived), used as the associated `Error` type for both `TryFrom<Runtime> for ConfigSchema` and `TryFrom<ConfigSchema> for Runtime`.
- Extended `parser.rs`/`builder.rs` to round-trip all seven v1 root device types (Memory, Chipset, EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs) with byte-identical fields.
- Extended PCIe device round-tripping to cover `HostPci` (multi-function, with deterministic `hostpciN` resource-ID indirection) and `Ivshmem` (`id`/`mem_path`/`size`).
- Fixed the synthetic-empty-string storage resource bug: SCSI (`Ssd`), SATA (`Hdd`), and IDE (`Cdrom`) resource paths are now resolved through real `ResourceSchema` entries instead of being silently discarded.
- Added an end-to-end Felucia-108 integration test (`tests/yaml_round_trip.rs`) exercising the real Proxmox-import → Runtime → YAML → Runtime pipeline.
- Added a deterministic `ResourceNotFound` typed-error test for a schema referencing a missing resource ID.

## Task Commits

**No commits were made — critical user instruction: do not run `git commit` under any circumstances.** All work below is present only in the uncommitted working tree. A human reviewer is expected to inspect the diff and commit (or request changes) manually.

| Task | Name | Status | Key files |
|------|------|--------|-----------|
| 1 (tracer) | `YamlRuntimeError` enum + EfiDisk round-trip | Done, uncommitted | `error.rs` (new), `parser.rs`, `builder.rs`, `runtime.rs` |
| 2 | TpmState/AudioDevice/RawArgs/SpiceDisplay round-trips | Done, uncommitted | `parser.rs`, `builder.rs` |
| 3 | HostPci/Ivshmem + storage resource path preservation + integration test | Done, uncommitted | `parser.rs`, `builder.rs`, `ide.rs`, `storage.rs`, `tests/yaml_round_trip.rs` (new) |

## Files Created/Modified
- `src/config/ezkvm/runtime/error.rs` (new) - `YamlRuntimeError` thiserror enum, 11 variants
- `tests/yaml_round_trip.rs` (new) - Felucia-108 end-to-end YAML round-trip integration test
- `src/config/ezkvm/runtime/parser.rs` - `TryFrom<Runtime> for ConfigSchema`; typed errors; EfiDisk/TpmState/AudioDevice/RawArgs/SpiceDisplay/HostPci/Ivshmem handling; storage resource path extraction; 11 new unit tests
- `src/config/ezkvm/runtime/builder.rs` - `TryFrom<ConfigSchema> for Runtime`; typed errors; device reconstruction incl. resource-ID resolution helpers (`find_storage_resource`, `find_pcie_resource`, `find_memory_resource`); 1 new unit test (`missing_resource_id_returns_typed_error`)
- `src/config/ezkvm/runtime.rs` - declares `pub mod error;` and re-exports `YamlRuntimeError`
- `src/runtime/ide.rs` - added `as_any()` to the `IdeDevice` trait (deviation, see below)
- `src/runtime/storage.rs` - implemented `as_any()` for `IdeDevice for Ssd/Hdd/Cdrom`

## Decisions Made
- Reused `YamlRuntimeError::UnsupportedPcieDevice { device_type }` for a few defensive/edge-case bare-String error sites not explicitly covered by the plan's 11 named variants (legacy PCI bus != 0, non-generic USB device kind [currently unreachable via the public API], VirtioNet on a non-zero PCIe bus) rather than inventing unplanned variants outside the plan's spec.
- `missing_resource_id_returns_typed_error` was placed as an internal `#[cfg(test)]` unit test in `builder.rs` rather than in `tests/yaml_round_trip.rs`, because constructing a minimal `ConfigSchema` by hand requires the private `crate::config::ezkvm::schema` module, which is not reachable from an external integration-test crate (only the `ConfigSchema` struct itself is re-exported as `EzkvmConfigSchema`). Expanding the crate's public API surface to accommodate one test's file placement was judged out of scope for this plan.
- `RuntimeBuilder::build()` (which can never actually fail) uses `unwrap_or_else(|_| unreachable!(...))` per the plan's explicit instruction, replacing the prior `.map_err(|_| "failed...".to_string())`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2/3 - Missing Critical / Blocking] Added `as_any()` to the `IdeDevice` trait**
- **Found during:** Task 3 (storage resource path preservation)
- **Issue:** `IdeDevice` (used by `Cdrom`) had no `as_any()` method, unlike `ScsiDevice`/`SataDevice`, blocking downcasting needed to extract IDE storage device resource paths during parsing — required to satisfy the plan's own truth "Hdd, Ssd, and Cdrom resource paths are non-empty strings after round-trip".
- **Fix:** Added `fn as_any(&self) -> &dyn std::any::Any;` to the `IdeDevice` trait in `src/runtime/ide.rs`, implemented for `Ssd`, `Hdd`, `Cdrom` in `src/runtime/storage.rs` (mirroring the existing `ScsiDevice`/`SataDevice` pattern).
- **Files modified:** `src/runtime/ide.rs`, `src/runtime/storage.rs` (not in the plan's `files_modified` list, but a necessary companion change).
- **Verification:** `round_trip_ide_cdrom_storage_resource_path` unit test passes; full test suite green.
- **Committed in:** not committed (uncommitted per user instruction).

**2. [Rule 2 - Missing Critical] Added two extra unit tests beyond the plan's explicit list**
- **Found during:** Task 3 (final self-check against plan success criterion 5)
- **Issue:** The plan's success criterion 5 requires Ssd, Hdd, *and* Cdrom resource paths to be verified non-empty after round-trip, but the originally-planned test coverage (`round_trip_scsi_storage_resource_path` + the Felucia integration test) only exercised the SCSI/Ssd path.
- **Fix:** Added `round_trip_sata_hdd_storage_resource_path` and `round_trip_ide_cdrom_storage_resource_path` unit tests in `parser.rs` to close the SATA/Hdd and IDE/Cdrom coverage gap.
- **Files modified:** `src/config/ezkvm/runtime/parser.rs`.
- **Verification:** Both new tests pass (`cargo test -p ezkvm --lib -- round_trip_sata_hdd_storage_resource_path round_trip_ide_cdrom_storage_resource_path`).
- **Committed in:** not committed (uncommitted per user instruction).

---

**Total deviations:** 2 auto-fixed (1 missing-critical/blocking trait addition, 1 missing-critical test-coverage gap fill)
**Impact on plan:** Both auto-fixes were necessary for correctness/completeness against the plan's own stated truths and success criteria. No scope creep beyond what the plan itself required.

## Issues Encountered
- Two pre-existing `builder.rs` unit tests (`sata_hdd_is_supported`, `q35_extended_device_types_are_supported`) referenced resource IDs (`storage0`, `iso0`) that were never previously validated (old code ignored the ID and used `String::new()` regardless). Once `find_storage_resource` began enforcing existence, these tests needed matching `ResourceSchema::Storage` entries added to their `HostSchema::new(...)` calls — fixed inline, no behavior change to the tests' intent.
- A minor typo (`Spice: :SpiceDisplay` → `SpiceDisplay`) and a `&Option<u16>` iterator-copy compile error (`spice.port().copied()` → `(*spice.port()).unwrap_or(0)`) were introduced and fixed during the edit cycle; no lasting impact.

## User Setup Required
None - no external service configuration required.

## Self-Check

**Files exist:**
- FOUND: `src/config/ezkvm/runtime/error.rs`
- FOUND: `tests/yaml_round_trip.rs`
- FOUND: `src/config/ezkvm/runtime/parser.rs` (modified)
- FOUND: `src/config/ezkvm/runtime/builder.rs` (modified)
- FOUND: `src/config/ezkvm/runtime.rs` (modified)
- FOUND: `src/runtime/ide.rs` (modified)
- FOUND: `src/runtime/storage.rs` (modified)

**Commits:** N/A — per explicit user instruction, no commits were made. All changes above are present only in the uncommitted working tree (`git status --short` at hand-off shows all 5 modified + 2 new files as unstaged/untracked, and `git log` shows no new commits beyond the pre-existing `76de120 prepare phase 6`).

**Tests:**
- `cargo test -p ezkvm --lib` → 49 passed, 0 failed (lib target) / 49 passed, 0 failed (bin target)
- `cargo test -p ezkvm --test proxmox_import` → 7 passed, 0 failed
- `cargo test -p ezkvm --test runtime_phase2` → 5 passed, 0 failed
- `cargo test -p ezkvm --test yaml_round_trip` → 1 passed, 0 failed
- Full `cargo test -p ezkvm` → all targets green, zero regressions

**Verification commands from plan (`<verification>` section):**
- `grep -r "type Error = String" src/config/ezkvm/runtime/ | wc -l` → `0` ✅
- `cargo test -p ezkvm -- round_trip_scsi_storage_resource_path --nocapture` → no assertion failures ✅
- `cargo test -p ezkvm 2>&1 | tail -20` → all pass, zero regressions ✅

## Self-Check: PASSED

## Known Stubs
None found — no hardcoded empty values, placeholder text, or unwired data sources introduced in this plan's files.

## Next Phase Readiness
- The Runtime ↔ ezkvm YAML conversion boundary is now fully typed and field-complete for all v1 root/PCIe devices and storage resources — ready for any subsequent phase (e.g., a YAML CLI/serialization-consumer phase) that depends on lossless round-tripping.
- **Blocker for the standard GSD workflow:** per the user's critical instruction, this plan's changes were **not committed**, and the standard `<state_updates>`/`<final_commit>` steps (STATE.md advance-plan, ROADMAP.md progress update, REQUIREMENTS.md mark-complete, final metadata commit) were **deliberately skipped** to avoid triggering any git write. `.planning/STATE.md`, `.planning/ROADMAP.md`, and `.planning/REQUIREMENTS.md` remain unmodified. A human (or a follow-up run of the standard executor flow, once changes are reviewed and committed) should perform those updates once this plan's diff has been accepted.
- `.vscode/`, `diffs/`, and `target/` remain untouched and untracked, as required.

---
*Phase: 06-yaml-runtime*
*Completed: uncommitted — pending manual review*
