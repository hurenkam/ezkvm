---
phase: "02-runtime-model"
status: complete
completed: 2025-07-15
commits:
  - 36e6412  # Wave 1: EfiDisk tracer
  - 83989c2  # Wave 2: TpmState, AudioDevice, SpiceDisplay, RawArgs, HostPci, Ivshmem
  - d6e40ad  # Wave 3: wire all types + integration test
---

# Phase 2 — Runtime Model: Summary

## Goal

Complete the v1 Runtime device vocabulary by adding seven first-class structs:
five `RootDevice` types and two `PcieDevice` types, giving downstream phases
(Proxmox importer, YAML, QEMU emitter) a complete, stable target API.

## What Was Done

### Wave 1 — Plan 02-01: EfiDisk tracer (end-to-end)
- Created `src/runtime/efidisk.rs` — 6 fields, dual-size pitfall documented
- Added `RootDeviceKind::EfiDisk` variant
- Added `mod efidisk`, `pub use EfiDisk`, `with_efidisk()` builder method to `runtime.rs`
- Added wildcard `_ =>` arm to `parser.rs` root device match (future-proofing)

### Wave 2 — Plans 02-02 + 02-03: remaining struct files
- Created `src/runtime/tpmstate.rs` — TpmState (storage_volume, version)
- Created `src/runtime/audio.rs` — AudioDevice (device_type, driver)
- Created `src/runtime/spice.rs` — SpiceDisplay (6 fields)
- Created `src/runtime/rawargs.rs` — RawArgs(pub String) opaque tuple newtype
- Created `src/runtime/devices/hostpci.rs` — HostPci (6 fields, multi-function Vec<u8>)
- Created `src/runtime/devices/ivshmem.rs` — Ivshmem (3 fields)
- Wired HostPci + Ivshmem into `devices.rs` (mod + pub use)
- Added `PcieBusDeviceKind::HostPci` and `::Ivshmem` variants to `pcie.rs`
- Added HostPci + Ivshmem to `pub use devices` in `runtime.rs`
- Added HostPci + Ivshmem arms to `format_pcie_device()` in `q35.rs`

### Wave 3 — Plan 02-04: wiring + integration test
- Added mod/pub use for TpmState, AudioDevice, SpiceDisplay, RawArgs in `runtime.rs`
- Added 4 new `RootDeviceKind` variants
- Added 4 new builder methods (with_tpmstate, with_audio_device, with_spice_display, with_raw_args)
- Added `with_host_pci` + `with_ivshmem` to `Q35ChipsetBuilder`
- Created `src/lib.rs` to expose library target for integration tests
- Created `tests/runtime_phase2.rs` with 5 passing tests covering all 7 new types

## Outcomes

- All 29 tests pass (12 unit × 2 targets + 5 integration)
- 7 new device types available via `crate::runtime::*`
- Runtime model is now complete enough for felucia/108.conf device set
- `src/lib.rs` added — project now has both binary and library targets

## Key Decisions Made During Execution

- Binary-only crate required `src/lib.rs` addition to support integration tests
- HostPci/Ivshmem `pub use` added to `runtime.rs` in Wave 2 (slightly ahead of Wave 3 plan) to resolve non-exhaustive match in `format_pcie_device()`
- `format_root_device()` uses `_ =>` wildcard for new variants — full Display deferred to later phases
- `parser.rs` root device match gets `_ => {}` wildcard to stay future-proof without blocking compilation
