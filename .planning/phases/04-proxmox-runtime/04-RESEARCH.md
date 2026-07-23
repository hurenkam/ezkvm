# Phase 4: Proxmox→Runtime Conversion — Research

**Researched:** 2025-07-22
**Domain:** Rust type conversion / Proxmox config parsing / Runtime model population
**Confidence:** HIGH — all findings from direct codebase inspection of verified source files

---

## Summary

Phase 4 implements `ProxmoxImporter`, a named struct that consumes a `ProxmoxVmConf` +
`ProxmoxStorageConf` + a numeric `vmid` and produces a fully populated `Runtime`. All
parsed Proxmox types from Phase 3 are already complete and correct. The Runtime types
from Phase 2 are already complete and correct. This phase is purely "wiring" — mapping
parsed values to typed constructors — plus the `StorageResolver` that translates
`pool:volume` references to host paths.

The single largest risk is misunderstanding how `HostPci` is placed: it is a **PCIe bus
device** (inside the Q35 chipset), not a `RootDevice`. `Q35ChipsetBuilder::with_host_pci(idx, device)`
inserts it at `PcieAddress::new(idx, 0)`. The other six root devices (Memory, Chipset,
EfiDisk, TpmState, AudioDevice, RawArgs) are registered directly on the `RuntimeBuilder`.

A secondary risk is the EfiDisk dual-size field: `logical_size` comes from
`options["size"]` and `block_device_size_bytes` is `None` (not present in Proxmox conf).
They are structurally unrelated; never convert between them.

**Primary recommendation:** Add `src/config/proxmox/importer.rs` containing
`ProxmoxImporter`, `ProxmoxImportError`, and `StorageResolver`. Export from `proxmox.rs`.
Add `pub use conf::ProxmoxVmConf;` to `proxmox.rs` (currently missing). Pass `vmid: u32`
as a third field on `ProxmoxImporter` so `StorageResolver` can handle `dir`-type storage.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Parse Proxmox conf/storage | `config::proxmox` (Phase 3 — done) | — | Parser is complete; importer consumes parsed types |
| Volume → host path resolution | `config::proxmox::importer` (StorageResolver) | — | StorageConf is in this module; resolution is import-time |
| Memory conversion | `config::proxmox::importer` | `runtime::Memory` | Simple u64 cast |
| Chipset type selection | `config::proxmox::importer` | `runtime::Chipset` | machine/bios fields drive Q35 vs I440FX |
| HostPci multi-function expansion | `config::proxmox::importer` | `runtime::HostPci` | Expansion logic belongs in the importer, not the model |
| EfiDisk / TpmState population | `config::proxmox::importer` | `runtime::EfiDisk`, `runtime::TpmState` | Proxmox options → Runtime struct fields |
| AudioDevice population | `config::proxmox::importer` | `runtime::AudioDevice` | Direct field mapping |
| RawArgs passthrough | `config::proxmox::importer` | `runtime::RawArgs` | Verbatim copy — never tokenize |
| SCSI/SATA/IDE storage placement | `config::proxmox::importer` | `runtime::q35::Q35ChipsetBuilder` | Disk devices live on chipset buses |

---

## Research Findings: All Eight Questions

### Q1 — Runtime struct and device collection

[VERIFIED: src/runtime.rs (direct read)]

```rust
#[derive(Debug, Default, Getters)]
pub struct Runtime {
    root_devices: Vec<Arc<dyn RootDevice>>,
}
```

