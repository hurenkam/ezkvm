# Phase 1: Foundation — Research

**Researched:** 2025-07-14
**Domain:** Rust trait design, typed error enums, interior-mutability refactor
**Confidence:** HIGH — all findings verified against actual source files in this repo.

---

## Summary

Phase 1 has two independent concerns: (1) replacing four `type Error = ()` stubs with real
`thiserror`-derived error enums, and (2) adding an exhaustiveness-checked `device_kind()`
method to every device trait so downstream conversion phases can `match` on device type
instead of chaining `downcast_ref` calls.  A smaller third concern is removing the spurious
`Mutex` wrapper inside `RuntimeBuilder`.

`thiserror` is **not yet in `Cargo.toml`**; it must be added.  The codebase compiles cleanly
today (`cargo check` exits 0), so every change in this phase should keep that true.

The 54-site downcast figure cited in the brief is higher than what exists today: a full grep
finds **31 `downcast_ref` call-sites** across four files.  Several of those sites can be
eliminated in Phase 1 itself using the `storage_options()` method that already exists on
`StorageDevice`; the rest become "guaranteed" downcasts (preceded by a `device_kind()` check)
rather than speculative ones.

**Primary recommendation:** Add thiserror, define two thin error enums, strip the Mutex, add
`RootDeviceKind`/`PcieKind` enums with `device_kind()` trait methods, and convert the two
`format_storage_device` helpers to call `storage_options().device_type` — this eliminates
~6 redundant downcasts in a single clean sweep.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RUNT-08 | Device trait exposes `device_kind()` to eliminate `downcast_ref()` | §Device-Kind Design + §Downcast Audit |
| RUNT-09 | All conversion errors use typed `thiserror` error enums (no `type Error = ()`) | §thiserror Setup + §TryFrom Stubs |

---

## Research Question Findings

### Q1: thiserror Setup

**Is thiserror in Cargo.toml?**
No. [VERIFIED: Cargo.toml] — `[dependencies]` contains only `derive-getters`, `derive-new`,
`hashlink`, `ordered-float`, `saphyr`, and `serde`. There is no `thiserror` entry.

**Version to add:** `thiserror = "2"` — resolves to 2.0.19 today (`cargo add thiserror --dry-run`).
[VERIFIED: cargo registry] — `thiserror = "2.0.19"` is the current stable release.

**Add to Cargo.toml:**
```toml
thiserror = "2"
```

**Error enum structure for Proxmox and QEMU:**

`src/config/proxmox.rs` has two `TryFrom` impls with `type Error = ()`:
- `TryFrom<(Runtime, ProxmoxHostSchema)> for ProxmoxVmSchema` — converts a runtime + host
  config into a Proxmox VM schema.  Currently the body is essentially a stub (just calls
  `builder.build()` after iterating devices).  The only error path today is
  `with_device()` returning `Err(())`.
- `TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime` — stub; always succeeds after
  calling `RuntimeBuilder::new().build()`.

`src/config/qemu.rs` mirrors the same pattern:
- `TryFrom<Runtime> for QemuSchema`
- `TryFrom<QemuSchema> for Runtime`

For Phase 1 the enums can be thin placeholders — they just need to be real types with
`#[derive(Debug, thiserror::Error)]` so the downstream phases have a stable surface:

```rust
// src/config/proxmox.rs
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxConversionError {
    #[error("no handler registered for device type: {type_name}")]
    UnknownDevice { type_name: &'static str },
}

// src/config/qemu.rs
#[derive(Debug, thiserror::Error)]
pub enum QemuConversionError {
    #[error("no handler registered for device type: {type_name}")]
    UnknownDevice { type_name: &'static str },
}
```

The current `println!` + `Err(())` in `ProxmoxSchemaBuilder::with_device` / 
`QemuSchemaBuilder::with_device` should become `Err(ProxmoxConversionError::UnknownDevice {...})`.

**Contrast with ezkvm config:** `src/config/ezkvm/runtime/parser.rs` already uses
`type Error = String` and `type Error = String` in its `TryFrom` impls — that module is
already healthy and is **not** in scope for RUNT-09.

---

### Q2: device_kind() Design

**Current device trait hierarchy** (verified from source):

