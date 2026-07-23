# Phase 1: Foundation — Pattern Map

**Mapped:** 2025-07-14
**Files analyzed:** 6 primary change sites + 2 trait families + 1 builder
**Analogs found:** 5 / 6 (1 has no codebase analog — `thiserror` is not yet present)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/runtime.rs` — add `device_kind()` to `RootDevice`; remove `Mutex` from `RuntimeBuilder` | trait + builder | request-response | `src/runtime/pci.rs`, `src/runtime/pcie.rs`, `src/runtime/usb.rs` | exact (same trait family) |
| `src/runtime/pci.rs` — add `device_kind()` to `PciDevice` | trait | request-response | `src/runtime/pcie.rs` | exact |
| `src/runtime/pcie.rs` — add `device_kind()` to `PcieDevice` | trait | request-response | `src/runtime/pci.rs` | exact |
| `src/runtime/sata.rs` — add `device_kind()` to `SataDevice` | trait | request-response | `src/runtime/scsi.rs` | exact |
| `src/runtime/scsi.rs` — add `device_kind()` to `ScsiDevice` | trait | request-response | `src/runtime/sata.rs` | exact |
| `src/runtime/usb.rs` — add `device_kind()` to `UsbDevice` | trait | request-response | `src/runtime/pcie.rs` | exact |
| `src/runtime/q35.rs` — replace `downcast_ref()` call sites | formatter utility | transform | itself (existing pattern to replace) | self-analog |
| `src/runtime/devices/pvscsi.rs` — replace `downcast_ref()` call sites | device impl | transform | `src/runtime/q35.rs` | exact |
| `src/runtime/devices/pci_generic.rs` — concrete `device_kind()` impl | device impl | request-response | `src/runtime/devices/usb_generic.rs` | exact |
| `src/runtime/devices/usb_generic.rs` — concrete `device_kind()` impl | device impl | request-response | `src/runtime/devices/pci_generic.rs` | exact |
| `src/runtime/devices/pcie_net.rs` — concrete `device_kind()` impl | device impl | request-response | `src/runtime/devices/pci_generic.rs` | role-match |
| `src/runtime/devices/pvscsi.rs` — concrete `device_kind()` impl | device impl | request-response | `src/runtime/devices/pci_generic.rs` | role-match |
| `src/config/qemu.rs` — `type Error = ()` → typed enum | config converter | request-response | `src/serde_yaml/error.rs` (manual enum) | role-match (no `thiserror` yet) |
| `src/config/proxmox.rs` — `type Error = ()` → typed enum | config converter | request-response | `src/serde_yaml/error.rs` (manual enum) | role-match |
| `src/runtime.rs` — `RuntimeBuilder::build()` error → typed enum | builder | request-response | `src/config/ezkvm/runtime/builder.rs` `Result<Runtime, String>` | partial-match |

---

## Pattern Assignments

---

### 1 · `device_kind()` on device traits (`PciDevice`, `PcieDevice`, `UsbDevice`, `SataDevice`, `ScsiDevice`, `RootDevice`)

**Goal:** Add a `fn device_kind(&self) -> &'static str` (or a `DeviceKind` enum) to every device trait so call sites can identify a device without `downcast_ref()`.

**Analog — current trait shape:** `src/runtime/pci.rs` (lines 7–9)

```rust
// src/runtime/pci.rs  lines 7-9
#[allow(dead_code)]
pub trait PciDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
}
```

All five bus-level device traits follow exactly this shape:

| File | Trait | Lines |
|---|---|---|
| `src/runtime/pci.rs` | `PciDevice` | 7–9 |
| `src/runtime/pcie.rs` | `PcieDevice` | 6–8 |
| `src/runtime/usb.rs` | `UsbDevice` | 6–8 |
| `src/runtime/sata.rs` | `SataDevice` | 8–10 |
| `src/runtime/scsi.rs` | `ScsiDevice` | 8–10 |
| `src/runtime.rs` | `RootDevice` | 74–80 |

**Pattern to add** — model every trait change after this shape:

