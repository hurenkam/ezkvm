---
phase: 07-qemu-cmdline
plan: 03
subsystem: infra
tags: [qemu, cmdline-emitter, pci-bus, sata-bus, ide-bus, usb-bus, proxmox-importer]

requires:
  - phase: 07-qemu-cmdline (plan 07-02)
    provides: "handlers::pcie::emit_pvscsi (pub(crate), reuse seam), handlers::storage shared drive+device emission (storage_device_model, emit_scsi_storage), root.rs's Chipset arm walking pcie_bus"
provides:
  - "src/config/qemu/handlers/storage.rs — emit_sata_storage/emit_ide_storage, sharing storage_device_model with Plan 07-02's emit_scsi_storage"
  - "src/config/qemu/handlers/pci.rs — emit_pci_device, dispatching GenericPciDevice inline and reusing handlers::pcie::emit_pvscsi for PvScsi-on-pci_bus (no duplicated recursion)"
  - "src/config/qemu/handlers/usb.rs — emit_usb_device, dispatching GenericUsbDevice's 3 UsbDeviceKind variants (HostPassthrough/Tablet/NetworkController)"
  - "root.rs's Chipset arm now walks all 5 Q35Chipset bus maps (pcie_bus, pci_bus, sata_bus, ide_bus, usb_bus), each sorted by address key before iterating"
  - "ProxmoxImporter::into_runtime() parses net/usb Proxmox conf entries into Q35Chipset.pcie_bus (VirtioNetPcie)/usb_bus (GenericUsbDevice), closing the import-side D-01 gap RESEARCH.md flagged as previously silently dropped"
affects: [07-qemu-cmdline (plan 07-04 bootindex wiring reuses emit_sata_storage/emit_ide_storage's bootindex parameter and root.rs's net/usb address plumbing), 08-vm-lifecycle, 09-round-trip-verification]

tech-stack:
  added: []
  patterns:
    - "Nested-bus dispatch completed for all 5 Q35Chipset bus maps — same sort-then-render idiom as Plan 07-02 (HashMap entries collected to Vec, sorted by address key, then iterated), applied independently to pci_bus/sata_bus/ide_bus/usb_bus"
    - "Cross-bus reuse: a device kind (PvScsi) appearing on more than one bus (pcie_bus in Plan 07-02, pci_bus here) is emitted by exactly one function, called from both bus-walk sites — no copy-pasted controller/recursion logic"

key-files:
  created:
    - src/config/qemu/handlers/pci.rs
    - src/config/qemu/handlers/usb.rs
  modified:
    - src/config/qemu/handlers/storage.rs
    - src/config/qemu/handlers/root.rs
    - src/config/qemu/handlers.rs
    - src/config/proxmox/importer.rs

key-decisions:
  - "D-08 followed: emit_sata_storage/emit_ide_storage/emit_pci_device/emit_usb_device all read Runtime types via typed getters only (.port(), .channel(), .device(), .kind(), .mac_address(), etc.) — no Runtime Display/.to_string() reliance anywhere in the new/modified handlers"
  - "ide label computed as channel*2+device (not hardcoded), matching the felucia sample's confirmed ide2 mapping from RESEARCH.md's Critical Gap table"
  - "PvScsi-on-pci_bus calls handlers::pcie::emit_pvscsi directly (D-01 'shared, not rebuilt') — grep -c 'scsi_bus().iter()' src/config/qemu/handlers/pci.rs returns 0, confirming no duplicated recursion"
  - "UsbDeviceKind::HostPassthrough.resource is split on the first '-' via .expect() (not a typed error) — per D-06, this data originates from already-typed Proxmox .conf data (Phase 3/4 boundary), so a malformed value is a programmer error, not a runtime-data conversion failure"
  - "ProxmoxImporter's new net loop places VirtioNetPcie entries starting at pcie slot 20 (documented inline) to avoid colliding with the existing pvscsi slot (16) and the hostpci slot range (0-N)"
  - "ProxmoxImporter's new usb loop uses the Proxmox conf key index (idx.to_string()) as the UsbAddress port — sufficient for structural correctness per D-05; no claim of matching real xhci port topology"

patterns-established:
  - "emit_pci_device/emit_usb_device follow the identical device_kind() -> match -> downcast_ref::<Concrete>().unwrap() three-step shape established in Plan 07-01/07-02, applied now to the two remaining un-implemented PciBusDeviceKind/UsbBusDeviceKind dispatch points"

requirements-completed: [QEMU-01, QEMU-02]

