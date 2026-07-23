---
phase: 01-foundation
plan: 01
type: execute
wave: 1-4
depends_on: []
files_modified:
  - Cargo.toml
  - src/config/proxmox.rs
  - src/config/qemu.rs
  - src/runtime.rs
  - src/runtime/pci.rs
  - src/runtime/pcie.rs
  - src/runtime/usb.rs
  - src/runtime/sata.rs
  - src/runtime/scsi.rs
  - src/runtime/storage.rs
  - src/runtime/memory.rs
  - src/runtime/chipset.rs
  - src/runtime/devices/pvscsi.rs
  - src/runtime/devices/pcie_net.rs
  - src/runtime/devices/pci_generic.rs
  - src/runtime/devices/usb_generic.rs
  - src/runtime/q35.rs
  - src/config/ezkvm/runtime/parser.rs
autonomous: true
requirements:
  - RUNT-08
  - RUNT-09

must_haves:
  truths:
    - All four `type Error = ()` stubs in proxmox.rs and qemu.rs are gone; each TryFrom impl names a typed thiserror enum
    - RuntimeBuilder.root_devices is a plain Vec<Arc<dyn RootDevice>>; Mutex and its import are deleted; with_memory/with_chipset use `mut self`; build() calls Ok(Runtime { root_devices: self.root_devices })
    - Every bus trait (PciDevice, PcieDevice, UsbDevice, SataDevice, ScsiDevice, IsaDevice, RootDevice) has fn device_kind() returning a typed enum; every existing concrete implementor compiles
    - The 12 storage dispatch downcasts use match device_kind() / storage_options().device_type; the 19 non-storage downcasts use match device.device_kind() + safe .unwrap() within match arms
    - StorageDeviceType derives Clone + Copy so device_kind() can return it by value from default impls
  artifacts:
    - Cargo.toml: thiserror = "2" under [dependencies]
    - src/config/proxmox.rs: ProxmoxConversionError enum with #[derive(Debug, thiserror::Error)]
    - src/config/qemu.rs: QemuConversionError enum with #[derive(Debug, thiserror::Error)]
    - src/runtime/pci.rs: PciBusDeviceKind enum (NOT PciDeviceKind — that already exists)
    - src/runtime/pcie.rs: PcieBusDeviceKind enum (NOT PcieDeviceKind)
    - src/runtime/usb.rs: UsbBusDeviceKind enum (NOT UsbDeviceKind — that already exists)
    - src/runtime/isa.rs: IsaBusDeviceKind enum
    - src/runtime.rs: RootDeviceKind enum; re-exports PciBusDeviceKind, PcieBusDeviceKind, UsbBusDeviceKind, IsaBusDeviceKind, RootDeviceKind
  key_links:
    - PciBusDeviceKind / PcieBusDeviceKind / UsbBusDeviceKind MUST NOT collide with the existing PciDeviceKind / UsbDeviceKind in pci_generic.rs / usb_generic.rs — those are per-device sub-kind enums; the new ones are bus-dispatch enums
    - StorageDeviceType::Odd MUST map to the string "Cdrom" in every migrated display site — never "Odd"
    - RuntimeBuilder::build() keeps return type Result<Runtime, ()> so call-sites in proxmox.rs, qemu.rs, and tests are unchanged
    - ProxmoxSchemaHandler and QemuSchemaHandler type aliases must update to use the typed error so the compiler enforces consistency across builder methods and TryFrom impls
---

<objective>
Phase 1 establishes the two foundational surfaces every later conversion phase depends on:
typed error propagation (thiserror enums) and exhaustive device dispatch (device_kind() on
every bus trait).  It also removes the spurious Mutex from RuntimeBuilder.

Purpose: After this phase all downstream TryFrom impls can return named errors, all device
dispatch code uses match instead of downcast_ref chains, and the builder is panic-free.

Output: 5 execution plans across 4 waves.  Wave 1 is a tracer that proves thiserror compiles
end-to-end.  Waves 2-4 complete the typed errors, Mutex removal, trait additions, and
downcast migration in parallel where files do not overlap.
</objective>

<execution_context>
@.github/gsd-core/workflows/execute-plan.md
@.github/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/01-foundation/RESEARCH.md
@.planning/phases/01-foundation/PATTERNS.md
</context>

