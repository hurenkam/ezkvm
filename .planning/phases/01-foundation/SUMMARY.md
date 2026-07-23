---
phase: "01-foundation"
status: complete
completed: 2025-07-15
commits:
  - 976d6b8  # Wave 1: thiserror + ProxmoxConversionError
  - aed21e3  # Wave 2: QemuConversionError + Mutex removal
  - ce6169a  # Wave 3: device_kind() on all traits and concrete types
  - 4e63fdb  # Wave 4: migrate all downcast_ref sites
---

# Phase 1 — Foundation: Summary

## Goal

Establish safe, typed foundations before any new conversion code is written:
typed errors via `thiserror`, Mutex removal from RuntimeBuilder, and exhaustive
device dispatch via `device_kind()` replacing all 31 `downcast_ref()` sites.

## What Was Done

### Wave 1 — Plan 01-01: thiserror + ProxmoxConversionError
- Added `thiserror = "2"` to Cargo.toml
- Defined `ProxmoxConversionError` enum in `proxmox.rs`
- Updated `ProxmoxSchemaHandler`, `with_device`, `build`, both `TryFrom` impls to use typed error

### Wave 2 — Plans 01-02 + 01-03: QemuConversionError + Mutex removal
- Defined `QemuConversionError` enum in `qemu.rs`
- Updated all handler/build/TryFrom/free-function signatures in `qemu.rs`
- Removed `Mutex` from `RuntimeBuilder`; changed to consuming `mut self` pattern

### Wave 3 — Plan 01-04: device_kind() on all traits
- Added `PciBusDeviceKind`, `PcieBusDeviceKind`, `UsbBusDeviceKind`, `IsaBusDeviceKind`, `RootDeviceKind` enums
- Extended all 5 bus traits with `fn device_kind()`
- Implemented on: Memory, Chipset, PvScsi (×3 impls), VirtioNetPcie, GenericPciDevice, GenericUsbDevice
- Added `Clone, Copy, PartialEq, Eq, Debug` derives to `StorageDeviceType`
- Default `device_kind()` impl on `SataDevice` and `ScsiDevice` traits via `storage_options().device_type`

### Wave 4 — Plan 01-05: migrate all downcast_ref sites
- Migrated q35.rs: 4 format_* helpers (storage, pcie, pci, usb) → `match device.device_kind()`
- Migrated runtime.rs: `format_root_device` → match on `RootDeviceKind`
- Migrated pvscsi.rs: `format_storage_device` → `match device.storage_options().device_type`
- Migrated parser.rs: 12 production sites + 6 test sites → device_kind() guards + safe `.unwrap()`

## Outcomes

- All 12 tests pass (`cargo test`)
- No unsafe `downcast_ref` chains remain; all remaining calls are guarded by `device_kind()`
- `thiserror`-based typed errors in both proxmox and qemu conversion layers
- RuntimeBuilder is Mutex-free and uses idiomatic consuming builder pattern
- Codebase is ready for Phase 2 (Proxmox parser expansion)

## Key Decisions Made During Execution

- `RuntimeBuilder::build()` keeps `Result<Runtime, ()>` signature; TryFrom impls use `.map_err(|_| unreachable!())` to bridge — avoids breaking call sites
- `IsaDevice` trait changed from empty marker to having `as_any() + device_kind()` (breaking for implementors — only PvScsi affected, updated accordingly)
- Bus-kind enums named `PciBusDeviceKind` etc. to avoid collision with per-device sub-kind enums (`PciDeviceKind`, `UsbDeviceKind`) in generic device files
- `StorageDeviceType::Odd` maps to the string `"Cdrom"` everywhere — preserved at all display and schema sites