```rust
// NEW method to add to every bus-level device trait
// Copy this pattern into pci.rs, pcie.rs, usb.rs, sata.rs, scsi.rs, and
// the RootDevice trait in runtime.rs
pub trait PciDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> &'static str;      // ← add this
}
```

**Existing `kind` field precedent** — `GenericPciDevice` and `GenericUsbDevice` already carry a `kind` field (same concept, stored on the concrete type):

```rust
// src/runtime/devices/pci_generic.rs  lines 8-18
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciDeviceKind {
    NetworkController,
    QxlGpu,
    Ac97,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct GenericPciDevice {
    kind: PciDeviceKind,
}
```

```rust
// src/runtime/devices/usb_generic.rs  lines 7-17
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbDeviceKind {
    NetworkController,
    Tablet,
    HostPassthrough { resource: String },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct GenericUsbDevice {
    kind: UsbDeviceKind,
}
```

**Concrete `device_kind()` impl pattern** — for all device structs, implement by returning a string literal matching the struct name:

```rust
// Model every impl after this (adapt the string literal per type)
impl PcieDevice for VirtioNetPcie {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn device_kind(&self) -> &'static str { "VirtioNetPcie" }   // ← add
}

impl PcieDevice for PvScsi {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn device_kind(&self) -> &'static str { "PvScsi" }          // ← add
}

impl PciDevice for GenericPciDevice {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn device_kind(&self) -> &'static str { "GenericPciDevice" } // ← add
}
```

For `RootDevice` impls (`Memory`, `Chipset`) the same pattern applies:

```rust
// src/runtime/memory.rs  lines 18-25 — existing impl, add device_kind()
impl RootDevice for Memory {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "memory" }
    fn device_kind(&self) -> &'static str { "Memory" }           // ← add
}

// src/runtime/chipset.rs  lines 10-17 — existing impl, add device_kind()
impl RootDevice for Chipset {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "chipset" }
    fn device_kind(&self) -> &'static str { "Chipset" }          // ← add
}
```

---

### 2 · Replace `downcast_ref()` call sites

**Goal:** Remove the `downcast_ref` chains in `format_*` helper functions by calling `device_kind()` instead.

**Current call-site pattern — `src/runtime/q35.rs`:**

```rust
// src/runtime/q35.rs  lines 158-175  ← REPLACE this pattern
fn format_pcie_device(device: &dyn PcieDevice) -> String {
    if let Some(pvscsi) = device.as_any().downcast_ref::<PvScsi>() {
        return format!("{}", pvscsi);
    }
    if let Some(virtio_net) = device.as_any().downcast_ref::<VirtioNetPcie>() {
        return format!(
            "VirtioNetPcie(resource={:?}, mac_address={:?}, ...)",
            virtio_net.resource(), ...
        );
    }
    format!("{:?}", device)
}
```

```rust
// src/runtime/q35.rs  lines 177-187  ← REPLACE
fn format_pci_device(device: &dyn PciDevice) -> String {
    if let Some(generic) = device.as_any().downcast_ref::<GenericPciDevice>() {
        return format!("GenericPciDevice({:?})", generic.kind());
    }
    if let Some(pvscsi) = device.as_any().downcast_ref::<PvScsi>() {
        return format!("{}", pvscsi);
    }
    format!("{:?}", device)
}
```

```rust
// src/runtime/q35.rs  lines 189-195  ← REPLACE
fn format_usb_device(device: &dyn UsbDevice) -> String {
    if let Some(generic) = device.as_any().downcast_ref::<GenericUsbDevice>() {
        return format!("GenericUsbDevice({:?})", generic.kind());
    }
    format!("{:?}", device)
}
```

```rust
// src/runtime/q35.rs  lines 197-208  ← REPLACE
fn format_storage_device(device: &dyn std::any::Any) -> &'static str {
    if device.downcast_ref::<Ssd>().is_some() { return "Ssd"; }
    if device.downcast_ref::<Hdd>().is_some() { return "Hdd"; }
    if device.downcast_ref::<Cdrom>().is_some() { return "Cdrom"; }
    "UnknownStorageDevice"
}
```