<!-- ===================================================================
     WAVE MAP (read this first before executing any plan)
     ===================================================================
     Wave 1:  Plan 01-01  (tracer — thiserror dep + ProxmoxConversionError)
     Wave 2:  Plan 01-02  (QemuConversionError)     — parallel with —
              Plan 01-03  (RuntimeBuilder Mutex removal)
     Wave 3:  Plan 01-04  (device_kind() on all traits + all concrete impls)
     Wave 4:  Plan 01-05  (migrate all 31 downcast_ref() sites)

     File overlap check (same-wave plans must not share files):
       01-02 touches: src/config/qemu.rs
       01-03 touches: src/runtime.rs
       → no overlap ✓
     =================================================================== -->

<tasks>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 01-01 · Wave 1 · TRACER
     Thinnest slice that proves thiserror works end-to-end
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 01-01 — Tracer: add thiserror + ProxmoxConversionError

```
wave: 1
depends_on: []
files_modified: [Cargo.toml, src/config/proxmox.rs]
autonomous: true
```

**Rationale:** Proves thiserror resolves and the macro compiles inside the real codebase in
a single `cargo check`.  Picks proxmox.rs as the tracer site because it has both TryFrom
impls in one file and no other plan touches it.

<task type="tracer">
  <name>01-01-T1: Wire thiserror dependency + ProxmoxConversionError end-to-end</name>
  <files>Cargo.toml, src/config/proxmox.rs</files>
  <action>
**Cargo.toml** — under `[dependencies]` add exactly:
  thiserror = "2"

**src/config/proxmox.rs** — make these four changes in order:

1. Add the error enum immediately after the existing `use` imports, before
   `ProxmoxVmSchema`:

   ```
   #[derive(Debug, thiserror::Error)]
   pub enum ProxmoxConversionError {
       #[error("no proxmox handler for device '{name}'")]
       NoHandler { name: String },
   }
   ```

2. Change the `ProxmoxSchemaHandler` type alias from `Result<(), ()>` to
   `Result<(), ProxmoxConversionError>`:

   ```
   type ProxmoxSchemaHandler =
       fn(&mut ProxmoxSchemaBuilder, &dyn RootDevice) -> Result<(), ProxmoxConversionError>;
   ```

3. Change `ProxmoxSchemaBuilder::with_device` return type and body:
   - Return type: `Result<(), ProxmoxConversionError>`
   - Replace the `println! + Err(())` branch with:
     `Err(ProxmoxConversionError::NoHandler { name: device.get_name().to_string() })`
   - Handler call site: `handler(self, device)` — no mapping needed (type now matches)

4. Change `ProxmoxSchemaBuilder::build` return type:
   `pub fn build(self) -> Result<ProxmoxVmSchema, ProxmoxConversionError>`
   Body stays `Ok(self.schema)`.

5. Both `TryFrom` impls: replace `type Error = ();` with
   `type Error = ProxmoxConversionError;`
   The `?` operators propagate automatically because the return types now unify.

**Risk:** `ProxmoxSchemaHandler` is a bare function-pointer type alias — it has no
implementors yet (the handlers HashMap is built with `HashMap::new()` in both TryFrom
impls), so the type change is zero-ripple.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep -c "type Error = ()" src/config/proxmox.rs` returns 0.
    `grep "thiserror" Cargo.toml` returns the version line.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 01-02 · Wave 2 · depends_on: [01-01]
     QemuConversionError — parallel with 01-03
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 01-02 — QemuConversionError typed enum

```
wave: 2
depends_on: [01-01]
files_modified: [src/config/qemu.rs]
autonomous: true
```

<task type="auto">
  <name>01-02-T1: Add QemuConversionError; replace both type Error = () in qemu.rs</name>
  <files>src/config/qemu.rs</files>
  <action>
Mirror the exact pattern established in Plan 01-01 for proxmox.rs:

1. Add error enum after `use` imports, before `QemuSchema`:

   ```
   #[derive(Debug, thiserror::Error)]
   pub enum QemuConversionError {
       #[error("no qemu handler for device '{name}'")]
       NoHandler { name: String },
   }
   ```

2. Change `QemuSchemaHandler` type alias to use `QemuConversionError`:
   `type QemuSchemaHandler = fn(&mut QemuSchemaBuilder, &dyn RootDevice) -> Result<(), QemuConversionError>;`

3. Change `QemuSchemaBuilder::with_device`:
   - Return type: `Result<(), QemuConversionError>`
   - Replace `println! + Err(())` with:
     `Err(QemuConversionError::NoHandler { name: device.get_name().to_string() })`

4. Change `QemuSchemaBuilder::build`:
   `pub fn build(self) -> Result<QemuSchema, QemuConversionError>`
   Body stays `Ok(self.schema)`.

5. Both `TryFrom` impls: `type Error = QemuConversionError;`

**Note:** `build_schema` and `build_runtime` free functions at the bottom of qemu.rs call
`QemuSchema::try_from(runtime)?` and `Runtime::try_from(schema)?` — their return types
`Result<QemuSchema, ()>` and `Result<Runtime, ()>` must become
`Result<QemuSchema, QemuConversionError>` and `Result<Runtime, QemuConversionError>`
respectively.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep -c "type Error = ()" src/config/qemu.rs` returns 0.
    `grep -c "type Error = ()" src/config/proxmox.rs src/config/qemu.rs` returns 0 total.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 01-03 · Wave 2 · depends_on: [01-01]
     RuntimeBuilder Mutex removal — parallel with 01-02
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 01-03 — Remove Mutex from RuntimeBuilder

