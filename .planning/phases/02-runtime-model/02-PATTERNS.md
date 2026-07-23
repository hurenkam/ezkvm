# Phase 2: Runtime Model — Pattern Map

**Mapped:** 2025-07-22
**Files analyzed:** 7 new device files + 4 integration touch-points
**Analogs found:** 7 / 7

---

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `src/runtime/devices/host_pci.rs` | device (PcieDevice) | request-response | `src/runtime/devices/pcie_net.rs` | exact |
| `src/runtime/devices/ivshmem.rs` | device (PcieDevice) | request-response | `src/runtime/devices/pcie_net.rs` | exact |
| `src/runtime/devices/audio_device.rs` | device (PcieDevice) | request-response | `src/runtime/devices/pcie_net.rs` | exact |
| `src/runtime/devices/efi_disk.rs` | device (SataDevice / ScsiDevice) | request-response | `src/runtime/storage.rs` (`Ssd`) | exact |
| `src/runtime/devices/tpm_state.rs` | root device | request-response | `src/runtime/memory.rs` | role-match |
| `src/runtime/devices/spice_display.rs` | root device | request-response | `src/runtime/memory.rs` | role-match |
| `src/runtime/devices/raw_args.rs` | root device | request-response | `src/runtime/memory.rs` | role-match |

**Integration touch-points (modified, not new):**

| Modified File | Change | Pattern Source |
|---------------|--------|----------------|
| `src/runtime/devices.rs` | add `mod` + `pub use` for all 7 | `src/runtime/devices.rs` lines 1–8 |
| `src/runtime.rs` | extend `pub use devices::{…}` line | `src/runtime.rs` line 15 |
| `src/runtime/q35.rs` | add `format_pcie_device` arms for new PCIe types | `src/runtime/q35.rs` lines 158–175 |
| `src/config/ezkvm/runtime/builder.rs` | add `match_*` arms for new device types | `src/config/ezkvm/runtime/builder.rs` lines 298–344 |

---

## Pattern Assignments

---

### `src/runtime/devices/host_pci.rs` (PcieDevice, request-response)

**Analog:** `src/runtime/devices/pcie_net.rs`

**Imports pattern** (pcie_net.rs lines 1–4):
```rust
use derive_getters::Getters;
use derive_new::new;

use crate::runtime::PcieDevice;
```

**Struct + derive pattern** (pcie_net.rs lines 6–14):
```rust
#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct VirtioNetPcie {
    resource: Option<String>,
    mac_address: Option<String>,
    rx_queue_size: Option<u16>,
    tx_queue_size: Option<u16>,
    vhost: Option<bool>,
}
```

**Trait impl pattern** (pcie_net.rs lines 16–20):
```rust
impl PcieDevice for VirtioNetPcie {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

**Applied to `HostPci`** — adapt struct fields for host PCI passthrough:
```rust
// Fields to model from: hostpci0: 0000:03:00,pcie=1,x-vga=1
pub struct HostPci {
    host: String,               // PCI address on host, e.g. "0000:03:00"
    pcie: Option<bool>,         // pcie=1 flag
    x_vga: Option<bool>,        // x-vga=1 flag
    rombar: Option<bool>,
    romfile: Option<String>,
    multifunction: Option<bool>,
}
```

---

### `src/runtime/devices/ivshmem.rs` (PcieDevice, request-response)

**Analog:** `src/runtime/devices/pcie_net.rs`

Same imports + derive + `PcieDevice` impl pattern as `HostPci` above.

**Applied to `Ivshmem`** — adapt struct fields from schema/conf:
```rust
// Fields to model from:
// args: -device ivshmem-plain,memdev=ivshmem0,bus=pcie.0
//       -object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
pub struct Ivshmem {
    mem_path: String,           // e.g. "/dev/kvmfr0"
    size: String,               // e.g. "128M"
}
```

---

### `src/runtime/devices/audio_device.rs` (PcieDevice, request-response)

**Analog:** `src/runtime/devices/pcie_net.rs`

Same imports + derive + `PcieDevice` impl pattern.

**Applied to `AudioDevice`** — adapt fields from:
```
audio0: device=ich9-intel-hda,driver=spice
```
```rust
pub struct AudioDevice {
    device_model: String,       // e.g. "ich9-intel-hda"
    driver: String,             // e.g. "spice", "alsa", "pa"
    codec: Option<String>,
}
```

Note: Schema already defines `PcieDeviceTypeSchema::Ich9IntelHda { codec: Option<String> }` in
`src/config/ezkvm/schema/pcie.rs` lines 65–68. The runtime `AudioDevice` is its Runtime-layer counterpart.

---

### `src/runtime/devices/efi_disk.rs` (SataDevice + ScsiDevice, request-response)

**Analog:** `src/runtime/storage.rs` (`Ssd` implementation)

**Imports pattern** (storage.rs lines 1–6):
```rust
use crate::runtime::{IdeDevice, SataDevice, scsi::ScsiDevice};