```rust
// src/runtime/devices/pvscsi.rs  lines 85-96  ← REPLACE (same pattern)
fn format_storage_device(device: &dyn std::any::Any) -> &'static str {
    if device.downcast_ref::<Ssd>().is_some() { return "Ssd"; }
    if device.downcast_ref::<Hdd>().is_some() { return "Hdd"; }
    if device.downcast_ref::<Cdrom>().is_some() { return "Cdrom"; }
    "UnknownStorageDevice"
}
```

Also in `src/runtime.rs` lines 48–55:

```rust
// src/runtime.rs  lines 47-58  ← REPLACE
fn format_root_device(f: &mut std::fmt::Formatter<'_>, device: &dyn RootDevice) -> std::fmt::Result {
    if let Some(memory) = device.as_any().downcast_ref::<Memory>() {
        return writeln!(f, "  {}", memory);
    }
    if let Some(chipset) = device.as_any().downcast_ref::<Chipset>() {
        let rendered = format!("{}", chipset).replace('\n', "\n  ");
        return writeln!(f, "  {}", rendered);
    }
    writeln!(f, "  {}: {:?}", device.get_name(), device)
}
```

**Replacement pattern** — after `device_kind()` exists, collapse every `downcast_ref` chain to a `match`:

```rust
// NEW shape for format_pcie_device after device_kind() is added
fn format_pcie_device(device: &dyn PcieDevice) -> String {
    match device.device_kind() {
        "PvScsi"        => format!("{}", device.as_any().downcast_ref::<PvScsi>().unwrap()),
        "VirtioNetPcie" => format!("{:?}", device.as_any().downcast_ref::<VirtioNetPcie>().unwrap()),
        _               => format!("{:?}", device),
    }
}

// NEW shape for format_storage_device — no more downcast, just use device_kind()
fn format_storage_device(device: &dyn SataDevice) -> &'static str {
    device.device_kind()
}
```

> **Note:** for storage devices (`Ssd`, `Hdd`, `Cdrom`) the `device_kind()` return value **is** the display string, so `format_storage_device` collapses to a one-liner.

---

### 3 · `type Error = ()` → typed error enum in `qemu.rs` and `proxmox.rs`

**Goal:** Replace `type Error = ()` with a named enum that carries a message.

**Current stub pattern — `src/config/qemu.rs`:**

```rust
// src/config/qemu.rs  lines 43-56
impl TryFrom<Runtime> for QemuSchema {
    type Error = ();

    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        let handlers = HashMap::new();
        let mut builder = QemuSchemaBuilder::new(handlers);
        for device in value.root_devices() {
            builder.with_device(device.as_ref())?;
        }
        builder.build()
    }
}

impl TryFrom<QemuSchema> for Runtime {
    type Error = ();

    fn try_from(_schema: QemuSchema) -> Result<Self, Self::Error> {
        let builder = RuntimeBuilder::new();
        builder.build()
    }
}
```

```rust
// src/config/proxmox.rs  lines 46-71  (same shape, two impls)
impl TryFrom<(Runtime, ProxmoxHostSchema)> for ProxmoxVmSchema {
    type Error = ();
    ...
}
impl TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime {
    type Error = ();
    ...
}
```

**Best analog for typed errors — `src/serde_yaml/error.rs`** (manual `std::error::Error` enum — no `thiserror`):

```rust
// src/serde_yaml/error.rs  lines 1-37
#[derive(Debug, Clone)]
pub enum Error {
    Parse(String),
    Message(String),
    InvalidType { expected: String, got: String },
    MissingField(String),
    UnknownField(String),
    UnexpectedEnd,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg)         => write!(f, "YAML parse error: {}", msg),
            Error::Message(msg)       => write!(f, "{}", msg),
            Error::InvalidType { expected, got }
                                      => write!(f, "invalid type: got {}, expected {}", got, expected),
            Error::MissingField(f_)   => write!(f, "missing field '{}'", f_),
            Error::UnknownField(f_)   => write!(f, "unknown field '{}'", f_),
            Error::UnexpectedEnd      => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for Error {}
```