```
wave: 2
depends_on: [01-01]
files_modified: [src/runtime.rs]
autonomous: true
```

**Model:** `Q35ChipsetBuilder` in `src/runtime/q35.rs` (lines 14–101) is the exact pattern
to follow — plain `HashMap` fields, each `with_*` method is `pub fn with_x(mut self, …) -> Self`
with a direct `.insert()` / `.push()`, and `build(self)` constructs directly.  Apply the
same pattern to `RuntimeBuilder`.

<task type="auto">
  <name>01-03-T1: Replace Mutex&lt;Vec&gt; with plain Vec in RuntimeBuilder; update all methods</name>
  <files>src/runtime.rs</files>
  <action>
Make these four targeted edits to `src/runtime.rs`:

1. **Remove the Mutex import.**
   Delete the line: `use std::sync::Mutex;`
   (The `Arc` import on the same `use` line must stay — split if needed.)

2. **Change the struct field** from:
   `root_devices: Mutex<Vec<Arc<dyn RootDevice>>>`
   to:
   `root_devices: Vec<Arc<dyn RootDevice>>`

3. **Fix RuntimeBuilder::new()** from:
   `root_devices: Mutex::new(Vec::new())`
   to:
   `root_devices: Vec::new()`

4. **Fix build()** — replace the entire body with:
   `Ok(Runtime { root_devices: self.root_devices })`
   (Removes the `into_inner().expect(…)` unwrap — no longer needed and no longer
   compilable without Mutex.)

5. **Fix with_memory()** from the lock pattern to the consuming pattern:
   `pub fn with_memory(mut self, memory: Memory) -> Self {`
   `    self.root_devices.push(Arc::new(memory));`
   `    self`
   `}`

6. **Fix with_chipset()** — identical change:
   `pub fn with_chipset(mut self, chipset: Chipset) -> Self {`
   `    self.root_devices.push(Arc::new(chipset));`
   `    self`
   `}`

Call-site signatures are unchanged: callers still write
`RuntimeBuilder::new().with_memory(m).with_chipset(c).build()` — the consuming-`mut self`
pattern is call-site transparent.

**Risk:** If any future code holds a shared reference to `RuntimeBuilder` across threads,
Mutex removal breaks that.  A grep for `Arc<RuntimeBuilder>` and `&RuntimeBuilder` confirms
no such usage exists today.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep "Mutex" src/runtime.rs` returns no lines.
    `grep "into_inner" src/runtime.rs` returns no lines.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 01-04 · Wave 3 · depends_on: [01-01, 01-02, 01-03]
     Add device_kind() to all bus traits + all concrete implementors
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 01-04 — Add device_kind() to all bus traits and all concrete types

```
wave: 3
depends_on: [01-01, 01-02, 01-03]
files_modified:
  - src/runtime.rs
  - src/runtime/pci.rs
  - src/runtime/pcie.rs
  - src/runtime/usb.rs
  - src/runtime/sata.rs
  - src/runtime/scsi.rs
  - src/runtime/storage.rs
  - src/runtime/memory.rs
  - src/runtime/chipset.rs
  - src/runtime/devices/pvscsi.rs
  - src/runtime/devices/pcie_net.rs
  - src/runtime/devices/pci_generic.rs
  - src/runtime/devices/usb_generic.rs
