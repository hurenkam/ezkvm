# Phase 7: QEMU Cmdline - Pattern Map

**Mapped:** 2026-07-24
**Files analyzed:** 9 (new/modified)
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|-----------------|---------------|
| `src/config/qemu.rs` (rewrite) | service/transform (entry point, `TryFrom`) | transform | `src/config/qemu.rs` (existing stub) + `src/config/proxmox/importer.rs` | exact (existing file to extend) |
| `src/config/qemu/builder.rs` | utility (segmented accumulator) | transform | `src/runtime/q35.rs` (`Q35Chipset::fmt`, sort-then-render) | role-match |
| `src/config/qemu/handlers/root.rs` | service (dispatch over `RootDeviceKind`) | transform | `src/runtime.rs::format_root_device` | exact |
| `src/config/qemu/handlers/pcie.rs` / `pci.rs` | service (dispatch over nested bus) | transform | `src/runtime/q35.rs::format_pcie_device` / `format_pci_device` | exact |
| `src/config/qemu/handlers/storage.rs` | service (shared drive/device emit) | transform | `src/runtime/q35.rs::format_storage_device` + `src/config/proxmox/importer.rs` (`classify_disk_*`) | role-match |
| `src/config/qemu/handlers/usb.rs` | service (dispatch over usb bus) | transform | `src/runtime/q35.rs::format_usb_device` | exact |
| `src/config/qemu/bootindex.rs` | utility (id reconstruction + lookup) | transform | `src/config/proxmox/importer.rs` (id/address reconstruction idioms) | role-match |
| `src/runtime.rs` (add `boot_order: Vec<String>` field) | model | CRUD (builder field) | `src/runtime.rs` (`RuntimeBuilder` existing fields/methods) | exact |
| `src/config/proxmox/importer.rs` (parse `boot` string) | service (import/parse) | transform | same file's existing per-device parse loops (e.g. `hostpci` loop) | exact |
| `src/config/qemu/error.rs` (or enum in `qemu.rs`) | error type | n/a | `src/config/proxmox/error.rs` (`ProxmoxImportError`) | exact |
| `tests/qemu_cmdline.rs` (new) | test | request-response (structural assertions) | `tests/runtime_phase2.rs`, `tests/proxmox_import.rs` | exact |

## Pattern Assignments

### `src/config/qemu.rs` (rewrite: `QemuCommandLine`, `QemuContext`, `TryFrom`)

**Analog:** existing `src/config/qemu.rs` stub (structure to keep) + `src/config/proxmox/importer.rs` (multi-step `TryFrom`/builder composition style)