pub trait StorageDevice {
    fn storage_options(&self) -> StorageOptions;
}
```

**Struct + multi-bus trait pattern** (storage.rs lines 22–68):
```rust
#[derive(Debug)]
pub struct Ssd {}
impl Ssd {
    pub fn new() -> Self { Ssd {} }
}

impl IdeDevice for Ssd {}

impl SataDevice for Ssd {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

impl ScsiDevice for Ssd {
    fn as_any(&self) -> &dyn std::any::Any { self }
}

impl StorageDevice for Ssd {
    fn storage_options(&self) -> StorageOptions {
        StorageOptions {
            device_type: StorageDeviceType::Ssd,
            read_only: false,
            cache_mode: None,
        }
    }
}
```

**Applied to `EfiDisk`** — adapt for EFI firmware storage fields:
```rust
// Fields to model from:
// efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
pub struct EfiDisk {
    resource: String,           // storage pool resource reference
    efi_type: String,           // e.g. "4m"
    pre_enrolled_keys: bool,
    ms_cert: Option<String>,    // optional ms cert year
    size: String,               // e.g. "4M"
}
```

`EfiDisk` implements `SataDevice` (and optionally `ScsiDevice`, `IdeDevice`) + `StorageDevice`
with `device_type: StorageDeviceType::Ssd` (read-write, no default cache mode).
This follows the exact multi-bus pattern of `Ssd`.

---

### `src/runtime/devices/tpm_state.rs` (RootDevice, request-response)

**Analog:** `src/runtime/memory.rs`

**Imports pattern** (memory.rs lines 1–4):
```rust
use derive_getters::Getters;
use derive_new::new;

use crate::runtime::RootDevice;
```

**Struct + RootDevice impl pattern** (memory.rs lines 6–31):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Default, Getters, new)]
pub struct Memory {
    size: usize,
}

impl RootDevice for Memory {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "memory" }
}

impl std::fmt::Display for Memory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Memory: {}", self.size)
    }
}
```

**Applied to `TpmState`**:
```rust
// Fields to model from: tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
pub struct TpmState {
    resource: String,           // e.g. "vm1-pool:vm-108-tpmstate"
    version: String,            // e.g. "v2.0"
    size: String,               // e.g. "4M"
}
impl RootDevice for TpmState {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "tpm_state" }
}
```

**RuntimeBuilder integration:** Add `with_tpm_state(tpm: TpmState)` method to `RuntimeBuilder`
following the pattern of `with_memory` (runtime.rs lines 102–105):
```rust
pub fn with_memory(self, memory: Memory) -> Self {
    self.root_devices.lock().unwrap().push(Arc::new(memory));
    self
}
```

---

### `src/runtime/devices/spice_display.rs` (RootDevice, request-response)

**Analog:** `src/runtime/memory.rs`

Same RootDevice impl pattern as `TpmState` above.

**Applied to `SpiceDisplay`**:
```rust
// Fields to model from:
// args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on ...
pub struct SpiceDisplay {
    port: u16,
    listen: String,             // e.g. "0.0.0.0"
    disable_ticketing: bool,
    gl_enabled: bool,
    tls_port: Option<u16>,
    seamless_migration: bool,
}
impl RootDevice for SpiceDisplay {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "spice_display" }
}
```

**RuntimeBuilder integration:** Add `with_spice_display(display: SpiceDisplay)` following
`with_memory` pattern (runtime.rs lines 102–105).

---

### `src/runtime/devices/raw_args.rs` (RootDevice, request-response)

**Analog:** `src/runtime/memory.rs`

Same RootDevice impl pattern.

**Applied to `RawArgs`**:
```rust
// Fields to model from: args: -spice port=5903 -device ... -object ...
pub struct RawArgs {
    args: Vec<String>,          // individual CLI tokens, pre-split on whitespace
}
impl RootDevice for RawArgs {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "raw_args" }
}
```

**RuntimeBuilder integration:** Add `with_raw_args(args: RawArgs)` following
`with_memory` pattern (runtime.rs lines 102–105).

---

## Shared Patterns

### 1. PCIe device template

**Source:** `src/runtime/devices/pcie_net.rs` (entire file, 21 lines)
**Apply to:** `host_pci.rs`, `ivshmem.rs`, `audio_device.rs`

```rust
use derive_getters::Getters;
use derive_new::new;
use crate::runtime::PcieDevice;

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct <Name> {
    // ... typed fields, all Option<T> where the conf key is optional
}

impl PcieDevice for <Name> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

### 2. RootDevice template

**Source:** `src/runtime/memory.rs` (entire file, 32 lines)
**Apply to:** `tpm_state.rs`, `spice_display.rs`, `raw_args.rs`

```rust
use derive_getters::Getters;
use derive_new::new;
use crate::runtime::RootDevice;

#[allow(dead_code)]
#[derive(Debug, Clone, Getters, new)]
pub struct <Name> {
    // ... fields
}

impl RootDevice for <Name> {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn get_name(&self) -> &str { "<snake_case_name>" }
}

impl std::fmt::Display for <Name> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Name>: ...")
    }
}
```

### 3. RuntimeBuilder `with_X` method for RootDevices

**Source:** `src/runtime.rs` lines 102–110
**Apply to:** `with_tpm_state`, `with_spice_display`, `with_raw_args` additions

```rust
pub fn with_memory(self, memory: Memory) -> Self {
    self.root_devices.lock().unwrap().push(Arc::new(memory));
    self
}