| Trait | File | Concrete Implementors |
|-------|------|-----------------------|
| `RootDevice` | `src/runtime.rs` | `Memory`, `Chipset` |
| `PcieDevice` | `src/runtime/pcie.rs` | `PvScsi`, `VirtioNetPcie` |
| `PciDevice` | `src/runtime/pci.rs` | `GenericPciDevice`, `PvScsi` |
| `ScsiDevice` | `src/runtime/scsi.rs` | `Hdd`, `Ssd`, `Cdrom` |
| `SataDevice` | `src/runtime/sata.rs` | `Hdd`, `Ssd`, `Cdrom` |
| `IdeDevice` | `src/runtime/ide.rs` | `Hdd`, `Ssd`, `Cdrom` |
| `UsbDevice` | `src/runtime/usb.rs` | `GenericUsbDevice` |
| `StorageDevice` | `src/runtime/storage.rs` | `Hdd`, `Ssd`, `Cdrom` |

`IdeDevice` does **not** use `as_any()` (no method in trait, no downcast calls in callers) —
it already dispatches via `storage_options().device_type`.  It does **not** need `device_kind()`.

**Naming conflict warning:** `PciDeviceKind` and `UsbDeviceKind` **already exist** as pub
enums in `src/runtime/devices/pci_generic.rs` and `src/runtime/devices/usb_generic.rs`.
These represent sub-kinds of `GenericPciDevice` and `GenericUsbDevice` respectively
(e.g. `NetworkController`, `QxlGpu`).  New trait-level dispatch enums must use different names:

| Trait | Proposed enum name | Variants |
|-------|-------------------|---------|
| `RootDevice` | `RootDeviceKind` | `Memory`, `Chipset` |
| `PcieDevice` | `PcieBusDeviceKind` | `PvScsi`, `VirtioNetPcie`, `Unknown` |
| `PciDevice` | `PciBusDeviceKind` | `GenericPci`, `PvScsi`, `Unknown` |
| `UsbDevice` | `UsbBusDeviceKind` | `Generic`, `Unknown` |
| `ScsiDevice` | — (reuse `StorageDeviceType`) | — |
| `SataDevice` | — (reuse `StorageDeviceType`) | — |

For `ScsiDevice` and `SataDevice`, `StorageDeviceType` (`Hdd`, `Ssd`, `Odd`) from
`src/runtime/storage.rs` already covers what callers want to know.  Since both traits
inherit `StorageDevice`, callers can call `device.storage_options().device_type` instead of
downcasting.  No new enum needed.

**Proposed trait method signatures:**

```rust
// src/runtime.rs — RootDevice trait
pub enum RootDeviceKind { Memory, Chipset }

pub trait RootDevice: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn std::any::Any;  // keep for now
    fn get_name(&self) -> &str;
    fn get_type(&self) -> TypeId { TypeId::of::<Self>() }
    fn device_kind(&self) -> RootDeviceKind;   // NEW
}

// Implementations:
impl RootDevice for Memory {
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::Memory }
    // ...
}
impl RootDevice for Chipset {
    fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::Chipset }
    // ...
}
```

```rust
// src/runtime/pcie.rs — PcieDevice trait
pub enum PcieBusDeviceKind { PvScsi, VirtioNetPcie, Unknown }

pub trait PcieDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;  // keep for now
    fn device_kind(&self) -> PcieBusDeviceKind;  // NEW
}
```

```rust
// src/runtime/pci.rs — PciDevice trait
pub enum PciBusDeviceKind { GenericPci, PvScsi, Unknown }

pub trait PciDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;  // keep for now
    fn device_kind(&self) -> PciBusDeviceKind;  // NEW
}
```

```rust
// src/runtime/usb.rs — UsbDevice trait
pub enum UsbBusDeviceKind { Generic, Unknown }

pub trait UsbDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;  // keep for now
    fn device_kind(&self) -> UsbBusDeviceKind;  // NEW
}
```

**Export note:** New enums must be added to the pub-use list in `src/runtime.rs`:
```rust
pub use pcie::{PcieAddress, PcieDevice, PcieBusDeviceKind};   // add PcieBusDeviceKind
pub use pci::{PciAddress, PciDevice, PciBusDeviceKind};        // add PciBusDeviceKind
pub use usb::{UsbAddress, UsbDevice, UsbBusDeviceKind};        // add UsbBusDeviceKind
// RootDeviceKind defined in runtime.rs itself — already in scope
```

---

### Q3: downcast_ref Audit

**Actual count: 31 sites** (grep of `src/` with `--include="*.rs"`), distributed as:

