---
phase: 07-qemu-cmdline
plan: 02
subsystem: infra
tags: [qemu, cmdline-emitter, pcie-bus, nested-dispatch, storage]

requires:
  - phase: 07-qemu-cmdline (plan 07-01)
    provides: Segmented QemuCommandLineBuilder/QemuContext/TryFrom<(Runtime, QemuContext)> skeleton, root-level handlers, Runtime boot_order/CpuTopology/VgaConfig
provides:
  - "src/config/qemu/handlers/pcie.rs — emit_pcie_device exhaustive dispatch over all 4 PcieBusDeviceKind variants (PvScsi, HostPci, Ivshmem, VirtioNet)"
  - "src/config/qemu/handlers/storage.rs — shared emit_scsi_storage/storage_device_model drive+device pair emission, reused by Plan 07-03's pci_bus PvScsi arm"
  - "root.rs's Chipset arm now walks Q35Chipset.pcie_bus (sorted by (device, function)) instead of the Plan 07-01 no-op"
affects: [07-qemu-cmdline (plan 07-03 reuses emit_pvscsi/emit_scsi_storage for pci_bus/sata_bus/ide_bus), 08-vm-lifecycle, 09-round-trip-verification]

tech-stack:
  added: []
  patterns:
    - "Two-level nested-bus dispatch: root device (Chipset) → bus map (pcie_bus) → per-entry device_kind() dispatch → (for PvScsi) a third-level scsi_bus recursion, each level sorting HashMap entries by address key before iterating (mirrors Q35Chipset::fmt's sort-then-render idiom per D-08, reimplemented independently — no Display/.to_string() dependency)"
    - "Shared storage emission module (storage.rs) parameterized by bus name string (\"scsi\"/\"ide\"/\"sata\") so Plan 07-03 can add sata/ide callers without duplicating drive+device formatting logic"

key-files:
  created:
    - src/config/qemu/handlers/pcie.rs
    - src/config/qemu/handlers/storage.rs
  modified:
    - src/config/qemu/handlers.rs
    - src/config/qemu/handlers/root.rs

key-decisions:
  - "D-08 followed: emit_pvscsi/emit_hostpci/emit_ivshmem/emit_virtio_net all read Runtime types via typed getters only (.scsi_bus(), .target(), .lun(), .functions(), .base_bdf(), .id(), .mem_path(), .size(), .mac_address(), .rx_queue_size(), .tx_queue_size(), .vhost()) — no Runtime Display/.to_string() reliance anywhere in the new handlers"
  - "storage_device_model's sata/ide arms both return ide-hd/ide-cd (no sata-hd QEMU device model exists), per the plan's explicit rationale — Plan 07-03 will call this same function for its sata_bus/ide_bus callers"
  - "HostPci multifunction=on placement rule implemented exactly as specified: only on function 0, and only when functions.len() > 1 (Pitfall 4)"
  - "Ivshmem pushed devices before objects in call order (call-order doesn't matter — QemuCommandLine::Display's fixed segment order always renders objects before devices, verified by test)"

patterns-established:
  - "emit_pvscsi is pub(crate) specifically so Plan 07-03's pci_bus PvScsi arm can call it without duplicating the scsi_bus sort+recurse logic — this is the reuse seam the plan calls out explicitly"

requirements-completed: [QEMU-01, QEMU-02]

coverage:
  - id: D1
    description: "Q35Chipset.pcie_bus is walked (sorted by device/function) and dispatched for all 4 PcieBusDeviceKind variants; root.rs's Chipset arm calls handlers::pcie::emit_pcie_device for each sorted entry"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_root_chipset_walks_pcie_bus_for_pvscsi_and_hostpci"
        status: pass
    human_judgment: false
  - id: D2
    description: "PvScsi controller line + scsi_bus recursion emits correctly-ordered drive+device pairs for Ssd/Hdd/Cdrom, sorted by (target, lun) not insertion order"
    requirement: "QEMU-02"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_pvscsi_controller_and_single_ssd_end_to_end"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_pvscsi_cdrom_and_hdd_device_models"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_pvscsi_scsi_bus_sorted_by_target_not_insertion_order"
        status: pass
    human_judgment: false
  - id: D3
    description: "HostPci multifunction=on appears only on function 0's device line, only when a companion function exists (Pitfall 4)"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_hostpci_multifunction_flag_only_on_function_zero"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_hostpci_single_function_no_multifunction_flag"
        status: pass
    human_judgment: false
  - id: D4
    description: "Ivshmem emits object+device pair with objects segment preceding devices segment in fixed Display order; VirtioNet emits netdev+device pair with netdevs preceding devices"
    requirement: "QEMU-02"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_ivshmem_object_before_device_in_display_order"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/pcie.rs#tests::test_07_02_virtio_net_netdev_before_device_in_display_order"
        status: pass
    human_judgment: false
  - id: D5
    description: "storage_device_model maps StorageDeviceType x bus-family to the correct QEMU device model string (scsi-hd/scsi-cd/ide-hd/ide-cd) shared across scsi/sata/ide callers"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_02_storage_device_model_scsi_and_ide_variants"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_02_scsi_hd_drive_and_device_no_media_token"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_02_scsi_cd_drive_has_media_cdrom_token"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-07-24