`root_devices` is a plain `Vec`, not a BTreeMap or HashMap. Devices are appended in
registration order. There is no deduplication — registering two `Memory` devices is legal
at the type level (semantic validation is the importer's job).

`RootDeviceKind` enum (all variants):
```rust
pub enum RootDeviceKind {
    Memory,
    Chipset,
    EfiDisk,
    TpmState,
    AudioDevice,
    SpiceDisplay,
    RawArgs,
}
```

**Important:** `HostPci` is **NOT** a `RootDevice`. It implements `PcieDevice` and lives
inside `Q35Chipset.pcie_bus: HashMap<PcieAddress, Arc<dyn PcieDevice>>`. Attempting to
register it as a root device will not compile.

Individual device types and their constructors (all verified from source):

| Type | Constructor | Key Fields |
|------|-------------|------------|
| `Memory` | `Memory::new(size: usize)` | `size: usize` (MiB) |
| `Chipset` | `Chipset::Q35(q35_builder.build())` | enum variant |
| `EfiDisk` | `EfiDisk::new(storage_volume, efitype, pre_enrolled_keys, ms_cert, logical_size, block_device_size_bytes)` | ⚠️ dual-size |
| `TpmState` | `TpmState::new(storage_volume, version)` | `version: String` |
| `AudioDevice` | `AudioDevice::new(device_type, driver)` | both `String` |
| `RawArgs` | `RawArgs(content: String)` | tuple struct, `.0` field |
| `HostPci` | `HostPci::new(base_bdf, functions, pcie, x_vga, rombar, romfile)` | `functions: Vec<u8>` |

---

### Q2 — RuntimeBuilder and Q35ChipsetBuilder construction patterns

[VERIFIED: src/runtime.rs, tests/runtime_phase2.rs (direct read)]

```rust
// RuntimeBuilder — fluent chain
let runtime = RuntimeBuilder::new()
    .with_memory(Memory::new(16384))
    .with_chipset(Chipset::Q35(q35_chipset))
    .with_efidisk(efidisk)
    .with_tpmstate(tpmstate)
    .with_audio_device(audio)
    .with_raw_args(raw_args)
    .build()        // -> Result<Runtime, ()>
    .unwrap();
```

`build()` returns `Result<Runtime, ()>` — it currently never fails (infallible). The `()`
error type means it cannot carry context; the importer should use its own error type
(`ProxmoxImportError`) for all validation before calling `build()`.

```rust
// Q35ChipsetBuilder — HostPci placement
let chipset = Q35ChipsetBuilder::new()
    .with_host_pci(idx: u8, Arc::new(host_pci))   // PcieAddress::new(idx, 0)
    .with_ivshmem(idx: u8, Arc::new(ivshmem))       // PcieAddress::new(32 + idx, 0)
    .with_pcie_device(addr, pcie_device)
    .with_pci_device(addr, pci_device)
    .with_sata_device(addr, sata_device)
    .with_ide_device(addr, ide_device)
    .with_usb_device(addr, usb_device)
    .build()    // -> Q35Chipset (infallible)
```

**Key pattern from `tests/runtime_phase2.rs`:** The chipset is built separately from
root-device registration. `Chipset::Q35(q35_builder.build())` wraps the chipset before
passing it to `.with_chipset(...)`.

---

### Q3 — ProxmoxVmConf and sub-struct field names

[VERIFIED: src/config/proxmox/conf.rs, src/config/proxmox/parser.rs (direct read)]

```rust
pub struct ProxmoxVmConf {
    // Indexed device maps (key = slot number u8)
    pub scsi:    BTreeMap<u8, ProxmoxDiskConf>,
    pub sata:    BTreeMap<u8, ProxmoxDiskConf>,
    pub ide:     BTreeMap<u8, ProxmoxDiskConf>,
    pub virtio:  BTreeMap<u8, ProxmoxDiskConf>,
    pub net:     BTreeMap<u8, ProxmoxNetConf>,
    pub hostpci: BTreeMap<u8, ProxmoxHostPciConf>,
    pub usb:     BTreeMap<u8, ProxmoxUsbConf>,
    pub serial:  BTreeMap<u8, ProxmoxSerialConf>,
    // Single-instance optional devices
    pub efidisk:  Option<ProxmoxEfiDiskConf>,
    pub tpmstate: Option<ProxmoxTpmConf>,
    pub audio:    Option<ProxmoxAudioConf>,
    // Scalar fields
    pub memory:  Option<u64>,    // MiB as integer
    pub machine: Option<String>,
    pub bios:    Option<String>,
    pub args:    Option<String>, // verbatim, never tokenized
    // ... (name, cpu, cores, sockets, etc.)
}

pub struct ProxmoxDiskConf {
    pub volume: String,                       // e.g. "vm1-pool:vm-108-boot" or "none"
    pub options: BTreeMap<String, String>,    // discard, size, ssd, media, etc.
}

pub struct ProxmoxHostPciConf {
    pub bdf: String,                          // e.g. "0000:03:00" (no .function suffix)
    pub options: BTreeMap<String, String>,    // pcie, x-vga, rombar, romfile
}

pub struct ProxmoxEfiDiskConf {
    pub volume: String,
    pub options: BTreeMap<String, String>,    // efitype, pre-enrolled-keys, ms-cert, size
}

pub struct ProxmoxTpmConf {
    pub volume: String,
    pub options: BTreeMap<String, String>,    // size, version
}

pub struct ProxmoxAudioConf {
    pub device: Option<String>,   // e.g. "ich9-intel-hda"
    pub driver: Option<String>,   // e.g. "spice"
}
```

**Critical option key names** (hyphenated, not underscored):
- `options["pre-enrolled-keys"]` → `"1"` for true
- `options["ms-cert"]` → e.g. `"2023"`
- `options["efitype"]` → e.g. `"4m"`
- `options["size"]` → e.g. `"4M"` (logical_size for EfiDisk)
- `options["version"]` → e.g. `"v2.0"` (TpmState version)
- `options["ssd"]` → `"1"` for SSD, absent for HDD
- `options["media"]` → `"cdrom"` or absent
- `options["x-vga"]` → `"1"` or absent (triggers multi-function expansion)
- `options["pcie"]` → `"1"` or absent

**`ide2: none,media=cdrom`** parses to `ProxmoxDiskConf { volume: "none", options: {"media": "cdrom"} }`.
The importer MUST skip any disk where `volume == "none"`.

---

### Q4 — Existing ProxmoxSchemaBuilder stub in proxmox.rs

[VERIFIED: src/config/proxmox.rs (direct read)]

The existing code in `src/config/proxmox.rs` goes in the **opposite** direction:
`Runtime → ProxmoxVmSchema` (export, not import). The import-direction `TryFrom` stub:

```rust
impl TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime {
    type Error = ProxmoxConversionError;
    fn try_from(value: (ProxmoxVmSchema, ProxmoxHostSchema)) -> Result<Self, Self::Error> {
        let (_vm_schema, _host_schema) = value;
        let builder = RuntimeBuilder::new();
        builder.build().map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))
    }
}
```

This stub returns an EMPTY runtime. Phase 4 does NOT continue this pattern — it adds
a new `ProxmoxImporter` struct as the canonical import path. The stub above is orthogonal
(it converts `ProxmoxVmSchema`, a different type).

There is already a `ProxmoxConversionError` in `proxmox.rs` — this covers the
export direction. Phase 4 introduces `ProxmoxImportError` (in `importer.rs`) for the
import direction. Keep them separate.

**Also note:** `ProxmoxVmConf` is currently NOT re-exported from `proxmox.rs`. The `conf`
module is private. The importer.rs file must access it as:
```rust
use crate::config::proxmox::conf::ProxmoxVmConf;
```
OR add `pub use conf::ProxmoxVmConf;` (and sub-structs) to `proxmox.rs`.
Recommend adding the re-exports so integration tests can import from the public API.

---

### Q5 — Where ProxmoxImporter should live

[VERIFIED: src/config/proxmox.rs module layout (direct read)]

Current module layout:
```
src/config/proxmox.rs          ← module root, pub use re-exports
src/config/proxmox/
    conf.rs                    ← ProxmoxVmConf and sub-structs
    error.rs                   ← ProxmoxParseError
    parser.rs                  ← parse_disk_raw, parse_hostpci_raw, etc.
    storage.rs                 ← ProxmoxStorageConf, ProxmoxStorageEntry, StorageType
```

**Recommended new file:** `src/config/proxmox/importer.rs`

This follows the existing convention of one concern per file. Add to `proxmox.rs`:
```rust
mod importer;
pub use importer::{ProxmoxImporter, ProxmoxImportError};
pub use conf::ProxmoxVmConf;        // currently missing — add this
pub use conf::{                     // expose sub-structs needed by tests
    ProxmoxDiskConf, ProxmoxHostPciConf, ProxmoxEfiDiskConf,
    ProxmoxTpmConf, ProxmoxAudioConf,
};
```

Do **not** put `ProxmoxImportError` in the existing `error.rs` — that file owns
`ProxmoxParseError` (parse phase). Import errors have a different concern.

---

### Q6 — StorageResolver vmid extraction

[VERIFIED: input/felucia/storage.cfg, input/felucia/108.conf, ROADMAP.md (direct read)]

Volume references in `.conf` files follow the pattern `<storage_name>:<volume_name>`:
- `vm1-pool:vm-108-boot` → storage=`vm1-pool`, volume=`vm-108-boot`
- `vm1-pool:vm-108-efidisk` → storage=`vm1-pool`, volume=`vm-108-efidisk`
- `vm1-pool:vm-108-tpmstate` → storage=`vm1-pool`, volume=`vm-108-tpmstate`
- `local:iso/virtio-win-0.1.248.iso` → storage=`local`, volume=`iso/virtio-win-0.1.248.iso`

Resolution rules by `StorageType`:

| StorageType | Rule | Example |
|-------------|------|---------|
| `LvmThin` | `/dev/<vgname>/<volume_name>` | `vm1-pool:vm-108-boot` → `/dev/vm1/vm-108-boot` |
| `Dir` | `<path>/images/<vmid>/<volume_name>` | `local:vm-108-disk.qcow2` → `/var/lib/vz/images/108/vm-108-disk.qcow2` |
| `Lvm` | `/dev/<vgname>/<volume_name>` (same as LvmThin) | — |
| `Unknown` | error | — |

**The `vmid` is required for `Dir` type resolution.** It cannot be extracted from the
volume name reliably (Proxmox volume naming is not guaranteed to include `<vmid>-`
prefix). Pass `vmid: u32` as a constructor parameter on `ProxmoxImporter`.

In the felucia corpus, all VM disks use `vm1-pool` (LvmThin). The `local` dir storage
appears only in snapshot sections for ISO media. However `vmid` must still be accepted
by the importer for correctness with other corpora.

The `ProxmoxStorageEntry.properties` BTreeMap supplies the resolution data:
- For `LvmThin`: `properties["vgname"]` → `"vm1"`, `properties["thinpool"]` → `"pool"`
- For `Dir`: `properties["path"]` → `"/var/lib/vz"`, `properties["content"]` → `"iso,vztmpl"`

**`none` volume special case:** `ide2: none,media=cdrom` has `volume = "none"`. The
StorageResolver must return an error OR the importer must skip disks where
`volume == "none"` before calling the resolver. Recommendation: skip at importer level
(it's a valid "empty cdrom slot" semantic, not a resolution error).

---

### Q7 — ProxmoxImportError placement and variants

[VERIFIED: src/config/proxmox/error.rs, ROADMAP phase plans (direct read)]

Existing `error.rs` owns `ProxmoxParseError` (for Phase 3 parse failures). Do not add
import errors there. Create them in `importer.rs`.

Recommended `ProxmoxImportError` variants covering all failure modes:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxImportError {
    #[error("memory not specified in vm conf")]
    MissingMemory,

    #[error("unsupported machine type '{machine}' — only q35 variants are supported")]
    UnsupportedMachine { machine: String },

    #[error("storage pool '{pool}' not found in storage.cfg")]
    UnknownStorage { pool: String },

    #[error("storage pool '{pool}' is of unknown type '{storage_type}' — cannot resolve volume")]
    UnsupportedStorageType { pool: String, storage_type: String },

    #[error("storage pool '{pool}' missing required property '{property}'")]
    MissingStorageProperty { pool: String, property: String },

    #[error("volume reference '{volume}' has no pool prefix (expected 'pool:name' format)")]
    MalformedVolumeRef { volume: String },

    #[error("{device}: audio device type not specified")]
    MissingAudioDeviceType { device: String },

    #[error("{device}: audio driver not specified")]
    MissingAudioDriver { device: String },
}
```

The `device` context field satisfies success criterion 4: "returns a `ProxmoxImportError`
with device context when a required field is absent".

---

### Q8 — Integration test patterns from tests/runtime_phase2.rs

[VERIFIED: tests/runtime_phase2.rs (direct read)]

The existing Phase 2 test in `tests/runtime_phase2.rs` establishes the pattern:
- Use `ezkvm::runtime::{...}` public imports
- Construct all types with `::new()` calls
- Assert on `runtime.root_devices().len()` and specific field values via getters

Phase 4 integration test should be in `tests/runtime_phase4.rs`. Pattern:

```rust
use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::runtime::{RootDeviceKind, AudioDevice, EfiDisk, TpmState, RawArgs};
use std::str::FromStr;

#[test]
fn test_felucia_108_import_full_runtime() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let conf_path = std::path::Path::new(&manifest).join("input/felucia/108.conf");
    let storage_path = std::path::Path::new(&manifest).join("input/felucia/storage.cfg");

    let conf = ProxmoxVmConf::from_str(&std::fs::read_to_string(conf_path).unwrap()).unwrap();
    let storage = ProxmoxStorageConf::from_str(&std::fs::read_to_string(storage_path).unwrap()).unwrap();

    let runtime = ProxmoxImporter::new(conf, storage, 108)
        .into_runtime()
        .expect("import should succeed");

    // Assert all seven device types present
    let kinds: Vec<_> = runtime.root_devices().iter().map(|d| d.device_kind()).collect();
    assert!(kinds.contains(&RootDeviceKind::Memory));
    assert!(kinds.contains(&RootDeviceKind::Chipset));
    assert!(kinds.contains(&RootDeviceKind::EfiDisk));
    assert!(kinds.contains(&RootDeviceKind::TpmState));
    assert!(kinds.contains(&RootDeviceKind::AudioDevice));
    assert!(kinds.contains(&RootDeviceKind::RawArgs));

    // Assert EfiDisk resolved storage path
    let efidisk = runtime.root_devices().iter()
        .find(|d| d.device_kind() == RootDeviceKind::EfiDisk).unwrap();
    let efidisk = efidisk.as_any().downcast_ref::<EfiDisk>().unwrap();
    assert_eq!(efidisk.storage_volume(), "/dev/vm1/vm-108-efidisk");
    assert_eq!(efidisk.logical_size(), "4M");
    assert_eq!(*efidisk.pre_enrolled_keys(), true);

    // Assert HostPci functions = [0, 1] via Chipset → Q35 → pcie_bus
    // (downcast Chipset → Q35Chipset, iterate pcie_bus for HostPci)
    // ...
}
```

---

## Recommended File/Module Layout for Phase 4

```
src/config/proxmox.rs                ← add: mod importer; pub use importer::*; pub use conf::ProxmoxVmConf;
src/config/proxmox/
    importer.rs                      ← NEW: ProxmoxImporter, ProxmoxImportError, StorageResolver
    conf.rs                          ← (Phase 3, unchanged)
    error.rs                         ← (Phase 3, unchanged)
    parser.rs                        ← (Phase 3, unchanged)
    storage.rs                       ← (Phase 3, unchanged)
tests/
    runtime_phase2.rs                ← (Phase 2, unchanged)
    runtime_phase4.rs                ← NEW: integration test (Plan 04-04)
```

---

## Exact Type Signatures

### ProxmoxImporter

```rust
pub struct ProxmoxImporter {
    vm_conf: ProxmoxVmConf,
    storage_conf: ProxmoxStorageConf,
    vmid: u32,
}

impl ProxmoxImporter {
    pub fn new(vm_conf: ProxmoxVmConf, storage_conf: ProxmoxStorageConf, vmid: u32) -> Self {
        ProxmoxImporter { vm_conf, storage_conf, vmid }
    }

    pub fn into_runtime(self) -> Result<Runtime, ProxmoxImportError> {
        // build memory, chipset, efidisk, tpmstate, audio, raw_args
        // ...
    }
}
```

**Do NOT use `TryFrom<(ProxmoxVmConf, ProxmoxStorageConf)>`** — the ROADMAP explicitly
forbids this pattern. Named struct + `into_runtime(self)` is the required interface.

### ProxmoxImportError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxImportError {
    #[error("memory not specified in vm conf")]
    MissingMemory,

    #[error("unsupported machine type '{machine}'")]
    UnsupportedMachine { machine: String },

    #[error("storage pool '{pool}' not found in storage.cfg")]
    UnknownStorage { pool: String },

    #[error("storage pool '{pool}' is of unsupported type '{storage_type}'")]
    UnsupportedStorageType { pool: String, storage_type: String },

    #[error("storage pool '{pool}' missing required property '{property}'")]
    MissingStorageProperty { pool: String, property: String },

    #[error("volume reference '{volume}' is not in 'pool:name' format")]
    MalformedVolumeRef { volume: String },

    #[error("{device}: required field '{field}' not specified")]
    MissingDeviceField { device: String, field: String },
}
```

### StorageResolver

```rust
pub struct StorageResolver<'a> {
    storage_conf: &'a ProxmoxStorageConf,
    vmid: u32,
}

impl<'a> StorageResolver<'a> {
    pub fn new(storage_conf: &'a ProxmoxStorageConf, vmid: u32) -> Self {
        StorageResolver { storage_conf, vmid }
    }

    /// Resolve a Proxmox volume reference (e.g. "vm1-pool:vm-108-boot") to a host path.
    /// Returns Err if the pool is unknown, unsupported, or a required property is missing.
    pub fn resolve(&self, volume_ref: &str) -> Result<String, ProxmoxImportError> {
        // Split on first ':' to get (pool_name, volume_name)
        let (pool_name, volume_name) = volume_ref.split_once(':')
            .ok_or_else(|| ProxmoxImportError::MalformedVolumeRef {
                volume: volume_ref.to_string(),
            })?;

        let entry = self.storage_conf.entries.get(pool_name)
            .ok_or_else(|| ProxmoxImportError::UnknownStorage {
                pool: pool_name.to_string(),
            })?;

        match &entry.storage_type {
            StorageType::LvmThin | StorageType::Lvm => {
                let vgname = entry.properties.get("vgname")
                    .ok_or_else(|| ProxmoxImportError::MissingStorageProperty {
                        pool: pool_name.to_string(),
                        property: "vgname".to_string(),
                    })?;
                Ok(format!("/dev/{}/{}", vgname, volume_name))
            }
            StorageType::Dir => {
                let path = entry.properties.get("path")
                    .ok_or_else(|| ProxmoxImportError::MissingStorageProperty {
                        pool: pool_name.to_string(),
                        property: "path".to_string(),
                    })?;
                Ok(format!("{}/images/{}/{}", path, self.vmid, volume_name))
            }
            StorageType::Unknown(t) => Err(ProxmoxImportError::UnsupportedStorageType {
                pool: pool_name.to_string(),
                storage_type: t.clone(),
            }),
        }
    }
}
```

---

## StorageResolver Path Derivation Examples

From the felucia corpus (`storage.cfg` + `108.conf`):

| Volume Reference | Storage Entry | Rule | Resolved Host Path |
|-----------------|---------------|------|--------------------|
| `vm1-pool:vm-108-boot` | `lvmthin: vm1-pool`, vgname=vm1 | `/dev/<vgname>/<vol>` | `/dev/vm1/vm-108-boot` |
| `vm1-pool:vm-108-tmp` | `lvmthin: vm1-pool`, vgname=vm1 | `/dev/<vgname>/<vol>` | `/dev/vm1/vm-108-tmp` |
| `vm1-pool:vm-108-efidisk` | `lvmthin: vm1-pool`, vgname=vm1 | `/dev/<vgname>/<vol>` | `/dev/vm1/vm-108-efidisk` |
| `vm1-pool:vm-108-tpmstate` | `lvmthin: vm1-pool`, vgname=vm1 | `/dev/<vgname>/<vol>` | `/dev/vm1/vm-108-tpmstate` |
| `none` | — | skip (empty slot) | — (caller skips) |

**`none` is NOT passed to StorageResolver.** The importer must filter before calling:
```rust
if disk_conf.volume == "none" {
    continue; // empty cdrom slot — skip silently
}
let path = resolver.resolve(&disk_conf.volume)?;
```

---

## Multi-Function PCI Expansion Strategy

[VERIFIED: input/felucia/108.conf, src/runtime/devices/hostpci.rs, tests/runtime_phase2.rs]

From `108.conf`:
```
hostpci0: 0000:03:00,pcie=1,x-vga=1
```

The parser produces:
```rust
ProxmoxHostPciConf {
    bdf: "0000:03:00",     // no .function suffix
    options: { "pcie": "1", "x-vga": "1" }
}
```

The Runtime model stores:
```rust
pub struct HostPci {
    base_bdf: String,       // "0000:03:00"
    functions: Vec<u8>,     // [0, 1] for multi-function; [0] for single
    pcie: bool,
    x_vga: bool,
    rombar: Option<bool>,
    romfile: Option<String>,
}
```

**Expansion rule (from ROADMAP + test corpus):**
- `x-vga=1` → `functions: vec![0, 1]` (GPU function 0 + HDMI audio function 1)
- `x-vga=0` or absent → `functions: vec![0]` (single function only)

```rust
fn expand_functions(options: &BTreeMap<String, String>) -> Vec<u8> {
    if options.get("x-vga").map(|v| v == "1").unwrap_or(false) {
        vec![0, 1]
    } else {
        vec![0]
    }
}
```

**Base BDF normalization:** The BDF from the Proxmox conf may appear as `0000:03:00` or
`0000:03:00.0`. Strip any trailing `.N` suffix before storing in `base_bdf`:
```rust
let base_bdf = bdf.split('.').next().unwrap().to_string();
```

**Placement in Q35Chipset:**
```rust
let host_pci = HostPci::new(base_bdf, functions, pcie, x_vga, rombar, romfile);
q35_builder = q35_builder.with_host_pci(hostpci_idx, Arc::new(host_pci));
// with_host_pci(0, ...) → PcieAddress::new(0, 0)
```

---

## EfiDisk Field Mapping

[VERIFIED: src/runtime/efidisk.rs, src/config/proxmox/conf.rs, input/felucia/108.conf]

From `108.conf`:
```
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
```

Parsed as:
```rust
ProxmoxEfiDiskConf {
    volume: "vm1-pool:vm-108-efidisk",
    options: {
        "efitype": "4m",
        "ms-cert": "2023",
        "pre-enrolled-keys": "1",
        "size": "4M",
    }
}
```

Mapping to `EfiDisk::new(storage_volume, efitype, pre_enrolled_keys, ms_cert, logical_size, block_device_size_bytes)`:

| EfiDisk field | Source | Notes |
|--------------|--------|-------|
| `storage_volume: String` | `StorageResolver::resolve(&conf.volume)` | resolved host path |
| `efitype: Option<String>` | `conf.options.get("efitype").cloned()` | `Some("4m")` |
| `pre_enrolled_keys: bool` | `conf.options.get("pre-enrolled-keys").map(|v| v == "1").unwrap_or(false)` | `true` |
| `ms_cert: Option<String>` | `conf.options.get("ms-cert").cloned()` | `Some("2023")` |
| `logical_size: String` | `conf.options.get("size").cloned().unwrap_or_default()` | `"4M"` |
| `block_device_size_bytes: Option<u64>` | `None` | **not in Proxmox conf** |

⚠️ **`logical_size` and `block_device_size_bytes` are structurally unrelated.** `"4M"` ≠
`540672` bytes. The `block_device_size_bytes` is the actual LVM block device size from
a `blockdev --getsize64` query — it is NOT computable from `logical_size`. Always set it
to `None` during import.

---

## TpmState Field Mapping

From `108.conf`:
```
tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
```

```rust
TpmState::new(
    StorageResolver::resolve("vm1-pool:vm-108-tpmstate")?,  // "/dev/vm1/vm-108-tpmstate"
    conf.options.get("version").cloned().unwrap_or_else(|| "v2.0".to_string()),
)
```

The `size` option (`"4M"`) is documented in the ROADMAP as "invariant and carries no
information beyond `version = v2.0`". Do not store it.

---

## Memory Conversion

From `108.conf`: `memory: 16384`

```rust
let size_mib = vm_conf.memory.ok_or(ProxmoxImportError::MissingMemory)? as usize;
Memory::new(size_mib)
```

Memory is in MiB in Proxmox conf. `Memory::new(size: usize)` takes the same MiB value.
No unit conversion needed.

---

## Chipset Type Selection

From `108.conf`: `machine: pc-q35-8.1`, `bios: ovmf`

```rust
let chipset = match vm_conf.machine.as_deref() {
    Some(m) if m.contains("q35") => {
        let mut q35 = Q35ChipsetBuilder::new();
        // populate: hostpci devices, scsi/sata/ide/usb storage
        Chipset::Q35(q35.build())
    }
    Some(other) => return Err(ProxmoxImportError::UnsupportedMachine {
        machine: other.to_string(),
    }),
    None => return Err(ProxmoxImportError::UnsupportedMachine {
        machine: "<none>".to_string(),
    }),
};
```

`bios: ovmf` implies UEFI boot. The Runtime `Chipset` enum doesn't currently carry BIOS
type — this is already accounted for by the presence of `EfiDisk` (UEFI if present).

---

## AudioDevice Field Mapping

From `108.conf`: `audio0: device=ich9-intel-hda,driver=spice`

```rust
let device_type = conf.audio.device.ok_or_else(||
    ProxmoxImportError::MissingDeviceField {
        device: "audio0".to_string(),
        field: "device".to_string(),
    })?;
let driver = conf.audio.driver.ok_or_else(||
    ProxmoxImportError::MissingDeviceField {
        device: "audio0".to_string(),
        field: "driver".to_string(),
    })?;
AudioDevice::new(device_type, driver)
```

---

## RawArgs Passthrough

From `108.conf`:
```
args: -spice port=5903,addr=0.0.0.0,... -device virtio-serial-pci ...
```

```rust
if let Some(args) = vm_conf.args {
    runtime_builder = runtime_builder.with_raw_args(RawArgs(args));
}
```

`RawArgs` is a newtype tuple struct. Its inner string is verbatim — never split, tokenize,
or reorder. Access via `.0`.

---

## SCSI Storage in Q35 Chipset

The felucia conf has `scsi0` and `scsi1` on a `pvscsi` controller (`scsihw: pvscsi`).

The existing `PvScsiBuilder` + `PvScsi` pattern from the ezkvm schema builder applies:

```rust
let mut pvscsi_builder = PvScsiBuilder::new();
for (idx, disk_conf) in &vm_conf.scsi {
    if disk_conf.volume == "none" { continue; }
    let resolved_path = resolver.resolve(&disk_conf.volume)?;
    let is_ssd = disk_conf.options.get("ssd").map(|v| v == "1").unwrap_or(false);
    let storage: Arc<dyn ScsiDevice> = if is_ssd {
        Arc::new(Ssd::new())
    } else {
        Arc::new(Hdd::new())
    };
    pvscsi_builder = pvscsi_builder.with_scsi_device(
        Some(ScsiAddress::new(*idx, 0)),
        storage,
    );
}
q35_builder = q35_builder.with_pcie_device(
    Some(PcieAddress::new(0, 0)),
    Arc::new(pvscsi_builder.build()),
);
```

⚠️ **Design limitation:** `Hdd` and `Ssd` in `src/runtime/storage.rs` currently have NO
`resource` or path field. The resolved host path (e.g., `/dev/vm1/vm-108-boot`) is
computed by `StorageResolver` but has nowhere to go in the current `Hdd`/`Ssd` struct.

**Implication for Phase 4:** The storage path for scsi/sata/ide disks is resolved and
validated (error if pool unknown) but cannot be stored in the Runtime model — this is a
Phase 5/6 concern when the YAML schema layer (`ScsiDeviceTypeSchema { Hdd { resource } }`)
gets wired back to the runtime. For Phase 4, `resolve()` the path for validation/error
purposes and drop it; `Hdd::new()` / `Ssd::new()` have no constructor parameter for it.

The integration test for Phase 4 must therefore assert storage resolution via
`StorageResolver` unit tests and via `EfiDisk.storage_volume` / `TpmState.storage_volume`
(which DO carry paths). The scsi disk paths will not be assertable from the Runtime until
Phase 5/6 extends the model.

---

## Common Pitfalls

### Pitfall 1: HostPci as RootDevice (won't compile)
**What goes wrong:** Attempting `RuntimeBuilder::with_*` for a `HostPci` — no such method exists.
**Why:** `HostPci` implements `PcieDevice`, not `RootDevice`. The `RootDeviceKind` enum
has no `HostPci` variant.
**Prevention:** Always route `HostPci` through `Q35ChipsetBuilder::with_host_pci(idx, device)`.

### Pitfall 2: Double-counting EfiDisk size
**What goes wrong:** Computing `block_device_size_bytes` from `logical_size = "4M"` (4M =
4_194_304 bytes) — but the actual LVM device is 540672 bytes (a different physical
allocation).
**Prevention:** Always set `block_device_size_bytes: None` during import. The test
`test_efidisk_dual_size_fields_are_independent` in Phase 2 documents this: `540672 ≠ 4 * 1024 * 1024`.

### Pitfall 3: `scsihw` prefix collision with `scsi`
**What goes wrong:** `key.starts_with("scsi")` matches both `scsi0` and `scsihw`.
**Why it's not a problem in the importer:** Phase 3 parser already handles this correctly
(`scsihw` is checked FIRST before `scsi` prefix matching). `ProxmoxVmConf.scsihw` is
already populated. The importer consumes the already-parsed struct — no re-parsing needed.

### Pitfall 4: Passing `"none"` to StorageResolver
**What goes wrong:** `StorageResolver::resolve("none")` fails with `MalformedVolumeRef`
because `"none"` has no `:` separator.
**Prevention:** Check `disk_conf.volume == "none"` BEFORE calling the resolver and skip.

### Pitfall 5: Option key names are hyphenated
**What goes wrong:** `options["pre_enrolled_keys"]` returns `None`; `options["pre-enrolled-keys"]`
is the correct key.
**Prevention:** Keys in `ProxmoxDiskConf.options` use the exact Proxmox conf key names
with hyphens preserved (e.g., `pre-enrolled-keys`, `ms-cert`, `x-vga`).

### Pitfall 6: TryFrom tuple pattern (ROADMAP-forbidden)
**What goes wrong:** Implementing `TryFrom<(ProxmoxVmConf, ProxmoxStorageConf)> for Runtime`
makes vmid impossible to pass without a wrapper tuple.
**Prevention:** Use the named struct pattern only: `ProxmoxImporter { vm_conf, storage_conf, vmid }`.

### Pitfall 7: ProxmoxVmConf not publicly exported
**What goes wrong:** Integration test `use ezkvm::config::proxmox::ProxmoxVmConf` fails —
`conf` module is private.
**Prevention:** Add `pub use conf::ProxmoxVmConf;` to `src/config/proxmox.rs` in Plan 04-01.

---

## Additional Pitfalls Discovered

### Pitfall 8: `ide2: none,media=cdrom` creates a ProxmoxDiskConf with `volume = "none"`
The conf entry `ide2: none,media=cdrom` is a valid Proxmox "empty cdrom slot". The Phase 3
parser correctly produces `ProxmoxDiskConf { volume: "none", options: {"media": "cdrom"} }`.
The importer must skip these silently rather than returning a resolution error.

### Pitfall 9: `ms-cert` field may be absent
`efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M` (in snapshots)
lacks `ms-cert`. The importer must use `options.get("ms-cert").cloned()` → `None`, not
`unwrap()`.

### Pitfall 10: Snapshot sections must be ignored
`ProxmoxVmConf::from_str()` already handles snapshot filtering (Phase 3). The importer
consumes the already-filtered `ProxmoxVmConf` — snapshot data is never visible to the
importer.

---

## Environment Availability Audit

SKIPPED — Phase 4 is purely code/logic changes. No external tools, services, or CLIs
beyond the existing Rust toolchain (`cargo build`, `cargo test`) are required.

```bash
cargo --version    # Rust 1.87+ expected (edition 2024 in Cargo.toml)
```

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`) via `cargo test` |
| Config file | none (edition 2024, tests in `tests/` and `#[cfg(test)]` modules) |
| Quick run | `cargo test --test runtime_phase4 2>&1` |
| Full suite | `cargo test 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command |
|--------|----------|-----------|-------------------|
| PROX-05 | Importer populates all 7 device types | integration | `cargo test --test runtime_phase4 test_felucia_108_import_full_runtime` |
| PROX-06 | HostPci multi-function expansion | unit + integration | `cargo test proxmox::importer::tests::test_expand_functions_xvga_produces_01` |
| PROX-04 | Storage vol resolution (lvmthin) | unit | `cargo test proxmox::importer::tests::test_storage_resolver_lvmthin` |
| PROX-04 | Storage vol resolution (dir) | unit | `cargo test proxmox::importer::tests::test_storage_resolver_dir` |
| PROX-05 | EfiDisk dual-size correctness | unit | `cargo test proxmox::importer::tests::test_efidisk_mapping` |
| PROX-05 | Missing memory returns error | unit | `cargo test proxmox::importer::tests::test_missing_memory_error` |
| PROX-05 | Unknown pool returns error | unit | `cargo test proxmox::importer::tests::test_unknown_pool_error` |
| PROX-05 | `none` volume skipped silently | unit | `cargo test proxmox::importer::tests::test_none_volume_skipped` |

### Wave 0 Gaps

- [ ] `tests/runtime_phase4.rs` — integration test file (Plan 04-04)
- [ ] `src/config/proxmox/importer.rs` — unit tests embedded in `#[cfg(test)]` mod

---

## Security Domain

Phase 4 performs no I/O, no network calls, no authentication, and no cryptography. All
input is already in parsed Rust types from Phase 3. ASVS categories V2–V6 do not apply.

The one relevant threat: **path traversal in volume names**. A malformed volume name like
`vm1-pool:../../etc/passwd` would resolve to `/dev/vm1/../../etc/passwd`. The importer
does not dereference the resolved path (no `open()` call), so this is deferred to the
consumer (Phase 7/8). Document the resolved path as "untrusted until consumed by
file-open code".

---

## Sources

### Primary (HIGH confidence — direct codebase inspection)
- `src/runtime.rs` — Runtime struct, RuntimeBuilder, RootDeviceKind, all `with_*` methods
- `src/runtime/efidisk.rs` — EfiDisk constructor and field names
- `src/runtime/tpmstate.rs` — TpmState constructor
- `src/runtime/audio.rs` — AudioDevice constructor
- `src/runtime/rawargs.rs` — RawArgs tuple struct
- `src/runtime/devices/hostpci.rs` — HostPci constructor and functions field
- `src/runtime/q35.rs` — Q35ChipsetBuilder, with_host_pci, with_ivshmem
- `src/config/proxmox/conf.rs` — ProxmoxVmConf and all sub-struct field names
- `src/config/proxmox/parser.rs` — parse_hostpci_raw, parse_efidisk_raw, option key names
- `src/config/proxmox/storage.rs` — ProxmoxStorageConf, StorageType, properties BTreeMap
- `src/config/proxmox.rs` — existing module structure, ProxmoxConversionError
- `src/config/proxmox/error.rs` — ProxmoxParseError (separate concern)
- `tests/runtime_phase2.rs` — integration test patterns, EfiDisk/TpmState/HostPci construction
- `input/felucia/108.conf` — primary test corpus, all field values
- `input/felucia/storage.cfg` — storage resolver test corpus

### Secondary (HIGH confidence — planning artifacts)
- `.planning/ROADMAP.md` — Phase 4 plans, pitfalls, success criteria
- `.planning/REQUIREMENTS.md` — PROX-04, PROX-05, PROX-06

---

## Metadata

**Confidence breakdown:**
- Type signatures: HIGH — read directly from source
- Field names / constructors: HIGH — read directly from source
- StorageResolver logic: HIGH — derived from storage.cfg + ROADMAP rules + felucia corpus
- Multi-function expansion rule: HIGH — confirmed by tests/runtime_phase2.rs and ROADMAP
- Hdd/Ssd path limitation: HIGH — Hdd and Ssd structs have zero fields (empty)

**Research date:** 2025-07-22
**Valid until:** Until any of the following files change: `src/runtime.rs`,
`src/runtime/efidisk.rs`, `src/runtime/devices/hostpci.rs`, `src/config/proxmox/conf.rs`,
`src/config/proxmox/storage.rs` (~30 day window otherwise)