**Closer runtime analog — `src/config/ezkvm/runtime/builder.rs` line 36** already uses `Result<Runtime, String>` as a typed error without a custom enum:

```rust
// src/config/ezkvm/runtime/builder.rs  line 36
pub fn build(&self) -> Result<Runtime, String> {
```

**Recommended pattern for `qemu.rs` and `proxmox.rs`** — introduce a small domain-specific error enum modelled on `serde_yaml/error.rs` but simpler, since `thiserror` is not in `Cargo.toml`:

```rust
// NEW — add above the TryFrom impls in qemu.rs  (same pattern for proxmox.rs)
#[derive(Debug, Clone)]
pub enum QemuConversionError {
    /// No handler registered for a device type.
    UnhandledDevice(String),
    /// Builder failed to produce a valid schema/runtime.
    BuildFailed(String),
}

impl std::fmt::Display for QemuConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QemuConversionError::UnhandledDevice(msg) => write!(f, "unhandled device: {}", msg),
            QemuConversionError::BuildFailed(msg)     => write!(f, "build failed: {}", msg),
        }
    }
}

impl std::error::Error for QemuConversionError {}

// THEN replace type Error = () with:
impl TryFrom<Runtime> for QemuSchema {
    type Error = QemuConversionError;
    fn try_from(value: Runtime) -> Result<Self, Self::Error> {
        ...
    }
}
```

> **If `thiserror` is added to `Cargo.toml`** the same enum can be written more concisely:
> ```rust
> #[derive(Debug, thiserror::Error)]
> pub enum QemuConversionError {
>     #[error("unhandled device: {0}")]
>     UnhandledDevice(String),
>     #[error("build failed: {0}")]
>     BuildFailed(String),
> }
> ```
> This is the preferred Phase 1 target. Add `thiserror = "1"` to `[dependencies]` in `Cargo.toml`.

---

### 4 · Remove `Mutex` from `RuntimeBuilder`

**Goal:** `RuntimeBuilder` currently wraps `root_devices` in `Mutex<Vec<...>>` even though `RuntimeBuilder` is never shared across threads. The builder-pattern `with_*` methods consume `self` and return `Self`, so no shared state is needed.

**Current broken pattern — `src/runtime.rs` lines 83–111:**

```rust
// src/runtime.rs  lines 83-111  ← CURRENT (to remove Mutex from)
#[allow(dead_code)]
pub struct RuntimeBuilder {
    root_devices: Mutex<Vec<Arc<dyn RootDevice>>>,   // ← unnecessary
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder {
            root_devices: Mutex::new(Vec::new()),
        }
    }

    pub fn build(self) -> Result<Runtime, ()> {
        let root_devices = self
            .root_devices
            .into_inner()
            .expect("Root devices mutex should not be poisoned");
        Ok(Runtime { root_devices })
    }

    pub fn with_memory(self, memory: Memory) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(memory));   // ← broken: consumes self but then mutates through lock
        self
    }

    pub fn with_chipset(self, chipset: Chipset) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(chipset));  // ← same issue
        self
    }
}
```

**Analog for correct builder pattern — `src/runtime/q35.rs` `Q35ChipsetBuilder`** (lines 22–101):

```rust
// src/runtime/q35.rs  lines 22-101  ← MODEL for RuntimeBuilder
pub struct Q35ChipsetBuilder {
    pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>,
    pci_bus:  HashMap<PciAddress,  Arc<dyn PciDevice>>,
    ...
}

impl Q35ChipsetBuilder {
    pub fn new() -> Self {
        Self {
            pcie_bus: HashMap::new(),
            pci_bus:  HashMap::new(),
            ...
        }
    }

    // ← takes mut self, pushes, returns Self — no Mutex needed
    pub fn with_pcie_device(
        mut self,
        address: Option<PcieAddress>,
        device: Arc<dyn PcieDevice>,
    ) -> Self {
        let address = address.unwrap_or_else(|| PcieAddress::new(0, 0));
        self.pcie_bus.insert(address, device);
        self                                   // ← returns owned self
    }

    pub fn build(self) -> Q35Chipset {
        Q35Chipset::new(self.pcie_bus, self.pci_bus, ...)
    }
}
```