| File | Count | Pattern |
|------|-------|---------|
| `src/config/ezkvm/runtime/parser.rs` | 18 | Type dispatch (parser) + test assertions |
| `src/runtime/q35.rs` | 8 | Display formatting only |
| `src/runtime/devices/pvscsi.rs` | 3 | Display formatting only |
| `src/runtime.rs` | 2 | Display formatting only |

**By call pattern:**

**Pattern A — Storage type dispatch (6 sites across q35.rs and pvscsi.rs):**
```rust
// q35.rs lines 198-207, pvscsi.rs lines 86-95
fn format_storage_device(device: &dyn std::any::Any) -> &'static str {
    if device.downcast_ref::<Ssd>().is_some() { return "Ssd"; }
    if device.downcast_ref::<Hdd>().is_some() { return "Hdd"; }
    if device.downcast_ref::<Cdrom>().is_some() { return "Cdrom"; }
    "UnknownStorageDevice"
}
```
**Fix in Phase 1:** Change signature to `&dyn StorageDevice` (or `&dyn SataDevice` /
`&dyn ScsiDevice`) and use `device.storage_options().device_type` instead. `StorageDeviceType`
already maps: `Hdd → "Hdd"`, `Ssd → "Ssd"`, `Odd → "Cdrom"`.

Callers pass `device.as_ref().as_any()` today; after the fix they pass `device.as_ref()`.

**Pattern B — RootDevice dispatch in display (2 sites in runtime.rs):**
```rust
// runtime.rs lines 48, 52
fn format_root_device(f: &mut ..., device: &dyn RootDevice) -> ... {
    if let Some(memory) = device.as_any().downcast_ref::<Memory>() { ... }
    if let Some(chipset) = device.as_any().downcast_ref::<Chipset>() { ... }
}
```
**Fix in Phase 1:** Replace with `match device.device_kind()` using `RootDeviceKind`.
After confirming kind, downcast with `.expect("guaranteed by kind check")` where the
concrete type is needed (e.g. for formatting Chipset internals).

**Pattern C — PCIe device dispatch in q35.rs (2 sites, lines 159, 163):**
```rust
fn format_pcie_device(device: &dyn PcieDevice) -> String {
    if let Some(pvscsi) = device.as_any().downcast_ref::<PvScsi>() { ... }
    if let Some(virtio_net) = device.as_any().downcast_ref::<VirtioNetPcie>() { ... }
}
```
**Fix in Phase 1:** Replace with `match device.device_kind()` using `PcieBusDeviceKind`.

**Pattern D — PCI/USB device dispatch in q35.rs (3 sites, lines 178, 182, 190):**
Similar to Pattern C.  Use `PciBusDeviceKind` and `UsbBusDeviceKind`.

**Pattern E — Parser root/pcie dispatch (4 sites, parser.rs lines 34, 39, 98, 137):**
```rust
// parser.rs
if let Some(mem) = root_device.as_any().downcast_ref::<Memory>() { ... }
if let Some(chipset) = root_device.as_any().downcast_ref::<Chipset>() { ... }
if let Some(pvscsi) = pcie_device.as_any().downcast_ref::<PvScsi>() { ... }
if let Some(virtio_net) = pcie_device.as_any().downcast_ref::<VirtioNetPcie>() { ... }
```
These downcasts also **read concrete fields** (e.g. `mem.size()`, `pvscsi.scsi_bus()`).
Phase 1 can convert the dispatch pattern to `match device_kind()` with a subsequent
`.expect("guaranteed by kind")`  downcast — or leave them for a later phase.
The `Chipset` case is already an enum; once confirmed via `device_kind()`, no downcast is
needed at all (just `downcast_ref::<Chipset>().expect(...)` then `match chipset {}`).

**Pattern F — Parser SCSI/SATA storage dispatch (6 sites, parser.rs lines 110, 114, 118, 160, 164, 168):**
```rust
let scsi_type = if scsi_device.as_any().downcast_ref::<Hdd>().is_some() { ... }
```
Since `ScsiDevice: StorageDevice` and `SataDevice: StorageDevice`, these can be replaced
exactly like Pattern A — use `scsi_device.storage_options().device_type`.

**Pattern G — Test assertions (7 sites, parser.rs tests):**
```rust
if let Some(memory) = root.as_any().downcast_ref::<Memory>() { ... }
if let Some(Chipset::Q35(q35)) = root.as_any().downcast_ref::<Chipset>() { ... }
```
Tests asserting round-trip identity.  Leave as-is in Phase 1 — test code is not required
to avoid `downcast_ref`, and these are clearly purposeful type checks.