autonomous: true
```

**Naming constraints (CRITICAL — naming conflicts already exist in codebase):**
- `PciDeviceKind` is taken (pci_generic.rs) — new bus-dispatch enum MUST be `PciBusDeviceKind`
- `UsbDeviceKind` is taken (usb_generic.rs) — new bus-dispatch enum MUST be `UsbBusDeviceKind`
- No existing `PcieBusDeviceKind` or `RootDeviceKind` — use those names freely

<task type="auto">
  <name>01-04-T1: Define bus-kind enums; add device_kind() to PciDevice, PcieDevice, UsbDevice, RootDevice traits; implement on all concrete types</name>
  <files>
    src/runtime/pci.rs,
    src/runtime/pcie.rs,
    src/runtime/usb.rs,
    src/runtime.rs,
    src/runtime/memory.rs,
    src/runtime/chipset.rs,
    src/runtime/devices/pvscsi.rs,
    src/runtime/devices/pcie_net.rs,
    src/runtime/devices/pci_generic.rs,
    src/runtime/devices/usb_generic.rs
  </files>
  <action>
**src/runtime/pci.rs** — add enum + extend trait:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBusDeviceKind {
    GenericPci,
    PvScsi,
}
```
Add to `PciDevice` trait body (after `fn as_any`):
```
fn device_kind(&self) -> PciBusDeviceKind;
```

**src/runtime/pcie.rs** — add enum + extend trait:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcieBusDeviceKind {
    VirtioNet,
    PvScsi,
}
```
Add to `PcieDevice` trait body (after `fn as_any`):
```
fn device_kind(&self) -> PcieBusDeviceKind;
```

**src/runtime/usb.rs** — add enum + extend trait:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbBusDeviceKind {
    Generic,
}
```
Add to `UsbDevice` trait body (after `fn as_any`):
```
fn device_kind(&self) -> UsbBusDeviceKind;
```

**src/runtime.rs** — add RootDeviceKind enum (before RootDevice trait) and extend trait:
```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDeviceKind {
    Memory,
    Chipset,
}
```
Add to `RootDevice` trait body (after `fn get_name`):
```
fn device_kind(&self) -> RootDeviceKind;
```
Update the `pub use devices::{…}` line to also export:
`PciBusDeviceKind` (from pci), `PcieBusDeviceKind` (from pcie), `UsbBusDeviceKind` (from usb).
Update the pub use lines for pci/pcie/usb modules:
```
pub use pci::{PciAddress, PciDevice, PciBusDeviceKind};
pub use pcie::{PcieAddress, PcieDevice, PcieBusDeviceKind};
pub use usb::{UsbAddress, UsbDevice, UsbBusDeviceKind};
```
Add `pub use RootDeviceKind;` (it lives in runtime.rs itself).

**src/runtime/memory.rs** — add `device_kind()` to the `impl RootDevice for Memory` block:
```
fn device_kind(&self) -> crate::runtime::RootDeviceKind {
    crate::runtime::RootDeviceKind::Memory
}
```

**src/runtime/chipset.rs** — add `device_kind()` to the `impl RootDevice for Chipset` block:
```
fn device_kind(&self) -> crate::runtime::RootDeviceKind {
    crate::runtime::RootDeviceKind::Chipset
}
```

**src/runtime/devices/pvscsi.rs** — PvScsi implements both PciDevice and PcieDevice.
In `impl PciDevice for PvScsi`:
```
fn device_kind(&self) -> crate::runtime::PciBusDeviceKind {
    crate::runtime::PciBusDeviceKind::PvScsi
}
```
In `impl PcieDevice for PvScsi`:
```
fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
    crate::runtime::PcieBusDeviceKind::PvScsi
}
```

**src/runtime/devices/pcie_net.rs** — in `impl PcieDevice for VirtioNetPcie`:
```
fn device_kind(&self) -> crate::runtime::PcieBusDeviceKind {
    crate::runtime::PcieBusDeviceKind::VirtioNet
}
```

**src/runtime/devices/pci_generic.rs** — in `impl PciDevice for GenericPciDevice`:
```
fn device_kind(&self) -> crate::runtime::PciBusDeviceKind {
    crate::runtime::PciBusDeviceKind::GenericPci
}
```

**src/runtime/devices/usb_generic.rs** — in `impl UsbDevice for GenericUsbDevice`:
```
fn device_kind(&self) -> crate::runtime::UsbBusDeviceKind {
    crate::runtime::UsbBusDeviceKind::Generic
}
```

**IsaDevice:** Add `device_kind()` to `IsaDevice` in `src/runtime/isa.rs` on par with the other bus traits:

```rust
pub enum IsaBusDeviceKind { PvScsi }

pub trait IsaDevice {
    fn device_kind(&self) -> IsaBusDeviceKind;
    fn as_any(&self) -> &dyn std::any::Any;
}
```

In `src/runtime/devices/pvscsi.rs`, in `impl IsaDevice for PvScsi`:
```rust
fn device_kind(&self) -> crate::runtime::IsaBusDeviceKind {
    crate::runtime::IsaBusDeviceKind::PvScsi
}
```