status: complete
---

# Phase 7 Plan 2: PCIe-Bus Nested Dispatch + Shared Storage Emission Summary

**Implemented the PCIe-bus branch of Q35Chipset's nested-device dispatch (all 4 `PcieBusDeviceKind` variants: PvScsi, HostPci, Ivshmem, VirtioNet), including one level of recursion into PvScsi's `scsi_bus`, and stood up the shared `storage.rs` drive+device emission module Plan 07-03 reuses for pci_bus/sata_bus/ide_bus**

## Performance

- **Duration:** ~20 min
- **Tasks:** 3/3 completed
- **Files modified:** 4 (2 created, 2 modified)

## Accomplishments
- Created `src/config/qemu/handlers/storage.rs` with `storage_device_model()` (StorageDeviceType × bus-family → QEMU device model string) and `emit_scsi_storage()` (drive+device pair emission), designed for reuse by Plan 07-03's sata/ide callers
- Created `src/config/qemu/handlers/pcie.rs` with `emit_pcie_device()` — an exhaustive match over all 4 `PcieBusDeviceKind` variants — plus `emit_pvscsi()` (controller line + sorted `scsi_bus` recursion, `pub(crate)` for Plan 07-03 reuse), `emit_hostpci()` (multifunction vfio-pci emission per Pitfall 4's exact rule), `emit_ivshmem()` (object+device pair), and `emit_virtio_net()` (netdev+device pair)
- Wired `root.rs`'s `Chipset` arm to sort and walk `Q35Chipset.pcie_bus` (mirroring `Q35Chipset::fmt`'s sort-then-render idiom, reimplemented independently per D-08), replacing Plan 07-01's deliberate no-op
- Updated `handlers.rs` to declare `mod pcie;` and `mod storage;` alongside the existing `mod root;`

## Task Commits

**Per standing project policy, no commits were made.** All changes are left as unstaged/untracked working-tree edits for the user to review and commit themselves.

1. **Task 1 (tracer): storage.rs shared scsi emission + PvScsi controller + scsi_bus recursion** - Not committed — per project's standing no-auto-commit policy; changes left unstaged
2. **Task 2: HostPci multifunction vfio-pci emission** - Not committed — per project's standing no-auto-commit policy; changes left unstaged
3. **Task 3: Ivshmem object+device pair, VirtioNet netdev+device pair, wire root.rs pcie_bus walk** - Not committed — per project's standing no-auto-commit policy; changes left unstaged

**Plan metadata:** Not committed — per project's standing no-auto-commit policy; changes left unstaged

_Note: No TDD RED/GREEN/REFACTOR commit sequence exists either, for the same reason — tests were written and verified green in the working tree only._

## Files Created/Modified
- `src/config/qemu/handlers/storage.rs` - New: `storage_device_model()` helper + `emit_scsi_storage()` drive+device pair emission, plus 3 unit tests
- `src/config/qemu/handlers/pcie.rs` - New: `emit_pcie_device()` exhaustive dispatch + `emit_pvscsi()`/`emit_hostpci()`/`emit_ivshmem()`/`emit_virtio_net()`, plus 8 unit tests covering Tasks 1-3's behavior blocks
- `src/config/qemu/handlers.rs` - Added `pub(crate) mod pcie;` and `pub(crate) mod storage;` declarations
- `src/config/qemu/handlers/root.rs` - `Chipset` arm now downcasts to `Q35Chipset`, sorts `pcie_bus` entries by `(device, function)`, and calls `handlers::pcie::emit_pcie_device` for each; removed the now-unreachable `_ => {}` catch-all arm (see Deviations)