**Existing stub shape to extend, not discard** (full file, ~76 lines):
```rust
use crate::runtime::{RootDevice, Runtime, RuntimeBuilder};
use std::{any::TypeId, collections::HashMap};

#[derive(Debug, thiserror::Error)]
pub enum QemuConversionError {
    #[error("no qemu handler for device '{name}'")]
    NoHandler { name: String },
}

pub struct QemuSchemaBuilder {
    schema: QemuSchema,
    handlers: HashMap<TypeId, QemuSchemaHandler>,
}
impl QemuSchemaBuilder {
    pub fn with_device(&mut self, device: &dyn RootDevice) -> Result<(), QemuConversionError> {
        let device_type = device.get_type();
        if let Some(handler) = self.handlers.get(&device_type) {
            handler(self, device)
        } else {
            Err(QemuConversionError::NoHandler { name: device.get_name().to_string() })
        }
    }
}

impl TryFrom<Runtime> for QemuSchema {
    type Error = QemuConversionError;
    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let handlers = HashMap::new();
        let mut builder = QemuSchemaBuilder::new(handlers);
        for device in value.root_devices() {
            builder.with_device(device.as_ref())?;
        }
        builder.build()
    }
}
```
**What to change:** Replace `TypeId`-keyed handler map + single `QemuSchema {}` with `device_kind()`-based `match` dispatch (per D-01, Phase 1's `device_kind()` convention — see `src/runtime.rs::format_root_device` below) writing into a 9-field segmented `QemuCommandLineBuilder` instead of an empty schema. Keep the `TryFrom` + `thiserror` error-enum shape — it is already the project convention.

**New signature per CONTEXT.md D-01/D-06:**
```rust
pub struct QemuContext { /* vm_name, socket_paths, storage_paths — caller-built, D-06 */ }

impl TryFrom<(Runtime, QemuContext)> for QemuCommandLine {
    type Error = QemuConversionError;
    fn try_from((runtime, ctx): (Runtime, QemuContext)) -> Result<Self, Self::Error> { ... }
}

impl std::fmt::Display for QemuCommandLine { /* see builder.rs pattern below */ }
```

---

### `src/config/qemu/builder.rs` (segmented accumulator)

**Analog:** `src/runtime/q35.rs` — `Q35Chipset`'s `Display` impl, lines showing sort-then-render into a formatted tree (full file read; segment pattern lives in `fmt`):

```rust
// Source: src/runtime/q35.rs — Display impl pattern to mirror (sort before render)
impl std::fmt::Display for Q35Chipset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  +-pcie:")?;
        let mut pcie_entries: Vec<_> = self.pcie_bus.iter().collect();
        pcie_entries.sort_by_key(|(address, _)| (*address.device(), *address.function()));
        for (address, device) in pcie_entries {
            let rendered = format_pcie_device(device.as_ref()).replace('\n', "\n         ");
            writeln!(f, "      +-{}: {}", address, rendered)?;
        }
        // ... repeated per bus (pci, sata, ide, usb), always sorted first
    }
}
```

**Apply this idiom to `QemuCommandLineBuilder`:** instead of sorting entries into one string, push formatted tokens into typed `Vec<String>` fields (`machine`, `firmware`, `drives`, `netdevs`, `chardevs`, `tpm`, `objects`, `devices`, `misc`), and concatenate those fields in fixed order at `Display`/`build()` time — same "collect, sort/segment, then render in a fixed pass" shape as `Q35Chipset::fmt`, just keyed by segment instead of by bus. Any bus-map iteration (`pcie_bus`, `sata_bus`, `ide_bus`, `usb_bus`) inside handlers MUST `.sort_by_key()` first exactly like this file does — `HashMap` iteration order is not stable.

---

### `src/config/qemu/handlers/root.rs` (dispatch over `RootDeviceKind`)

**Analog:** `src/runtime.rs::format_root_device` (lines ~35-58)

```rust
// Source: src/runtime.rs
fn format_root_device(f: &mut std::fmt::Formatter<'_>, device: &dyn RootDevice) -> std::fmt::Result {
    match device.device_kind() {
        RootDeviceKind::Memory => {
            let memory = device.as_any().downcast_ref::<Memory>().unwrap();
            writeln!(f, "  {}", memory)
        }
        RootDeviceKind::Chipset => {
            let chipset = device.as_any().downcast_ref::<Chipset>().unwrap();
            let rendered = format!("{}", chipset).replace('\n', "\n  ");
            writeln!(f, "  {}", rendered)
        }
        RootDeviceKind::EfiDisk => {
            let efidisk = device.as_any().downcast_ref::<EfiDisk>().unwrap();
            writeln!(f, "  EfiDisk(volume={:?}, logical_size={:?})", efidisk.storage_volume(), efidisk.logical_size())
        }
        _ => writeln!(f, "  {}()", device.get_name()),
    }
}
```
**Core pattern to copy:** `device.device_kind()` → `match` → `device.as_any().downcast_ref::<ConcreteType>().unwrap()` → operate on concrete fields via getters. Every emit handler (root, pcie, pci, sata, ide, usb) follows this exact three-step shape. Replace the `writeln!` bodies with `builder.push_*(format!("..."))` calls into the correct segment.

---

### `src/config/qemu/handlers/pcie.rs` / `pci.rs` (nested bus dispatch)

**Analog:** `src/runtime/q35.rs::format_pcie_device`, `format_pci_device`, `format_usb_device` (full functions, ~40 lines each)

```rust
// Source: src/runtime/q35.rs
fn format_pcie_device(device: &dyn PcieDevice) -> String {
    match device.device_kind() {
        PcieBusDeviceKind::PvScsi => {
            let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
            format!("{}", pvscsi)
        }
        PcieBusDeviceKind::VirtioNet => {
            let virtio_net = device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
            format!("VirtioNetPcie(resource={:?}, mac_address={:?}, ...)", ...)
        }
        PcieBusDeviceKind::HostPci => { /* downcast + getters */ }
        PcieBusDeviceKind::Ivshmem => { /* downcast + getters */ }
    }
}
```
**Core pattern to copy:** Same `device_kind()` → `match` → `downcast_ref::<T>().unwrap()` shape, but recurses: `PvScsi` handler must additionally iterate+sort `pvscsi.scsi_bus()` (see `src/runtime/devices/pvscsi.rs::Display`, lines ~76-90, which already sorts by `(target, lun)` before rendering) — mirror that recursion into the emitter's `emit_pvscsi` handler.

**Recursion analog** (`src/runtime/devices/pvscsi.rs`):
```rust
impl std::fmt::Display for PvScsi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  +-scsi:")?;
        let mut entries: Vec<_> = self.scsi_bus.iter().collect();
        entries.sort_by_key(|(address, _)| (*address.target(), *address.lun()));
        for (address, device) in entries {
            writeln!(f, "      +-{}: {}", address, format_storage_device(device.as_ref()))?;
        }
        Ok(())
    }
}
```

---

### `src/config/qemu/handlers/storage.rs` (drive+device pair emission across scsi/sata/ide)

**Analog:** `src/runtime/q35.rs::format_storage_device` (dispatch by `storage_options().device_type`) + `src/config/proxmox/importer.rs::classify_disk_scsi_resolved`/`classify_disk_sata_resolved`/`classify_disk_ide_resolved` (not shown above but same file, used for the inverse direction — disk-type classification)

```rust
// Source: src/runtime/q35.rs
fn format_storage_device(device: &dyn SataDevice) -> &'static str {
    match device.storage_options().device_type {
        StorageDeviceType::Ssd => "Ssd",
        StorageDeviceType::Hdd => "Hdd",
        StorageDeviceType::Odd => "Cdrom",
    }
}
```
**Core pattern to copy:** dispatch on `storage_options().device_type` (`StorageDeviceType::{Ssd,Hdd,Odd}`), not on a device-kind enum, for storage devices across all three buses (sata/ide/scsi) — a shared function keyed on `StorageDeviceType` avoids duplicating the drive+device emission logic per bus, exactly as this function is shared already for `Display`.

---

### `src/runtime.rs` (add `boot_order: Vec<String>` field, D-07)

**Analog:** existing `RuntimeBuilder` field/method pattern (already in file, lines ~100-145)

```rust
// Source: src/runtime.rs
pub struct RuntimeBuilder {
    root_devices: Vec<Arc<dyn RootDevice>>,
}
impl RuntimeBuilder {
    pub fn with_memory(mut self, memory: Memory) -> Self {
        self.root_devices.push(Arc::new(memory));
        self
    }
    // ... one with_* per existing root-device kind
}
```
**Core pattern to copy:** Add `boot_order: Vec<String>` alongside `root_devices` in both `Runtime` and `RuntimeBuilder` (also add to `#[derive(Getters)]` struct so `.boot_order()` exists), plus a builder method `with_boot_order(mut self, order: Vec<String>) -> Self` following the exact `with_*` consuming-builder shape shown above.

---

### `src/config/proxmox/importer.rs` (parse `order=a;b;c` into `boot_order`, D-07)

**Analog:** same file's existing per-field parse-then-builder-call idiom (`hostpci` loop, lines ~76-91)

```rust
// Source: src/config/proxmox/importer.rs
for (idx, pci_conf) in &self.vm_conf.hostpci {
    let raw_bdf = &pci_conf.bdf;
    let base_bdf = if let Some(pos) = raw_bdf.rfind('.') {
        raw_bdf[..pos].to_string()
    } else {
        raw_bdf.clone()
    };
    let pcie = pci_conf.options.get("pcie").map(|v| v == "1").unwrap_or(false);
    // ... build typed struct, then:
    chipset_builder = chipset_builder.with_host_pci(*idx, Arc::new(host_pci));
}
```
**Core pattern to copy:** Read `self.vm_conf.boot` (`Option<String>`), strip `"order="` prefix, `.split(';')`, `.map(str::to_string).collect()`, then `builder = builder.with_boot_order(order_list)` — same "parse raw Proxmox string field → typed Vec/struct → builder method call" shape as every other field in this function (memory, hostpci, efidisk, tpmstate all follow this).

---

### Error type — `QemuConversionError` (keep thiserror convention, D-06)

**Analog:** `src/config/proxmox/error.rs` (`ProxmoxImportError`, full file, 33 lines)

```rust
// Source: src/config/proxmox/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxImportError {
    #[error("vm_conf has no memory field")]
    MissingMemory,
    #[error("machine type '{machine}' is not supported (only q35 variants)")]
    UnsupportedMachine { machine: String },
    // ...
}
```
**Core pattern to copy:** `#[derive(Debug, thiserror::Error)]` enum, `#[error("...")]` per variant with named fields interpolated in the message. Per D-06, `QemuConversionError` variants must be reserved for genuine Runtime-data conversion problems (e.g. malformed device data), NOT for missing `QemuContext` fields — those use `.expect("clear message")`/`panic!` instead, mirroring nowhere in the codebase yet but consistent with Rust idiom already used for `unreachable!()` in the existing `qemu.rs` stub (`.map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))`).

---

### `tests/qemu_cmdline.rs` (new — golden-fixture + synthetic tests, D-04/D-05)

**Analog:** `tests/proxmox_import.rs` (felucia fixture loading) + `tests/runtime_phase2.rs` (synthetic `Runtime`/device construction via builders)

```rust
// Source: tests/proxmox_import.rs
fn load_felucia_108() -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let conf_str = std::fs::read_to_string("input/felucia/108.conf")
        .expect("input/felucia/108.conf not found");
    let storage_str = std::fs::read_to_string("input/felucia/storage.cfg")
        .expect("input/felucia/storage.cfg not found");
    let vm_conf = ProxmoxVmConf::from_str(&conf_str).expect("parse 108.conf");
    let storage_conf = ProxmoxStorageConf::from_str(&storage_str).expect("parse storage.cfg");
    (vm_conf, storage_conf)
}

#[test]
fn test_proxmox_import_felucia_108_root_devices() {
    let (vm_conf, storage_conf) = load_felucia_108();
    let runtime = ProxmoxImporter::new(vm_conf, storage_conf, 108)
        .into_runtime()
        .expect("into_runtime");
    assert_eq!(runtime.root_devices().len(), 6);
}
```
```rust
// Source: tests/runtime_phase2.rs — synthetic construction via builders (for device kinds
// ProxmoxImporter doesn't cover, e.g. VirtioNetPcie per Pitfall 7 in RESEARCH.md)
let host_pci = HostPci::new("0000:03:00".to_string(), vec![0, 1], true, true, None, None);
let chipset = Q35ChipsetBuilder::new()
    .with_host_pci(0, Arc::new(host_pci))
    .build();
assert_eq!(chipset.pcie_bus().len(), 2);
```
**Core pattern to copy:** (1) load felucia fixture → `ProxmoxImporter::into_runtime()` → build `QemuCommandLine` via `TryFrom<(Runtime, QemuContext)>` → assert token presence + relative ordering via `.find()`/substring index comparisons on the rendered `Display` string (D-05: structural only, not byte-exact); (2) synthetic per-device tests constructing `Runtime`/`Q35Chipset` directly via builders (`RuntimeBuilder`, `Q35ChipsetBuilder`, `PvScsiBuilder`) for device kinds the felucia fixture doesn't exercise (network, USB — per RESEARCH.md Pitfall 7).

---

## Shared Patterns

### `device_kind()` + `downcast_ref` dispatch (Phase 1 convention)
**Source:** `src/runtime.rs::format_root_device`, `src/runtime/q35.rs::format_pcie_device`/`format_pci_device`/`format_usb_device`, `src/runtime/devices/pvscsi.rs::Display`
**Apply to:** Every emit handler in `src/config/qemu/handlers/*.rs` — root devices, pcie/pci bus devices, usb bus devices, and the storage sub-dispatch.
```rust
match device.device_kind() {
    SomeKind::Variant => {
        let concrete = device.as_any().downcast_ref::<ConcreteType>().unwrap();
        // use concrete's getters
    }
}
```

### Sort-before-render for HashMap bus maps
**Source:** `src/runtime/q35.rs::Display` (`pcie_entries.sort_by_key(|(address, _)| (*address.device(), *address.function()));`), `src/runtime/devices/pvscsi.rs::Display`
**Apply to:** Every handler that iterates `pcie_bus`, `pci_bus`, `sata_bus`, `ide_bus`, `usb_bus`, or `scsi_bus` — always `.iter().collect::<Vec<_>>()` then `.sort_by_key(...)` on the address before emitting, never trust raw `HashMap` iteration order (RESEARCH.md Anti-Pattern, roadmap Pitfall 5/6).

### thiserror typed error enums
**Source:** `src/config/proxmox/error.rs` (`ProxmoxImportError`, `ProxmoxParseError`), existing `src/config/qemu.rs` stub (`QemuConversionError`)
**Apply to:** `QemuConversionError` — extend the existing enum with new variants for genuine Runtime-data conversion problems only (per D-06); do NOT add variants for missing `QemuContext` data.

### Consuming builder pattern (`with_*(mut self, ...) -> Self`)
**Source:** `src/runtime.rs::RuntimeBuilder`, `src/runtime/q35.rs::Q35ChipsetBuilder`, `src/runtime/devices/pvscsi.rs::PvScsiBuilder`
**Apply to:** `QemuCommandLineBuilder`'s per-segment push methods (`push_drive`, `push_device`, etc. — already sketched in RESEARCH.md Pattern 1) and `RuntimeBuilder::with_boot_order`.

### Raw string passthrough (verbatim, never split)
**Source:** `src/runtime/rawargs.rs` (doc comment: "never split, tokenize, or reorder it")
**Apply to:** `RawArgs` handler in `handlers/root.rs` — `misc.push(raw_args.0.clone())`, single push, no `.split()`/`.split_whitespace()` anywhere near it (roadmap Pitfall 5/RESEARCH.md Pitfall 5).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/config/qemu/bootindex.rs` (device-id reconstruction, e.g. `ScsiAddress{target,lun}` → `"scsi{target}"`) | utility | transform | No existing code reconstructs Proxmox-style label strings from Runtime bus addresses in the reverse direction (importer only goes label→address, never address→label). RESEARCH.md's Critical Gap section supplies the mapping table (`scsi{target}`, `ide{channel*2+device}`, `net{ordinal}`, `sata{port}` [ASSUMED]) — planner should treat that table as the primary source, not a codebase analog. |
| `QemuContext` struct itself (vm_name, socket_paths, storage_paths) | config/model | n/a | No existing "caller-supplied context" struct exists in the codebase; closest conceptual sibling is `ProxmoxImporter`'s own `vmid: u32` field (external identity data passed alongside parsed config), but there's no direct structural analog — build from D-06's plain description. |

## Metadata

**Analog search scope:** `src/runtime.rs`, `src/runtime/*.rs`, `src/runtime/devices/*.rs`, `src/config/qemu.rs`, `src/config/proxmox/*.rs`, `tests/*.rs`
**Files scanned:** 20 (all `src/runtime` module files listed, `src/config/qemu.rs`, `src/config/proxmox/{error,importer}.rs`, `tests/{runtime_phase2,proxmox_import}.rs`)
**Pattern extraction date:** 2026-07-24