---

### Q4: RuntimeBuilder Mutex

**Current code** (`src/runtime.rs` lines 82–113) [VERIFIED: source]:

```rust
pub struct RuntimeBuilder {
    root_devices: Mutex<Vec<Arc<dyn RootDevice>>>,
}
impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder { root_devices: Mutex::new(Vec::new()) }
    }
    pub fn build(self) -> Result<Runtime, ()> {
        let root_devices = self.root_devices
            .into_inner()
            .expect("Root devices mutex should not be poisoned");
        Ok(Runtime { root_devices })
    }
    pub fn with_memory(self, memory: Memory) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(memory));
        self
    }
    pub fn with_chipset(self, chipset: Chipset) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(chipset));
        self
    }
}
```

**Why the Mutex is wrong:** `RuntimeBuilder` uses the consuming builder pattern — all methods
take `self` by value and return `Self`.  Mutation is never concurrent; the Mutex provides
interior mutability only to allow `&self` → `&mut Vec` without `mut self`, but since the
methods consume `self` anyway this is unnecessary complexity.  The `into_inner().expect(...)`
in `build()` is a tell: you never need `.into_inner()` on a Mutex you actually need.

**Safe migration path:**
1. Remove `use std::sync::Mutex;` from `runtime.rs` (and `use std::sync::Arc;` stays).
2. Change `root_devices: Mutex<Vec<...>>` → `root_devices: Vec<Arc<dyn RootDevice>>`.
3. Change `new()` to use `Vec::new()` directly.
4. Change builder methods to `mut self` and push directly:
   ```rust
   pub fn with_memory(mut self, memory: Memory) -> Self {
       self.root_devices.push(Arc::new(memory));
       self
   }
   pub fn with_chipset(mut self, chipset: Chipset) -> Self {
       self.root_devices.push(Arc::new(chipset));
       self
   }
   ```
5. Change `build()`:
   ```rust
   pub fn build(self) -> Result<Runtime, ()> {
       Ok(Runtime { root_devices: self.root_devices })
   }
   ```

**Callers:** `src/config/ezkvm/runtime/builder.rs` uses `RuntimeBuilder` but only calls
`with_memory`, `with_chipset`, and `.build()`.  All three are safe after the refactor with
no call-site changes required.  The tests in `parser.rs` also chain these — they compile
without changes.

---

### Q5: TryFrom Stubs

**All `type Error = ()` locations** [VERIFIED: grep]:

| File | Impl | Notes |
|------|------|-------|
| `src/config/proxmox.rs:47` | `TryFrom<(Runtime, ProxmoxHostSchema)> for ProxmoxVmSchema` | handler dispatch, Err(()) on unknown device |
| `src/config/proxmox.rs:63` | `TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime` | stub — always Ok |
| `src/config/qemu.rs:44` | `TryFrom<Runtime> for QemuSchema` | handler dispatch, Err(()) on unknown device |
| `src/config/qemu.rs:59` | `TryFrom<QemuSchema> for Runtime` | stub — always Ok |

**Not in scope for RUNT-09:**
- `src/config/ezkvm/runtime/parser.rs` → `type Error = String` (already typed)
- `src/config/ezkvm/runtime/builder.rs` → `type Error = String` (already typed)

**`with_device` helper:** Both `ProxmoxSchemaBuilder::with_device` and
`QemuSchemaBuilder::with_device` return `Result<(), ()>`.  Their error path today is:
```rust
println!("No qemu handler for device type: {:?}", device_type);
Err(())
```
After RUNT-09, replace with:
```rust
return Err(QemuConversionError::UnknownDevice { type_name: "unknown" });
```
The `type_name` can be populated from `std::any::type_name::<T>()` or from the new
`device_kind()` method's `Debug` output.

**`build()` stubs returning `Result<_, ()>`:** `RuntimeBuilder::build()` itself returns
`Result<Runtime, ()>`.  After the Mutex removal it can return `Result<Runtime, ()>` or be
simplified to just `Runtime`.  Phase 1 can change the return to `Result<Runtime, Infallible>`
or keep `()` since the two proxmox/qemu callers just `.map_err(|_| ...)`.  Recommend keeping
`()` for now and leaving that cleanup to a later phase to minimise diff noise.

---

## Architecture Patterns