coverage:
  - id: D1
    description: "Hdd/sata0 and Cdrom/ide-channel1-device0 each emit correct drive+device pairs via new emit_sata_storage/emit_ide_storage, sorted by (port,device)/(channel,device) not insertion order"
    requirement: "QEMU-02"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_03_sata_hdd_drive_and_device"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_03_ide_cdrom_channel1_device0_label_ide2"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/storage.rs#tests::test_07_03_ide_entries_sorted_by_channel_device_not_insertion_order"
        status: pass
    human_judgment: false
  - id: D2
    description: "GenericPciDevice(Ac97) on pci_bus emits '-device ac97'; PvScsi found on pci_bus emits an identical controller+drive+device triple to Plan 07-02's pcie_bus PvScsi test, by calling handlers::pcie::emit_pvscsi (no duplicated recursion)"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/pci.rs#tests::test_07_03_generic_pci_ac97_emits_device"
        status: pass
      - kind: unit
        ref: "src/config/qemu/handlers/pci.rs#tests::test_07_03_pvscsi_on_pci_bus_reuses_emit_pvscsi"
        status: pass
    human_judgment: false
  - id: D3
    description: "GenericUsbDevice(HostPassthrough{resource:'1-2.2'}) emits '-device usb-host,...,hostbus=1,hostport=2.2,id=usb0' with hostbus/hostport parsed (not hardcoded) from the resource string"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/qemu/handlers/usb.rs#tests::test_07_03_usb_host_passthrough_parses_hostbus_hostport"
        status: pass
    human_judgment: false
  - id: D4
    description: "ProxmoxImporter::into_runtime() populates Q35Chipset.pcie_bus with a VirtioNetPcie per net entry and usb_bus with a GenericUsbDevice per usb entry; felucia 108.conf fixture round-trips with exactly 1 VirtioNetPcie and 1 usb_bus entry, and the pre-existing root_devices().len()==8 assertion (Phase 4/Plan 07-01) is unaffected"
    requirement: "QEMU-01"
    verification:
      - kind: unit
        ref: "src/config/proxmox/importer.rs#tests::test_07_03_net0_parsed_into_pcie_bus_virtio_net"
        status: pass
      - kind: unit
        ref: "src/config/proxmox/importer.rs#tests::test_07_03_usb0_parsed_into_usb_bus_host_passthrough"
        status: pass
      - kind: unit
        ref: "src/config/proxmox/importer.rs#tests::test_07_03_felucia_108_net_and_usb_round_trip"
        status: pass
      - kind: integration
        ref: "tests/proxmox_import.rs#test_proxmox_import_felucia_108_root_devices"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-07-24
status: complete
---

# Phase 7 Plan 3: PCI/SATA/IDE/USB Bus Dispatch + Importer Net/USB Wiring Summary

**Completed nested-bus device dispatch across the remaining `pci_bus`/`sata_bus`/`ide_bus`/`usb_bus` maps (reusing Plan 07-02's `emit_pvscsi`/storage helpers where a device kind appears on more than one bus), and closed the import-side half of D-01 by wiring `ProxmoxImporter::into_runtime()` to construct `VirtioNetPcie`/`GenericUsbDevice` entries from previously-parsed-but-dropped `net`/`usb` Proxmox conf fields**

## Performance

- **Duration:** ~20 min
- **Tasks:** 3/3 completed
- **Files modified:** 6 (2 created, 4 modified)

## Accomplishments
- Added `emit_sata_storage`/`emit_ide_storage` to `src/config/qemu/handlers/storage.rs`, sharing `storage_device_model` with Plan 07-02's `emit_scsi_storage`; ide label computed as `channel*2+device` (matches felucia's confirmed `ide2` mapping)
- Created `src/config/qemu/handlers/pci.rs` with `emit_pci_device`, dispatching `GenericPciDevice` inline (`ac97`/`e1000`/`qxl-vga` model mapping) and calling `handlers::pcie::emit_pvscsi` directly for `PvScsi` found on `pci_bus` — zero duplicated `scsi_bus` recursion (verified via `grep -c`)
- Created `src/config/qemu/handlers/usb.rs` with `emit_usb_device`, dispatching all 3 `UsbDeviceKind` variants (`HostPassthrough` parsing `hostbus`/`hostport` from the `"<bus>-<port>"` resource string, `Tablet`, `NetworkController`)
- Wired `root.rs`'s `Chipset` arm to sort-then-walk all 5 `Q35Chipset` bus maps (`pcie_bus` from Plan 07-02, `pci_bus`/`sata_bus`/`ide_bus`/`usb_bus` added here), replacing the `// pci/sata/ide/usb bus walk added in Plan 07-03` placeholder
- Wired `ProxmoxImporter::into_runtime()`'s new `net`/`usb` parse loops (after the existing `hostpci` loop) to construct `VirtioNetPcie`/`GenericUsbDevice` entries from `ProxmoxVmConf.net`/`.usb`, previously parsed-but-silently-dropped per RESEARCH.md Pitfall 7

## Task Commits

**Per standing project policy, no commits were made.** All changes are left as unstaged/untracked working-tree edits for the user to review and commit themselves.

1. **Task 1: sata/ide storage emission + root.rs sata_bus/ide_bus walk** - Not committed — per project's standing no-auto-commit policy; changes left unstaged
2. **Task 2: pci.rs (GenericPci + PvScsi reuse) and usb.rs (GenericUsbDevice) + root.rs pci_bus/usb_bus walk** - Not committed — per project's standing no-auto-commit policy; changes left unstaged
3. **Task 3: ProxmoxImporter wiring for net0/usb0 (D-01 import-side completion)** - Not committed — per project's standing no-auto-commit policy; changes left unstaged

