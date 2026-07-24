---
phase: 07-qemu-cmdline
verified: 2026-07-24T17:49:20Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps: []
closed_gaps:
  - truth: "QemuCommandLine::try_from((runtime, context)) produces a commandline containing all seven v1 device types"
    closed_at: 2026-07-24T17:58:00Z
    resolution: "Added test_07_verify_spice_display_emits_spice_flag to src/config/qemu.rs, constructing a SpiceDisplay via RuntimeBuilder::with_spice_display and asserting the emitted -spice token's port=/addr=/disable-ticketing=on/gl=on/rendernode= sub-options. All seven v1 device types are now proven via a passing test."
  - truth: "CpuTopology (-smp/-cpu) and VgaConfig (-vga none/-nographic) emission, added specifically to satisfy D-02, is exercised by at least one test"
    closed_at: 2026-07-24T17:58:00Z
    resolution: "Added test_07_verify_cpu_topology_emits_smp_and_cpu, test_07_verify_vga_config_none_emits_nographic, and test_07_verify_vga_config_other_mode_emits_nothing to src/config/qemu.rs, asserting -smp/-cpu token shape and the -vga none/-nographic scope boundary. Full workspace suite now 108/108 passing (was 104/104), 0 regressions."
deferred: []
---

# Phase 7: QEMU Cmdline Verification Report