### Device Trait Layer — Before vs. After Phase 1

```
BEFORE Phase 1:
  Runtime
    └─ Vec<Arc<dyn RootDevice>>
           ├─ as_any() → &dyn Any  ←── downcast_ref (speculative)
           └─ get_name()

AFTER Phase 1:
  Runtime
    └─ Vec<Arc<dyn RootDevice>>
           ├─ as_any() → &dyn Any  ←── downcast_ref (guaranteed, after kind check)
           ├─ get_name()
           └─ device_kind() → RootDeviceKind  ←── exhaustive match, NEW
```

### Recommended Project Structure (no changes to file layout in Phase 1)

All new enums live in the same file as their trait:
- `RootDeviceKind` in `src/runtime.rs`
- `PcieBusDeviceKind` in `src/runtime/pcie.rs`
- `PciBusDeviceKind` in `src/runtime/pci.rs`
- `UsbBusDeviceKind` in `src/runtime/usb.rs`
- Error enums in `src/config/proxmox.rs` and `src/config/qemu.rs` respectively

### Error Enum Pattern

```rust
// Pattern to use — thiserror 2.x derive style
use thiserror::Error;

#[derive(Debug, Error)]
pub enum QemuConversionError {
    #[error("no QEMU handler registered for device: {device_kind}")]
    UnknownDevice { device_kind: String },
    
    // Reserve space for errors downstream phases will populate:
    #[error("device configuration is invalid: {reason}")]
    InvalidConfiguration { reason: String },
}
```

Variants with `{ named_fields }` are preferred over tuple variants for `thiserror` —
they make the `#[error("...{field}...")]` interpolation self-documenting.

### Anti-Patterns to Avoid

- **Don't add `device_kind()` as a default method returning `Unknown`.**
  If a new implementor forgets to override it, dispatch silently falls through.
  Make it a required method (no default body) so missing impls become compile errors.

- **Don't replace `Mutex<Vec>` with `RwLock<Vec>`.**
  The pattern needed is consuming builder (takes `self`), not shared mutation.  Both
  wrappers are wrong for this use case.

- **Don't change `build() -> Result<Runtime, ()>` to `build() -> Runtime` in Phase 1.**
  Proxmox and QEMU callers both pattern-match on the `Result`.  Leave the signature stable
  until those modules are updated.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Typed error derivation | Manual `impl Display + Error` | `thiserror` | Missing `source()`, `cause()` chains; error on impl divergence |
| Type dispatch without downcasting | Visitor pattern from scratch | `device_kind()` match + `expect()` downcast | Visitor requires adding a method per new consumer; enums are zero-cost and exhaustiveness-checked |
| Storage type identification in formatting | New enum / extra downcast chain | `storage_options().device_type` (already on `StorageDevice`) | Already implemented correctly in `StorageDevice`; duplicating it is dead code |

---

## Common Pitfalls

### Pitfall 1: Name collision with existing `PciDeviceKind` / `UsbDeviceKind`
**What goes wrong:** Adding `pub enum PciDeviceKind` for the PCI bus trait conflicts with
the existing `pub use devices::PciDeviceKind` in `runtime.rs`, which is already a different
enum (`NetworkController | QxlGpu | Ac97`).
**Why it happens:** Both enums represent "kinds of PCI device" but at different abstraction
layers — one is a bus-level type tag, the other is a sub-variant of `GenericPciDevice`.
**How to avoid:** Use the `PciBusDeviceKind` / `UsbBusDeviceKind` naming scheme proposed
in §Q2 above.  Keep the existing `PciDeviceKind` / `UsbDeviceKind` names unchanged.

### Pitfall 2: Forgetting to export new enums in `runtime.rs`
**What goes wrong:** Downstream callers (particularly `config/ezkvm/runtime/parser.rs`)
import from `crate::runtime::*`.  Enums defined in sub-modules but not re-exported via
`pub use` are invisible to callers.
**How to avoid:** After adding each enum, immediately add a `pub use` line in `runtime.rs`.
**Warning signs:** `error[E0412]: cannot find type 'PcieBusDeviceKind'` in caller modules.

### Pitfall 3: Mutex removal breaks `with_memory` / `with_chipset` borrow check
**What goes wrong:** The current signature `pub fn with_memory(self, memory: Memory) -> Self`
takes `self` by value.  If `self.root_devices` is a plain `Vec`, you need `mut self` to push
to it.  Forgetting `mut` gives `error[E0596]: cannot borrow ... as mutable`.
**How to avoid:** Add `mut` to both parameters: `pub fn with_memory(mut self, ...) -> Self`.