**Plan metadata:** Not committed — per project's standing no-auto-commit policy; changes left unstaged

_Note: No TDD RED/GREEN/REFACTOR commit sequence exists either, for the same reason — tests were written and verified green in the working tree only._

## Files Created/Modified
- `src/config/qemu/handlers/storage.rs` - Added `emit_sata_storage`/`emit_ide_storage`, plus 3 new unit tests (Task 1)
- `src/config/qemu/handlers/pci.rs` - New: `emit_pci_device` (GenericPci inline dispatch, PvScsi reuse via `handlers::pcie::emit_pvscsi`), plus 2 unit tests
- `src/config/qemu/handlers/usb.rs` - New: `emit_usb_device` (all 3 `UsbDeviceKind` variants), plus 1 unit test
- `src/config/qemu/handlers.rs` - Added `pub(crate) mod pci;` and `pub(crate) mod usb;` declarations
- `src/config/qemu/handlers/root.rs` - `Chipset` arm now sorts+walks `pci_bus`/`sata_bus`/`ide_bus`/`usb_bus` in addition to Plan 07-02's `pcie_bus` walk; removed the `// pci/sata/ide/usb bus walk added in Plan 07-03` placeholder comment
- `src/config/proxmox/importer.rs` - Added `net`/`usb` parse loops after the existing `hostpci` loop, plus 3 new unit tests (Task 3)

## Decisions Made
- Followed D-08 strictly: every new/modified handler function reads `Runtime` types via typed getters only — no `Display`/`.to_string()` dependency on any `Runtime` type
- `emit_pci_device`'s `PvScsi` arm calls `handlers::pcie::emit_pvscsi` directly rather than reimplementing the controller+`scsi_bus`-recursion logic, satisfying the plan's explicit reuse requirement (verified: `grep -c "scsi_bus().iter()" src/config/qemu/handlers/pci.rs` returns `0`)
- `UsbDeviceKind::HostPassthrough.resource.split_once('-')` uses `.expect()` with a clear message per D-06/T-07-05 — the data originates from already-typed Proxmox conf data, so a malformed value is a programmer-error panic, not a typed `QemuConversionError` variant
- Net devices in `ProxmoxImporter` start at pcie slot 20 (documented inline comment) to avoid colliding with the existing pvscsi slot (16) and hostpci's `0..N` slot range

## Deviations from Plan

None - plan executed exactly as written. All three tasks' behavior blocks, acceptance criteria, and verification commands were implemented and passed without needing bug fixes, missing-functionality additions, or blocking-issue workarounds beyond what the plan already specified.

## Issues Encountered

None.

## Known Stubs

None introduced by this plan. All 5 `Q35Chipset` bus maps are now fully dispatched with real emission logic (no placeholders remain in `root.rs`'s `Chipset` arm). `bootindex` parameters on `emit_sata_storage`/`emit_ide_storage` are threaded through as `None` everywhere in this plan (mirroring Plan 07-02's `emit_scsi_storage` convention) — Plan 07-04 replaces those `None` literals with a computed lookup; the function signatures do not need to change.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All 5 `Q35Chipset` bus maps (`pcie_bus`, `pci_bus`, `sata_bus`, `ide_bus`, `usb_bus`) are walked and dispatched — `QEMU-01`'s full device-family coverage goal is met
- `ProxmoxImporter::into_runtime()` no longer silently drops `net`/`usb` Proxmox config entries — the felucia 108.conf fixture now round-trips with 1 `VirtioNetPcie` and 1 `usb_bus` entry in addition to the existing pvscsi/sata/ide/hostpci coverage
- `emit_sata_storage`/`emit_ide_storage`'s `bootindex: Option<u32>` parameter (currently always `None`) is ready for Plan 07-04 to replace with a computed lookup from `Runtime::boot_order()`, without changing the function signature
- Pre-existing unrelated warning (`unused import: Q35Chipset` in `importer.rs`'s test module, introduced by a prior plan) was left untouched — out of this plan's scope per the deviation-rules scope boundary (not caused by this plan's changes)

---
*Phase: 07-qemu-cmdline*
*Completed: 2026-07-24*

## Self-Check: PASSED

All 6 files claimed as created/modified verified present on disk:
`src/config/qemu/handlers/storage.rs`, `src/config/qemu/handlers/pci.rs`,
`src/config/qemu/handlers/usb.rs`, `src/config/qemu/handlers.rs`,
`src/config/qemu/handlers/root.rs`, `src/config/proxmox/importer.rs`.

No commit hashes to verify — per the project's standing no-auto-commit policy, no
`git add`/`git commit` was run at any point during this execution. All changes remain
unstaged/untracked working-tree edits.

Full workspace test suite (`cargo test`) confirmed green after all three tasks:
92 tests total (77 lib + 7 proxmox_import + 5 runtime_phase2 + 3 yaml_round_trip) — 0 failed.
`cargo build` succeeds with zero errors. `grep -c "scsi_bus().iter()" src/config/qemu/handlers/pci.rs`
returns `0`, confirming the PvScsi-on-pci_bus reuse requirement.