**Phase Goal:** The QEMU commandline emitter produces a correctly ordered, valid argument list for all v1 Runtime device types, with raw args appended verbatim and all drive/netdev references preceding their dependent `-device` arguments.
**Verified:** 2026-07-24T17:49:20Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `QemuCommandLine::try_from((runtime, context))` produces a commandline containing all seven v1 device types | ⚠️ PARTIAL | 6/7 v1 types (EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, RawArgs) proven via passing tests that invoke `try_from` or the dispatched handler directly (`src/config/qemu.rs` tests, `handlers/pcie.rs` tests). `SpiceDisplay`'s `emit_root_device` arm exists and is D-08-compliant but is never invoked by any test — see Gap 1. |
| 2 | Every `-drive if=none,id=<X>,...` argument appears before its corresponding `-device ...,drive=<X>,...` argument | ✓ VERIFIED | Segment-based builder (`builder.rs`) structurally guarantees `drives` segment renders before `devices` segment regardless of call order (`QemuCommandLine::fmt`, qemu.rs:44-60). Proven via `tests/qemu_cmdline.rs::test_qemu_cmdline_felucia_108_drive_before_device_ordering` (real felucia fixture, scsi0 + ide2) using string-position `assert_precedes`, not substring presence. |
| 3 | Every `-netdev ...,id=<N>,...` argument appears before its corresponding `-device virtio-net-pci,...,netdev=<N>,...` argument | ✓ VERIFIED | Same segment-order guarantee (`netdevs` before `devices`). Proven via `tests/qemu_cmdline.rs::test_qemu_cmdline_synthetic_netdev_before_device_ordering` — a synthetic `VirtioNetPcie` test, explicitly added because the felucia fixture's single NIC alone is insufficient (RESEARCH.md Pitfall 7, confirmed absent from the fixture-only test). |
| 4 | The `RawArgs` blob is appended verbatim after all structured arguments, with internal token order preserved | ✓ VERIFIED | `RawArgs.0` pushed unmodified once into `misc` segment (`root.rs:175-177`), `misc` is the last-rendered segment. Proven via `src/config/qemu.rs::test_07_01_rawargs_verbatim_passthrough` (isolated) and `tests/qemu_cmdline.rs::test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end` (end-to-end, re-verified after Plans 07-02/07-03/07-04 added devices/netdevs/objects/tpm segments — correctly resolves the "last real device" via `cmdline.devices().last()` rather than a naive `rfind("-device")`, avoiding a false-positive match inside the felucia blob's own embedded `-device` substrings). |

**Score:** 4/5 must-haves verified (see also PLAN-frontmatter-level truths below); 2 partial-coverage gaps noted under Truth #1's scope (SpiceDisplay + CpuTopology/VgaConfig)

### PLAN Frontmatter Must-Haves (Selected Cross-Check)

| Plan | Truth (abbreviated) | Status | Evidence |
|------|---------------------|--------|----------|
| 07-01 | `Runtime` exposes `boot_order()`/`CpuTopology`/`VgaConfig` populated by `ProxmoxImporter` | ✓ VERIFIED | `src/runtime/cpu.rs`, `src/runtime/vga.rs` exist; `importer.rs` parses `boot`/`cores`/`sockets`/`cpu`/`vga`; `tests/proxmox_import.rs` asserts 8 root devices for felucia-108 (was 6) |
| 07-01 | `QemuContext` missing-field access uses `.expect()`, never `QemuConversionError` (D-06) | ✓ VERIFIED | `qemu.rs`'s `QemuConversionError` enum only has `EmptyHostPciFunctions`; `ctx.tpm_socket_path()`/`ovmf_code_path()` accessed via direct field reads / `.expect()` in root.rs |
| 07-02 | pvscsi + scsi drive/device ordering, HostPci multifunction only on `.0`, Ivshmem object-before-device, VirtioNet netdev-before-device | ✓ VERIFIED | `handlers/pcie.rs` tests: `test_07_02_pvscsi_controller_and_single_ssd_end_to_end`, `test_07_02_hostpci_multifunction_flag_only_on_function_zero`, `test_07_02_ivshmem_object_before_device_in_display_order`, `test_07_02_virtio_net_netdev_before_device_in_display_order` — all pass |
| 07-03 | pci_bus PvScsi reuses `emit_pvscsi` unmodified; sata/ide storage sorted by address; USB host-passthrough parses `hostbus`/`hostport`; importer populates net/usb | ✓ VERIFIED | `pci.rs::test_07_03_pvscsi_on_pci_bus_reuses_emit_pvscsi` (calls shared fn, `grep -c "scsi_bus().iter()" pci.rs` = 0 per SUMMARY); `storage.rs::test_07_03_ide_entries_sorted_by_channel_device_not_insertion_order`; `usb.rs::test_07_03_usb_host_passthrough_parses_hostbus_hostport`; `importer.rs::test_07_03_felucia_108_net_and_usb_round_trip` |
| 07-04 | felucia `boot_order=[scsi0,ide2,net0]` → bootindex 100/101/102; absent device → no token; empty `boot_order` → no-op | ✓ VERIFIED | `root.rs::test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102` (real importer output, exact mapping), `test_07_04_felucia_sata_absent_from_boot_order_gets_no_bootindex`, `test_07_04_empty_boot_order_emits_zero_bootindex_tokens` — all pass |
| 07-05 | felucia + synthetic ordering proofs via string-position, RawArgs verbatim re-verified | ✓ VERIFIED | `tests/qemu_cmdline.rs` — 5/5 tests pass (independently re-run, see below) |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/config/qemu.rs` | `QemuCommandLine`, `QemuContext`, `TryFrom` dispatch, fixed-order `Display` | ✓ VERIFIED | 205 lines; 9-segment struct; `Display` iterates fixed segment order; 4 unit tests pass |
| `src/config/qemu/builder.rs` | `QemuCommandLineBuilder` with per-segment `push_*` | ✓ VERIFIED | 74 lines; 9 distinct `Vec<String>` fields, one `push_*` per segment |
| `src/config/qemu/handlers.rs` | Module declarations | ✓ VERIFIED | 5 lines; declares `root`, `pcie`, `pci`, `storage`, `usb` |
| `src/config/qemu/handlers/root.rs` | Root-device dispatch, all 9 `RootDeviceKind` variants + 5-bus `Chipset` walk | ✓ VERIFIED | 407 lines; all variants present; walks `pcie_bus`/`pci_bus`/`sata_bus`/`ide_bus`/`usb_bus`, sorted by address key each time |
| `src/config/qemu/handlers/pcie.rs` | `PvScsi`/`HostPci`/`Ivshmem`/`VirtioNet` emission | ✓ VERIFIED | 389 lines; 8 unit tests pass |
| `src/config/qemu/handlers/pci.rs` | `GenericPciDevice`/`PvScsi`-on-pci_bus | ✓ VERIFIED | 69 lines; reuses `emit_pvscsi` (no duplicated recursion, confirmed by code read) |
| `src/config/qemu/handlers/storage.rs` | scsi/sata/ide drive+device emission | ✓ VERIFIED | 266 lines; 6 unit tests pass; drive pushed before device in every function |
| `src/config/qemu/handlers/usb.rs` | `GenericUsbDevice` emission | ✓ VERIFIED | 66 lines; 1 unit test (`hostbus`/`hostport` parsing from resource string) |
| `src/config/qemu/bootindex.rs` | Label reconstruction + `lookup_bootindex` | ✓ VERIFIED | 74 lines; 3 unit tests pass; `100 + position`, never panics on empty/absent |
| `src/runtime/cpu.rs`, `src/runtime/vga.rs` | New root device types for D-02 | ✓ VERIFIED (exists/substantive) — ⚠️ emission UNTESTED | See Gap 2 |
| `tests/qemu_cmdline.rs` | Ordering validation suite | ✓ VERIFIED | 221 lines; 5/5 tests pass; `assert_precedes` helper used consistently |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `QemuCommandLine::try_from` | `handlers::root::emit_root_device` | Loop over `runtime.root_devices()` | ✓ WIRED | qemu.rs:69-76 |
| `root.rs` Chipset arm | `handlers::pcie::emit_pcie_device` | Sorted `pcie_bus` iteration | ✓ WIRED | root.rs:26-38 |
| `root.rs` Chipset arm | `handlers::pci::emit_pci_device` | Sorted `pci_bus` iteration | ✓ WIRED | root.rs:41-44 |
| `root.rs` Chipset arm | `handlers::storage::emit_sata_storage`/`emit_ide_storage` | Sorted `sata_bus`/`ide_bus` iteration | ✓ WIRED | root.rs:47-101 |
| `root.rs` Chipset arm | `handlers::usb::emit_usb_device` | Sorted `usb_bus` iteration | ✓ WIRED | root.rs:104-109 |
| `emit_pcie_device` (PvScsi arm) | `emit_pvscsi` → `emit_scsi_storage` | Sorted `scsi_bus` recursion | ✓ WIRED | pcie.rs:31-33, confirmed by passing tests |
| `emit_pci_device` (PvScsi arm) | `handlers::pcie::emit_pvscsi` (shared, not duplicated) | Direct call | ✓ WIRED | pci.rs:26-28; SUMMARY's `grep -c "scsi_bus().iter()"` = 0 independently re-confirmed by code read |
| `root.rs` bootindex sites | `bootindex::lookup_bootindex` | Direct call (sata/ide) or threaded `boot_order: &[String]` (scsi via `emit_pvscsi`/`emit_pci_device`, net via `emit_virtio_net`) | ✓ WIRED | root.rs:63-101, pcie.rs, pci.rs |
| `ProxmoxImporter::into_runtime` | `Q35Chipset.pcie_bus`/`usb_bus` (net/usb) | New import loops (Plan 07-03) | ✓ WIRED | Confirmed by `test_07_03_felucia_108_net_and_usb_round_trip` passing |
| `ProxmoxImporter::into_runtime` | `Runtime.boot_order` | `order=a;b;c` parsing (Plan 07-01) | ✓ WIRED | Confirmed by felucia bootindex test reproducing `["scsi0","ide2","net0"]` from the real `.conf` |
| `RootDeviceKind::SpiceDisplay` arm | *(no test)* | — | ⚠️ ORPHANED (untested) | Code present and wired into the `match`, but never invoked by any test — see Gap 1 |
| `RootDeviceKind::CpuTopology`/`VgaConfig` arms | *(no test)* | — | ⚠️ ORPHANED (untested) | Code present and wired into the `match`, but never invoked by any test — see Gap 2 |

### Behavioral Spot-Checks / Independent Test Run

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full workspace build | `cargo build` | 0 errors, 64 warnings (all pre-existing dead-code warnings from the `bin` target's demo `main.rs`, which does not yet call the QEMU emitter — expected, Phase 8 scope per D-03) | ✓ PASS |
| Full workspace test suite (run once, independently) | `cargo test` | **104/104 passed, 0 failed** — 84 lib + 7 `proxmox_import` + 5 `qemu_cmdline` + 5 `runtime_phase2` + 3 `yaml_round_trip` | ✓ PASS — confirms the SUMMARY.md claim independently |
| Drive/device ordering (felucia) | (part of full run above) `test_qemu_cmdline_felucia_108_drive_before_device_ordering` | ok | ✓ PASS |
| Netdev/device ordering (synthetic) | (part of full run above) `test_qemu_cmdline_synthetic_netdev_before_device_ordering` | ok | ✓ PASS |
| RawArgs verbatim-at-end (felucia) | (part of full run above) `test_qemu_cmdline_felucia_108_rawargs_verbatim_at_end` | ok | ✓ PASS |
| Bootindex exact mapping (felucia) | (part of full run above) `test_07_04_felucia_scsi0_ide2_net0_bootindex_100_101_102` | ok | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| QEMU-01 | 07-01..07-05 | Runtime generates a valid QEMU commandline covering all RUNT-01–07 device types | ⚠️ PARTIAL | 6/7 v1 types + all 5 bus families proven by test; SpiceDisplay emission untested (Gap 1). D-02's CpuTopology/VgaConfig additions also untested (Gap 2), though not part of the original RUNT-01–07 set. |
| QEMU-02 | 07-01..07-05 | Emitter guarantees drive/netdev argument precedes its corresponding `-device` argument | ✓ SATISFIED | Structural segment-order guarantee + string-position tests (felucia + synthetic) |
| QEMU-03 | 07-01, 07-05 | Raw `args` passthrough is appended verbatim at end of generated commandline | ✓ SATISFIED | Verified in isolation and end-to-end after all segments added |

No orphaned requirements found — REQUIREMENTS.md maps only QEMU-01/02/03 to Phase 7 (QEMU-04 correctly deferred to Phase 9), and all three appear in at least one plan's `requirements:` frontmatter field.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No `TODO`/`FIXME`/`TBD`/`XXX`/`HACK`/`PLACEHOLDER` markers found in any phase-modified file | — | None — debt-marker gate clean |
| `src/config/ezkvm/runtime/parser.rs` | (Plan 07-01 deviation) | `RootDeviceKind::CpuTopology \| RootDeviceKind::VgaConfig => {}` no-op arm | ℹ️ Info | Documented, intentional, deferred YAML-schema gap (not a stub introduced to hide missing QEMU behavior — the QEMU emission itself works from `Runtime` directly, independent of YAML round-trip) |
| `tests/qemu_cmdline.rs` | `felucia_runtime_for_cmdline()` | Filters `EfiDisk` out of the felucia-imported `Runtime` before calling `try_from`, to route around a pre-existing (Plan 07-01) importer gap (`EfiDisk.block_device_size_bytes` always `None` for imported configs, which panics `emit_root_device`'s `EfiDisk` arm) | ℹ️ Info | Documented as a known, out-of-scope stub in three separate SUMMARY.md files; does not affect this phase's own truths since `EfiDisk` emission is proven correct via a dedicated synthetic test (`test_07_01_memory_chipset_efidisk_end_to_end`) that supplies `block_device_size_bytes` directly. Flagged here because it means **the felucia fixture alone cannot currently prove "all seven v1 types" simultaneously through the real import path** — a second, independent piece of evidence for Gap 1's underlying coverage concern (EfiDisk works around the gap by using a synthetic Runtime instead; SpiceDisplay has no synthetic-Runtime test at all). |

### Human Verification Required

None. The two gaps identified (SpiceDisplay emission untested; CpuTopology/VgaConfig emission untested) are not behavior-dependent state-transition truths — they are plain string-formatting code, straightforward to verify by adding a unit test. This is a closable gap, not something requiring human judgment/UI/timing verification.

### Gaps Summary

The phase's **hard structural guarantees are solid and independently re-confirmed**: the segment-based builder genuinely enforces drive/netdev-before-device ordering by construction (not by call-order discipline), `RawArgs` is proven appended verbatim after every other segment even after three later plans added new segments, and the bootindex derivation exactly reproduces the real felucia `scsi0→100/ide2→101/net0→102` mapping end-to-end through the real Proxmox importer — including two real importer bugs (ide channel/device decomposition, skipped empty-CD-ROM entries) that Plan 07-04 discovered and fixed along the way. `cargo build`/`cargo test` were run independently in this verification (not trusted from SUMMARY.md) and reproduce the claimed 104/104 passing result exactly.

The one substantive gap is **test-coverage completeness for Success Criterion 1** ("all seven v1 device types"): `SpiceDisplay` is fully implemented (and, on manual code review, correctly follows the same typed-getter/D-08-compliant pattern as every other handler) but has never been exercised by any test — not a synthetic unit test, not the felucia fixture (which the importer never populates a `SpiceDisplay` for). The same is true for the two new root device types (`CpuTopology`, `VgaConfig`) this phase added specifically to satisfy D-02's `-smp`/`-cpu`/`-vga` requirement. Both gaps are low-risk (simple, inspectable formatting code, no ordering/state semantics) and closable with two or three small unit tests, but per this verifier's mandate they cannot be marked VERIFIED on code presence alone when the phase's own testing strategy (D-04) calls for per-device unit tests and none exist for these three device kinds.

**Recommended closure:** add (a) a `SpiceDisplay` emission unit test asserting the `-spice port=...,addr=...,disable-ticketing=on,gl=on,rendernode=...` token shape, and (b) `CpuTopology`/`VgaConfig` emission unit tests asserting `-smp`/`-cpu`/`-vga none`/`-nographic` token shapes (including a non-`"none"` `VgaConfig` case proving no flags are emitted, to document the D-02 scope boundary). This is a small, mechanical addition (no design decisions required) and does not require revisiting any of Phase 7's locked CONTEXT.md decisions.

---

*Verified: 2026-07-24T17:49:20Z*
*Verifier: the agent (gsd-verifier)*