### Pitfall 4: `format_storage_device` takes `&dyn Any` — changing its signature breaks two callers
**What goes wrong:** Both `q35.rs` and `pvscsi.rs` call
`format_storage_device(device.as_ref().as_any())`.  Changing the parameter type requires
updating both call sites to pass `device.as_ref()` instead.
**How to avoid:** Update callers in the same commit as the function signature change;
`cargo check` will catch all mismatches.

### Pitfall 5: `StorageDeviceType::Odd` vs `"Cdrom"` string mismatch
**What goes wrong:** `StorageDeviceType` uses `Odd` (optical disc device) but the display
functions return `"Cdrom"`.  When converting `storage_options().device_type` to a display
string, `Odd` must map to `"Cdrom"` to maintain existing output.
**How to avoid:** Use an explicit match:
```rust
match device.storage_options().device_type {
    StorageDeviceType::Hdd => "Hdd",
    StorageDeviceType::Ssd => "Ssd",
    StorageDeviceType::Odd => "Cdrom",
}
```

---

## Recommended Task Ordering

**Wave 1 — Dependencies and builder fix (zero-risk, no trait changes):**
1. Add `thiserror = "2"` to `Cargo.toml`
2. Remove `Mutex` from `RuntimeBuilder` in `src/runtime.rs`
   - Change field type, add `mut` to builder methods, simplify `build()`
   - Verify: `cargo test` passes

**Wave 2 — Error enums (isolated to two files):**
3. Define `ProxmoxConversionError` in `src/config/proxmox.rs`; replace `type Error = ()`
   on both `TryFrom` impls; update `with_device` error path
4. Define `QemuConversionError` in `src/config/qemu.rs`; same treatment
   - Verify: `cargo test` passes

**Wave 3 — Storage downcast elimination (safe, no new APIs):**
5. In `src/runtime/q35.rs` and `src/runtime/devices/pvscsi.rs`, change
   `format_storage_device` to accept `&dyn StorageDevice` (or the appropriate sub-trait)
   and use `storage_options().device_type`; update call sites
   - Verify: `cargo test` passes; display output unchanged

**Wave 4 — device_kind() enums and trait methods:**
6. Define `RootDeviceKind` in `src/runtime.rs`; add `fn device_kind(&self) -> RootDeviceKind`
   to `RootDevice`; implement on `Memory` and `Chipset`; export
7. Define `PcieBusDeviceKind` in `src/runtime/pcie.rs`; add to `PcieDevice`; implement on
   `PvScsi` and `VirtioNetPcie`; export in `runtime.rs`
8. Define `PciBusDeviceKind` in `src/runtime/pci.rs`; add to `PciDevice`; implement on
   `GenericPciDevice` and `PvScsi`; export
9. Define `UsbBusDeviceKind` in `src/runtime/usb.rs`; add to `UsbDevice`; implement on
   `GenericUsbDevice`; export
   - Verify: `cargo test` passes; all trait impls cover every concrete type

**Wave 5 — Convert display formatters to use device_kind():**
10. Update `format_root_device` in `src/runtime.rs` to `match device.device_kind()`
11. Update `format_pcie_device`, `format_pci_device`, `format_usb_device` in
    `src/runtime/q35.rs` to use `device_kind()` — remaining downcasts become
    `.expect("guaranteed by RootDeviceKind match")`
    - Verify: `cargo test` passes; display output unchanged

---

## Open Questions

1. **Should `build() -> Result<Runtime, ()>` change to `build() -> Runtime`?**
   After the Mutex removal there is no failure path.  Changing the return type would
   clean up `.expect("...")` call sites but requires updating `proxmox.rs`, `qemu.rs`,
   and `ezkvm/runtime/builder.rs` in the same PR.  Recommend leaving as `Result<_, ()>`
   in Phase 1 and tracking as a follow-up.

2. **Should `as_any()` be removed from the traits once all display formatters migrate?**
   Tests in `parser.rs` still rely on `downcast_ref` for assertion logic (Pattern G
   above).  As long as tests use it, `as_any()` must stay on the trait.  A later phase
   can add typed accessor helpers (`fn as_pvscsi(&self) -> Option<&PvScsi>`) and remove
   `as_any()`.  Out of scope for Phase 1.