Re-export `IsaBusDeviceKind` from `src/runtime.rs` alongside the other bus-kind enums.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -10</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep -rn "fn device_kind" src/runtime/pci.rs src/runtime/pcie.rs src/runtime/usb.rs src/runtime/isa.rs src/runtime.rs` shows exactly 5 trait declarations.
    `grep -rn "fn device_kind" src/runtime/devices/ src/runtime/memory.rs src/runtime/chipset.rs` shows exactly 8 implementations (PvScsi gains a second impl for IsaDevice).
  </done>
</task>

<task type="auto">
  <name>01-04-T2: Add device_kind() to SataDevice and ScsiDevice via StorageDeviceType default impl; derive Clone + Copy on StorageDeviceType</name>
  <files>
    src/runtime/storage.rs,
    src/runtime/sata.rs,
    src/runtime/scsi.rs
  </files>
  <action>
**src/runtime/storage.rs** — `StorageDeviceType` currently has no derives.  Add
`Clone, Copy, PartialEq, Eq, Debug` so that the trait method can return it by value:
```
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDeviceType {
    Hdd,
    Ssd,
    Odd,
}
```
No other changes to storage.rs.

**src/runtime/sata.rs** — add a default implementation of `device_kind()` to the `SataDevice`
trait.  `SataDevice: StorageDevice` means `storage_options()` is already in scope:
```
fn device_kind(&self) -> StorageDeviceType {
    self.storage_options().device_type
}
```
Add the `StorageDeviceType` import at the top of the file:
`use crate::runtime::StorageDeviceType;`
The trait implementors (`Ssd`, `Hdd`, `Cdrom`) do NOT need to override this method — the
default delegation is sufficient.

**src/runtime/scsi.rs** — identical change:
```
use crate::runtime::StorageDeviceType;
```
Add the same default `device_kind()` body to `ScsiDevice`.

**Note:** `IdeDevice` (ide.rs) currently has NO `as_any()` and NO downcast sites in the
codebase (IDE dispatch uses `storage_options().device_type` already at parser.rs line 211).
Do NOT add `device_kind()` to `IdeDevice` in this phase — it is not needed.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep "#\[derive" src/runtime/storage.rs` includes `Clone, Copy`.
    `grep "fn device_kind" src/runtime/sata.rs src/runtime/scsi.rs` returns 2 lines.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 01-05 · Wave 4 · depends_on: [01-04]
     Migrate all 31 downcast_ref() sites
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 01-05 — Migrate all 31 downcast_ref() sites

```
wave: 4
depends_on: [01-04]
files_modified:
  - src/runtime/q35.rs
  - src/runtime/devices/pvscsi.rs
  - src/runtime.rs
  - src/config/ezkvm/runtime/parser.rs
autonomous: true
```

**Site inventory (31 total):**

| File | Sites | Category |
|------|-------|----------|
| `src/runtime/q35.rs` | 3 | storage display (`format_storage_device`) |
| `src/runtime/q35.rs` | 5 | non-storage display (format_pcie_device×2, format_pci_device×2, format_usb_device×1) |
| `src/runtime/devices/pvscsi.rs` | 3 | storage display (`format_storage_device`) |
| `src/runtime.rs` | 2 | root device display (`format_root_device`) |
| `src/config/ezkvm/runtime/parser.rs` | 6 | storage data-access (sata×3, scsi×3) |
| `src/config/ezkvm/runtime/parser.rs` | 12 | non-storage data-access + tests |

**Migration rule — STORAGE (12 sites):** replace `downcast_ref::<Ssd/Hdd/Cdrom>()` chains
with `match device.storage_options().device_type`.  `StorageDeviceType::Odd` MUST map to
the string literal `"Cdrom"` — not `"Odd"`.

**Migration rule — NON-STORAGE (19 sites):** replace `if let Some(x) = device.as_any().downcast_ref::<X>()` chains with `match device.device_kind()`.  Within each match arm, use `device.as_any().downcast_ref::<X>().unwrap()` to obtain the concrete type for data access — this is a SAFE unwrap because `device_kind()` has already confirmed the type.  This pattern is exhaustiveness-checked (the compiler enforces all arms) and panic-free (unwrap cannot fail).

<task type="auto">
  <name>01-05-T1: Migrate 11 storage display downcasts in q35.rs and pvscsi.rs</name>
  <files>src/runtime/q35.rs, src/runtime/devices/pvscsi.rs</files>
  <action>
**src/runtime/q35.rs** — `format_storage_device` (line ~197):