pub fn with_chipset(self, chipset: Chipset) -> Self {
    self.root_devices.lock().unwrap().push(Arc::new(chipset));
    self
}
```

### 4. `devices.rs` module gateway — `mod` + `pub use *` pattern

**Source:** `src/runtime/devices.rs` (entire file, 9 lines)
**Apply to:** all 7 new device files

```rust
mod pvscsi;
mod pcie_net;
mod pci_generic;
mod usb_generic;
pub use pvscsi::*;
pub use pcie_net::*;
pub use pci_generic::*;
pub use usb_generic::*;
```

Each new device gets one `mod <name>;` line and one `pub use <name>::*;` line in `devices.rs`.

### 5. `runtime.rs` pub-use extension pattern

**Source:** `src/runtime.rs` line 15
**Apply to:** `src/runtime.rs` — extend the `pub use devices::{...}` list

```rust
// Current:
pub use devices::{GenericPciDevice, GenericUsbDevice, PciDeviceKind, PvScsi, PvScsiBuilder, UsbDeviceKind, VirtioNetPcie};

// After Phase 2 — add new exports:
pub use devices::{
    AudioDevice, EfiDisk, GenericPciDevice, GenericUsbDevice, HostPci, Ivshmem,
    PciDeviceKind, PvScsi, PvScsiBuilder, RawArgs, SpiceDisplay, TpmState,
    UsbDeviceKind, VirtioNetPcie,
};
```

### 6. Q35 display formatter arms — `format_pcie_device` extension

**Source:** `src/runtime/q35.rs` lines 158–175
**Apply to:** new PCIe types in `format_pcie_device`

```rust
fn format_pcie_device(device: &dyn PcieDevice) -> String {
    if let Some(pvscsi) = device.as_any().downcast_ref::<PvScsi>() {
        return format!("{}", pvscsi);
    }
    if let Some(virtio_net) = device.as_any().downcast_ref::<VirtioNetPcie>() {
        return format!("VirtioNetPcie(resource={:?}, ...)", virtio_net.resource(), ...);
    }
    // Add arms for HostPci, Ivshmem, AudioDevice following same downcast_ref pattern
    format!("{:?}", device)
}
```

### 7. StorageDevice + multi-bus trait pattern

**Source:** `src/runtime/storage.rs` lines 22–112
**Apply to:** `efi_disk.rs`

`EfiDisk` follows the exact `Ssd` pattern: implement `StorageDevice`, `SataDevice`, `ScsiDevice`,
and optionally `IdeDevice`, all via `as_any()` forwarding.

---

## File Naming Conventions

| Pattern | Convention | Examples |
|---------|-----------|---------|
| Device file names | `snake_case.rs` in `src/runtime/devices/` | `pcie_net.rs`, `pci_generic.rs`, `usb_generic.rs`, `pvscsi.rs` |
| Struct names | `PascalCase` (not matching file exactly) | file `pcie_net.rs` → struct `VirtioNetPcie` |
| New device structs | Match the Proxmox/conf key concept | `host_pci.rs` → `HostPci`, `efi_disk.rs` → `EfiDisk` |
| Module file | `devices.rs` (not `devices/mod.rs`) | project uses `name.rs + name/` layout per README |

---

## No Analog Found

All 7 new device types have close analogs. No files require falling back to external research patterns.

---

## Metadata

**Analog search scope:** `src/runtime/`, `src/runtime/devices/`, `src/config/ezkvm/`
**Key files read:** `pvscsi.rs`, `pcie_net.rs`, `pci_generic.rs`, `usb_generic.rs`, `memory.rs`,
  `storage.rs`, `pcie.rs`, `pci.rs`, `usb.rs`, `isa.rs`, `scsi.rs`, `sata.rs`, `ide.rs`,
  `q35.rs`, `chipset.rs`, `runtime.rs` (top-level), `devices.rs` (gateway),
  `builder.rs` (ezkvm config runtime), `pcie.rs` (schema), `display.rs`, `audio.rs`, `tpm.rs`
**Pattern extraction date:** 2025-07-22