3. **`PvScsi` implements both `PcieDevice` and `PciDevice`.**
   `PciBusDeviceKind::PvScsi` and `PcieBusDeviceKind::PvScsi` will both exist.  This is
   correct — the same concrete type is addressable on both buses.  No issue, just worth
   noting for reviewers.

4. **`StorageDeviceType::Odd` vs a richer storage kind enum.**
   `Odd` means "optical disc device" but the codebase uses the string `"Cdrom"`.  If
   future phases need finer grain (e.g. BluRay vs CD), `StorageDeviceType` would need
   extension.  For Phase 1, treat `Odd == Cdrom` as acceptable.

---

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Verdict | Disposition |
|---------|----------|-----|-----------|---------|-------------|
| thiserror | crates.io | ~6 yrs | Very high (major Rust ecosystem crate) | OK | Approved |

`thiserror` is maintained by David Tolnay (dtolnay), the same author as `serde`, `syn`,
and `quote`.  [VERIFIED: cargo registry via `cargo add thiserror --dry-run`] — resolves to
2.0.19, clean.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / `rustc` | All tasks | ✓ | edition 2024 | — |
| crates.io network | `thiserror` fetch | ✓ (assumed) | — | vendor / offline |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`) |
| Config file | none (standard cargo test) |
| Quick run command | `cargo test` |
| Full suite command | `cargo test --all` |

### Phase Requirements → Test Map

| ID | Behavior | Test Type | Automated Command | Tests Exist? |
|----|----------|-----------|-------------------|-------------|
| RUNT-08 | `device_kind()` returns correct variant for every concrete type | unit | `cargo test` | ❌ Wave 0 |
| RUNT-08 | `format_root_device` / `format_pcie_device` use match not downcast | compilation | `cargo check` | N/A |
| RUNT-09 | `TryFrom` impls on proxmox/qemu use typed error, not `()` | compilation | `cargo check` | N/A |
| RUNT-09 | `Err` paths return the correct typed error variant | unit | `cargo test` | ❌ Wave 0 |
| Mutex removal | `RuntimeBuilder` builds correctly without Mutex | existing round-trip tests | `cargo test` | ✅ parser.rs tests |

### Wave 0 Gaps

- [ ] `src/runtime.rs` — unit test: `Memory::device_kind() == RootDeviceKind::Memory`, `Chipset::device_kind() == RootDeviceKind::Chipset`
- [ ] `src/runtime/pcie.rs` — unit test: `PvScsi::device_kind() == PcieBusDeviceKind::PvScsi`, etc.
- [ ] `src/config/proxmox.rs` — unit test: unknown device returns `ProxmoxConversionError::UnknownDevice`
- [ ] `src/config/qemu.rs` — unit test: unknown device returns `QemuConversionError::UnknownDevice`

---

## Security Domain

No authentication, session management, input from external sources, or cryptography in scope
for this phase.  Changes are internal Rust type system refactors.  ASVS categories V2–V6
do not apply.

---

## Sources

### Primary (HIGH confidence — verified against codebase)
- `src/runtime.rs` — RuntimeBuilder struct, RootDevice trait, as_any/downcast_ref pattern
- `src/runtime/pcie.rs`, `pci.rs`, `scsi.rs`, `sata.rs`, `usb.rs` — trait definitions
- `src/runtime/storage.rs` — StorageDevice/StorageDeviceType (Hdd/Ssd/Odd)
- `src/runtime/devices/pci_generic.rs` — existing PciDeviceKind enum (name conflict source)
- `src/runtime/devices/usb_generic.rs` — existing UsbDeviceKind enum (name conflict source)
- `src/runtime/q35.rs`, `devices/pvscsi.rs` — downcast_ref in display formatters
- `src/config/proxmox.rs`, `src/config/qemu.rs` — all four `type Error = ()` impls
- `src/config/ezkvm/runtime/parser.rs` — 18-site downcast_ref audit + round-trip tests
- `Cargo.toml` — confirmed absence of thiserror

### Secondary (MEDIUM confidence — cargo registry)
- `cargo add thiserror --dry-run` → resolves to `thiserror = "2.0.19"` [VERIFIED: registry]

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — thiserror version verified via cargo registry
- Architecture: HIGH — all findings from direct source inspection
- Pitfalls: HIGH — all pitfalls are mechanical consequences of the code as read

**Research date:** 2025-07-14
**Valid until:** 2025-08-14 (thiserror version may advance; trait design is stable)