**Target pattern for `RuntimeBuilder`:**

```rust
// REPLACE the Mutex-based RuntimeBuilder with this shape
#[allow(dead_code)]
pub struct RuntimeBuilder {
    root_devices: Vec<Arc<dyn RootDevice>>,   // ← plain Vec
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        RuntimeBuilder { root_devices: Vec::new() }
    }

    pub fn build(self) -> Result<Runtime, ()> {
        Ok(Runtime { root_devices: self.root_devices })
    }

    pub fn with_memory(mut self, memory: Memory) -> Self {
        self.root_devices.push(Arc::new(memory));
        self
    }

    pub fn with_chipset(mut self, chipset: Chipset) -> Self {
        self.root_devices.push(Arc::new(chipset));
        self
    }
}
```

Also remove `use std::sync::Mutex;` from `src/runtime.rs` line 28 once the change is applied.

---

## Shared Patterns

### Builder consuming-`mut self` idiom
**Source:** `src/runtime/q35.rs` `Q35ChipsetBuilder` (lines 32–101), also `src/runtime/devices/pvscsi.rs` `PvScsiBuilder` (lines 22–46)
**Apply to:** `RuntimeBuilder` refactor, and any future builder in `src/runtime/`

```rust
// The idiom: take ownership, mutate, return self
pub fn with_X(mut self, x: X) -> Self {
    self.collection.insert(key, x);
    self
}
```

### Trait bounds on device traits
**Source:** `src/runtime/pci.rs` (line 7), `src/runtime/pcie.rs` (line 6), `src/runtime/sata.rs` (line 8)
**Apply to:** Every new method added to bus-level traits — do NOT relax `Debug + Sync + Send + 'static`

```rust
pub trait XxxDevice: Debug + Sync + Send + 'static {
    fn as_any(&self) -> &dyn std::any::Any;
    fn device_kind(&self) -> &'static str;
}
```

### `#[allow(dead_code)]` placement
**Source:** Every trait definition and struct in `src/runtime/` (e.g. `pci.rs` line 6, `pcie.rs` line 5, `usb.rs` line 5)
**Apply to:** All new enums (`QemuConversionError`, `ProxmoxConversionError`) and any new trait methods until they are wired up

### `derive_getters` + `derive_new` import block
**Source:** `src/runtime/pci.rs` (lines 1–4), `src/runtime/pcie.rs` (lines 1–3), `src/runtime/memory.rs` (lines 1–3)
**Apply to:** Any new concrete device struct

```rust
use derive_getters::Getters;
use derive_new::new;
```

### `Result<T, String>` as lightweight typed error (interim)
**Source:** `src/config/ezkvm/runtime/builder.rs` line 36
**Apply to:** `RuntimeBuilder::build()` if a full error enum is too much for the first step; upgrade to the typed enum in the same PR

---

## No Analog Found

| File / Change | Role | Data Flow | Reason |
|---|---|---|---|
| `thiserror` derive macro usage | macro / dependency | N/A | `thiserror` is not in `Cargo.toml` and no file in the codebase uses it. The manual error enum pattern from `src/serde_yaml/error.rs` is the closest in-codebase analog. Add `thiserror = "1"` to `Cargo.toml` as part of Phase 1 and model the new error enums on the `thiserror` derive style shown in §3 above. |

---

## Metadata

**Analog search scope:** `src/runtime/`, `src/runtime/devices/`, `src/config/proxmox.rs`, `src/config/qemu.rs`, `src/config/ezkvm/runtime/builder.rs`, `src/serde_yaml/error.rs`, `Cargo.toml`
**Files scanned:** 21
**Pattern extraction date:** 2025-07-14