Change the function signature from `fn format_storage_device(device: &dyn std::any::Any) -> &'static str`
to `fn format_storage_device(device: &dyn SataDevice) -> &'static str`.

Replace the three `downcast_ref` checks with a match:
```
match device.storage_options().device_type {
    StorageDeviceType::Ssd => "Ssd",
    StorageDeviceType::Hdd => "Hdd",
    StorageDeviceType::Odd => "Cdrom",
}
```
Remove the trailing `"UnknownStorageDevice"` fallback — it is unreachable after exhaustive match.

Update the call site in the SATA bus loop (line ~137) from:
`format_storage_device(device.as_ref().as_any())`
to:
`format_storage_device(device.as_ref())`

Add `StorageDeviceType` to the imports at the top of q35.rs.  Remove the `Ssd`, `Hdd`, `Cdrom` imports if they are no longer referenced anywhere else in the file after this change (verify with grep).

**src/runtime/devices/pvscsi.rs** — `format_storage_device` (line ~85):

Same signature change: `fn format_storage_device(device: &dyn ScsiDevice) -> &'static str`.

Replace the three `downcast_ref` checks with the same `match device.storage_options().device_type` pattern (same three arms, same "Cdrom" spelling for Odd).

Update call site in the scsi_bus loop (line ~79) from:
`format_storage_device(device.as_ref().as_any())`
to:
`format_storage_device(device.as_ref())`

Add `StorageDeviceType` to the pvscsi.rs imports.  Remove `Ssd`, `Hdd`, `Cdrom` imports if no longer used.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep -c "downcast_ref" src/runtime/q35.rs` returns 5 (the 5 non-storage sites — untouched by this task).
    `grep -c "downcast_ref" src/runtime/devices/pvscsi.rs` returns 0.
  </done>
</task>

<task type="auto">
  <name>01-05-T2: Migrate 5 non-storage display downcasts in q35.rs and 2 in runtime.rs</name>
  <files>src/runtime/q35.rs, src/runtime.rs</files>
  <action>
**src/runtime/q35.rs — format_pcie_device (line ~158):**

Replace the `if let Some(pvscsi) = device.as_any().downcast_ref::<PvScsi>()` / `if let Some(virtio_net) = device.as_any().downcast_ref::<VirtioNetPcie>()` chain with:
```
match device.device_kind() {
    PcieBusDeviceKind::PvScsi => {
        let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
        format!("{}", pvscsi)
    }
    PcieBusDeviceKind::VirtioNet => {
        let virtio_net = device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
        format!(
            "VirtioNetPcie(resource={:?}, mac_address={:?}, rx_queue_size={:?}, tx_queue_size={:?}, vhost={:?})",
            virtio_net.resource(), virtio_net.mac_address(),
            virtio_net.rx_queue_size(), virtio_net.tx_queue_size(), virtio_net.vhost(),
        )
    }
}
```
Remove the trailing `format!("{:?}", device)` fallback — it is unreachable after exhaustive match.

**src/runtime/q35.rs — format_pci_device (line ~177):**

Replace the two `downcast_ref` checks with:
```
match device.device_kind() {
    PciBusDeviceKind::GenericPci => {
        let generic = device.as_any().downcast_ref::<GenericPciDevice>().unwrap();
        format!("GenericPciDevice({:?})", generic.kind())
    }
    PciBusDeviceKind::PvScsi => {
        let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
        format!("{}", pvscsi)
    }
}
```

**src/runtime/q35.rs — format_usb_device (line ~189):**

Replace with:
```
match device.device_kind() {
    UsbBusDeviceKind::Generic => {
        let generic = device.as_any().downcast_ref::<GenericUsbDevice>().unwrap();
        format!("GenericUsbDevice({:?})", generic.kind())
    }
}
```

Add `PcieBusDeviceKind, PciBusDeviceKind, UsbBusDeviceKind` to the imports in q35.rs
(they re-export from `crate::runtime`).

**src/runtime.rs — format_root_device (line ~47):**

Replace:
```
if let Some(memory) = device.as_any().downcast_ref::<Memory>() {
    return writeln!(f, "  {}", memory);
}
if let Some(chipset) = device.as_any().downcast_ref::<Chipset>() {
    …
}
writeln!(f, "  {}: {:?}", device.get_name(), device)
```
With:
```
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
}
```
The trailing `writeln!(f, "  {}: {:?}", …)` fallback is unreachable and is removed.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep -c "downcast_ref" src/runtime/q35.rs` returns 0.
    `grep -c "downcast_ref" src/runtime.rs` returns 0.
  </done>