## Decisions Made
- Followed D-08 strictly: every new handler function reads `Runtime` types via typed getters only — no `Display`/`.to_string()` dependency on any `Runtime` type anywhere in the new code
- `storage_device_model`'s `"ide"` and `"sata"` bus arms both return `ide-hd`/`ide-cd` (confirmed no `sata-hd` QEMU device model exists), matching the plan's explicit rationale — this shared function is ready for Plan 07-03's sata_bus/ide_bus callers without modification
- `emit_pvscsi` is `pub(crate)` (not private) specifically so Plan 07-03's `pci_bus` `PvScsi` arm can call it directly, per the plan's `key_links` note
- Implemented all three tasks' production code together before adding tests incrementally per task's `<behavior>` block, since all three tasks in this plan build on the same `pcie.rs` file and splitting into separate no-op-then-fill passes would have produced identical final code with more churn

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed now-unreachable `_ => {}` catch-all arm in `root.rs`'s `emit_root_device` match**
- **Found during:** Task 3, after wiring the `Chipset` arm and running `cargo build`
- **Issue:** `cargo build` emitted an `unreachable_patterns` warning: the pre-existing `_ => { // handled by Task 3 in this same plan }` catch-all in Plan 07-01's `RootDeviceKind` match became unreachable once all 9 `RootDeviceKind` variants (Memory, Chipset, EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs, CpuTopology, VgaConfig) had explicit arms — the comment referred to this same plan's Task 3, but the wildcard was structurally dead code once the match became exhaustive
- **Fix:** Removed the `_ => {}` arm entirely; the match is now exhaustive over `RootDeviceKind` with no wildcard
- **Files modified:** `src/config/qemu/handlers/root.rs`
- **Verification:** `cargo build` produces zero `unreachable_patterns` warnings for this match; full `cargo test` suite still green
- **Committed in:** Not committed — per project's standing no-auto-commit policy

---

**Total deviations:** 1 auto-fixed (compiler-warning cleanup, no behavior change)
**Impact on plan:** None on emitted QEMU output — purely removes dead code the compiler flagged once this plan's task made the match exhaustive.

## Issues Encountered
None beyond the deviation documented above.

## Known Stubs

None introduced by this plan. `PciBusDeviceKind::PvScsi` (the `pci_bus` recursion, as opposed to this plan's `pcie_bus` recursion), `sata_bus`, `ide_bus`, and `usb_bus` remain the deliberate no-op left by Plan 07-01/07-02 for Plan 07-03, per the updated `// pci/sata/ide/usb bus walk added in Plan 07-03` comment in `root.rs`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `handlers::pcie::emit_pvscsi` and `handlers::storage::emit_scsi_storage`/`storage_device_model` are `pub(crate)` and ready for Plan 07-03 to call directly from the `pci_bus` `PvScsi` arm and new `sata_bus`/`ide_bus` handlers, without duplicating logic
- `root.rs`'s `Chipset` arm's `// pci/sata/ide/usb bus walk added in Plan 07-03` comment marks the exact extension point
- `bootindex` parameter on `emit_scsi_storage` is threaded through as `None` everywhere in this plan; Plan 07-04 replaces those call sites with a computed lookup — the function signature does not need to change

---
*Phase: 07-qemu-cmdline*
*Completed: 2026-07-24*

## Self-Check: PASSED

All 4 files claimed as created/modified verified present on disk:
`src/config/qemu/handlers/pcie.rs`, `src/config/qemu/handlers/storage.rs`,
`src/config/qemu/handlers.rs`, `src/config/qemu/handlers/root.rs`.

No commit hashes to verify — per the project's standing no-auto-commit policy, no
`git add`/`git commit` was run at any point during this execution. All changes remain
unstaged/untracked working-tree edits.

Full workspace test suite (`cargo test`) confirmed green after all three tasks:
83 tests total (68 lib + 7 proxmox_import + 5 runtime_phase2 + 3 yaml_round_trip) — 0 failed.
`cargo build` succeeds with zero errors and zero new non-dead-code warnings.