</task>

<task type="auto">
  <name>01-05-T3: Migrate all 18 downcast_ref() sites in parser.rs (6 storage + 12 non-storage)</name>
  <files>src/config/ezkvm/runtime/parser.rs</files>
  <action>
**Storage sites (6) — replace with storage_options().device_type match:**

parser.rs currently has IDE dispatch already using `storage_options().device_type` at
~line 211 — that is the exact pattern to replicate for SATA and SCSI.

SATA dispatch (~lines 160–168) — replace `if / else if / else if downcast_ref` chain with:
```
let sata_type = match sata_device.storage_options().device_type {
    StorageDeviceType::Hdd => SataDeviceTypeSchema::Hdd { resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()) },
    StorageDeviceType::Ssd => SataDeviceTypeSchema::Ssd { resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()) },
    StorageDeviceType::Odd => SataDeviceTypeSchema::Cdrom { resource: format!("sata-{}-{}", sata_addr.port(), sata_addr.device()) },
};
```
Remove the `else { return Err(…) }` fallback — exhaustive match replaces it.

SCSI dispatch (~lines 110–118) — same pattern using `scsi_device.storage_options().device_type`:
```
let scsi_type = match scsi_device.storage_options().device_type {
    StorageDeviceType::Hdd => ScsiDeviceTypeSchema::Hdd { … },
    StorageDeviceType::Ssd => ScsiDeviceTypeSchema::Ssd { … },
    StorageDeviceType::Odd => ScsiDeviceTypeSchema::Cdrom { … },
};
```

Add `StorageDeviceType` to the parser.rs runtime imports.
Remove `Ssd`, `Hdd`, `Cdrom` from parser.rs imports if they are no longer used after
both storage conversions and all tests are migrated (verify).

**Non-storage production code sites (6) — lines 34, 39, 98, 137, 190, 237:**

Root device dispatch (~lines 34, 39) — replace `if let Some(mem) = root_device.as_any().downcast_ref::<Memory>()` / `if let Some(chipset_runtime) = root_device.as_any().downcast_ref::<Chipset>()` with:
```
match root_device.device_kind() {
    RootDeviceKind::Memory => {
        let mem = root_device.as_any().downcast_ref::<Memory>().unwrap();
        memory = Some(MemorySchema::new(*mem.size(), None, false));
        continue;
    }
    RootDeviceKind::Chipset => {
        let chipset_runtime = root_device.as_any().downcast_ref::<Chipset>().unwrap();
        chipset = Some(self.parse_chipset(chipset_runtime)?);
        continue;
    }
}
```
Remove the trailing `return Err(format!("unsupported root device…"))` — it is unreachable.

PvScsi PCIe dispatch (~line 98) — replace `if let Some(pvscsi) = pcie_device.as_any().downcast_ref::<PvScsi>()` with:
```
if pcie_device.device_kind() == PcieBusDeviceKind::PvScsi {
    let pvscsi = pcie_device.as_any().downcast_ref::<PvScsi>().unwrap();
    …
    continue;
}
```

VirtioNet PCIe dispatch (~line 137) — replace `if let Some(virtio_net) = pcie_device.as_any().downcast_ref::<VirtioNetPcie>()` with:
```
if pcie_device.device_kind() == PcieBusDeviceKind::VirtioNet {
    let virtio_net = pcie_device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
    …
    continue;
}
```

PCI dispatch (~line 190) — replace `.downcast_ref::<GenericPciDevice>().ok_or_else(…)?` with a device_kind check:
```
if pci_device.device_kind() != PciBusDeviceKind::GenericPci {
    return Err("unsupported q35 pci device in runtime parser".to_string());
}
let generic = pci_device.as_any().downcast_ref::<GenericPciDevice>().unwrap();
```

USB dispatch (~line 237) — same pattern for `GenericUsbDevice`:
```
if usb_device.device_kind() != UsbBusDeviceKind::Generic {
    return Err("unsupported q35 usb device in runtime parser".to_string());
}
let generic = usb_device.as_any().downcast_ref::<GenericUsbDevice>().unwrap();
```

**Test sites (6) — lines 340, 344, 399, 422, 425, 429 (inside #[cfg(test)]):**

These tests assert round-trip correctness by inspecting recovered runtime state.  Replace
each `downcast_ref` check with the device_kind() + safe unwrap pattern.

`root.as_any().downcast_ref::<Memory>()` (lines 340, 422) →
```
if root.device_kind() == RootDeviceKind::Memory {
    let memory = root.as_any().downcast_ref::<Memory>().unwrap();
    …
}
```

`root.as_any().downcast_ref::<Chipset>()` / `root.as_any().downcast_ref::<Chipset>()` →
```
if root.device_kind() == RootDeviceKind::Chipset {
    let chipset = root.as_any().downcast_ref::<Chipset>().unwrap();
    …
}
```

`pcie_dev.as_any().downcast_ref::<crate::runtime::PvScsi>()` (~line 429) →
```
if pcie_dev.device_kind() == PcieBusDeviceKind::PvScsi {
    let pvscsi = pcie_dev.as_any().downcast_ref::<crate::runtime::PvScsi>().unwrap();
    scsi_count += pvscsi.scsi_bus().len();
}
```

Add all newly needed imports to parser.rs:
`RootDeviceKind, PcieBusDeviceKind, PciBusDeviceKind, UsbBusDeviceKind, StorageDeviceType`
from `crate::runtime`.

**Risk:** `parse_chipset` receives `&Chipset` and pattern-matches on `Chipset::Q35(q35)` /
`Chipset::I440FX` internally (~lines 68–95).  It does NOT use `downcast_ref` for that
matching — it uses the existing enum variant syntax.  Do NOT touch `parse_chipset` unless
its signature changes.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo test 2>&amp;1 | tail -20</automated>
  </verify>
  <done>
    `cargo test` exits 0 with all tests passing.
    `grep -c "downcast_ref" src/config/ezkvm/runtime/parser.rs` returns 0.
    `grep -rn "downcast_ref" src/runtime/ src/config/` returns 0 lines total.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Cargo.toml → crates.io | thiserror v2 fetched from registry on next build |
| Runtime dispatch | device_kind() is implemented by each concrete type; a buggy impl returning the wrong kind makes the safe unwrap() panic at runtime |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-01-01 | Tampering | thiserror crate install | medium | mitigate | Pin to `thiserror = "2"` (SemVer major only); verify crate name at crates.io/crates/thiserror before `cargo check` |
| T-01-02 | Repudiation | device_kind() impl correctness | medium | mitigate | Plan 01-05 verifications assert grep returns 0 downcast_ref; cargo test confirms round-trip tests still pass |
| T-01-03 | Denial of Service | safe-unwrap in match arms | low | accept | Unwraps are only reachable when device_kind() guarantees the type; wrong impl would surface immediately in tests |
| T-01-04 | Information Disclosure | error messages expose device names | low | accept | ProxmoxConversionError/QemuConversionError NoHandler variant exposes device names — acceptable for internal diagnostic tool |
</threat_model>

<verification>
Run these checks in order after all five plans complete:

```bash
# 1. Full compile — no warnings about unused imports from removed Mutex
cargo check 2>&1 | grep -E "^error" | wc -l   # must be 0

# 2. All tests pass (includes round-trip tests in parser.rs)
cargo test 2>&1 | tail -5

# 3. No type Error = () anywhere
grep -rn "type Error = ()" src/   # must return nothing

# 4. No Mutex in RuntimeBuilder
grep "Mutex" src/runtime.rs        # must return nothing

# 5. No downcast_ref in runtime dispatch or config conversion files
grep -rn "downcast_ref" src/runtime/ src/config/   # must return nothing

# 6. New kind enums are re-exported
grep "PciBusDeviceKind\|PcieBusDeviceKind\|UsbBusDeviceKind\|RootDeviceKind" src/runtime.rs
```
</verification>

<success_criteria>
1. **RUNT-09 — Typed errors:** `grep -rn "type Error = ()" src/` returns zero lines. Both
   `ProxmoxConversionError` and `QemuConversionError` are `#[derive(Debug, thiserror::Error)]`
   enums with at least one named variant.

2. **RUNT-08 — device_kind() dispatch:** Every bus trait (`RootDevice`, `PciDevice`, `IsaDevice`, 
   `PcieDevice`, `UsbDevice`, `SataDevice`, `ScsiDevice`) declares `fn device_kind(…)`
   returning a typed enum.  All existing concrete types compile.  No `downcast_ref` remains
   in `src/runtime/` or `src/config/`.

3. **Downcast discipline:** `grep -rn "downcast_ref" src/` returns zero lines after Plan 01-05.
   No new `downcast_ref` sites introduced.

4. **RuntimeBuilder:** `grep "Mutex" src/runtime.rs` returns nothing.  `cargo test` passes
   all existing round-trip tests without modification to test call-sites.
</success_criteria>

<output>
Create `.planning/phases/01-foundation/SUMMARY.md` when all five plans are executed and
all verification checks pass.
</output>
