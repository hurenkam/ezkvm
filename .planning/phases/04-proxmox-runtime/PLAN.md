# Phase 04 — Proxmox → Runtime Conversion

**Goal:** Implement `ProxmoxImporter` that converts `ProxmoxVmConf` + `ProxmoxStorageConf`
into a fully populated `Runtime`, with all seven v1 device types, storage volumes resolved to
host paths, and descriptive errors on failure.

**Wave structure:**

| Wave | Plans | Can run in parallel? |
|------|-------|----------------------|
| 1 | 04-01 | sole plan |
| 2 | 04-02 | depends on 04-01 scaffold |
| 3 | 04-03 | depends on 04-01 + 04-02 |
| 4 | 04-04 | depends on all |

---

## Plan 04-01 — Scaffold, `resource` field, re-exports, base conversion

**Wave 1 — no dependencies.**

Lay the structural foundation: add the `resource` field the Proxmox importer requires to
`Hdd`/`Ssd`/`Cdrom`, expose `ProxmoxVmConf` and friends publicly, create the `ProxmoxImporter`
skeleton, define all `ProxmoxImportError` variants, and implement Memory + disk-device
conversion (using raw volume strings as placeholder resource values until Plan 04-03 wires
in `StorageResolver`).

---

### Task 04-01-A — Add `resource: String` to `Hdd`, `Ssd`, `Cdrom`

**File:** `src/runtime/storage.rs`

**What to implement:**

Add a public `resource: String` field to each of `Hdd`, `Ssd`, and `Cdrom`.  Update their
manual `new()` constructors to accept `resource: String` as the sole argument and store it.
No trait implementations change; only the struct definitions and constructors are touched.

The updated shapes are:

```
pub struct Hdd { pub resource: String }
impl Hdd { pub fn new(resource: String) -> Self { Hdd { resource } } }

pub struct Ssd { pub resource: String }
impl Ssd { pub fn new(resource: String) -> Self { Ssd { resource } } }

pub struct Cdrom { pub resource: String }
impl Cdrom { pub fn new(resource: String) -> Self { Cdrom { resource } } }
```

All `IdeDevice`, `SataDevice`, `ScsiDevice`, and `StorageDevice` trait impls on each type
remain unchanged — do not touch them.

Check whether any other file in `src/` or `tests/` calls `Hdd::new()`, `Ssd::new()`, or
`Cdrom::new()` without an argument and fix those call sites so the crate compiles.  (At time
of writing there are no such call sites, but verify with `grep -rn "Hdd::new\|Ssd::new\|Cdrom::new" src/ tests/` before proceeding.)

**Verification:** `cargo check` exits 0.

---

### Task 04-01-B — Re-exports, `mod importer`, and `ProxmoxImportError` variants

**Files:**
- `src/config/proxmox.rs` — add `mod importer;` and `pub use` lines
- `src/config/proxmox/error.rs` — add `ProxmoxImportError` enum
- `src/config/proxmox/importer.rs` — create new file with `ProxmoxImporter` struct + stub `into_runtime()`

**What to implement:**

**`src/config/proxmox.rs`** — at the top, add `mod importer;` after the existing `mod storage;`
line (keep bare, not `pub mod`).  Add these `pub use` lines after the existing ones:

```rust
pub use conf::{ProxmoxVmConf, ProxmoxDiskConf, ProxmoxAudioConf,
               ProxmoxEfiDiskConf, ProxmoxHostPciConf, ProxmoxTpmConf};
pub use importer::ProxmoxImporter;
pub use error::ProxmoxImportError;
```

**`src/config/proxmox/error.rs`** — append a second enum `ProxmoxImportError` (keep the
existing `ProxmoxParseError`):

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxImportError {
    #[error("vm_conf has no memory field")]
    MissingMemory,

    #[error("machine type '{machine}' is not supported (only q35 variants)")]
    UnsupportedMachine { machine: String },

    #[error("volume reference '{raw}' has no ':' separator — cannot resolve storage pool")]
    InvalidVolumeRef { raw: String },

    #[error("storage pool '{pool}' not found in storage.cfg")]
    UnknownStorage { pool: String },

    #[error("storage pool '{pool}' has unsupported type '{storage_type}'")]
    UnsupportedStorageType { pool: String, storage_type: String },

    #[error("storage pool '{pool}' is missing required property '{property}'")]
    MissingStorageProperty { pool: String, property: String },
}
```

**`src/config/proxmox/importer.rs`** — create this file (no `mod.rs`; file-based module):

```rust
use crate::config::proxmox::{ProxmoxStorageConf, ProxmoxVmConf};
use crate::config::proxmox::error::ProxmoxImportError;
use crate::runtime::Runtime;

pub struct ProxmoxImporter {
    pub vm_conf: ProxmoxVmConf,
    pub storage_conf: ProxmoxStorageConf,
    pub vmid: u32,
}

impl ProxmoxImporter {
    pub fn new(vm_conf: ProxmoxVmConf, storage_conf: ProxmoxStorageConf, vmid: u32) -> Self {
        ProxmoxImporter { vm_conf, storage_conf, vmid }
    }

    pub fn into_runtime(self) -> Result<Runtime, ProxmoxImportError> {
        todo!("implemented in 04-01-C through 04-03-B")
    }
}
```

**Verification:** `cargo check` exits 0.

---

### Task 04-01-C — Implement base conversion: Memory + disk devices

**File:** `src/config/proxmox/importer.rs`

**What to implement:**

Replace the `todo!()` body of `into_runtime()` with a working implementation for Memory, the
Q35 chipset scaffold, and all Proxmox disk buses (scsi / sata / ide / virtio).  Storage
volumes are **not** resolved in this plan — use the raw volume string (e.g.
`"vm1-pool:vm-108-boot"`) directly as the `resource` value.  Plan 04-03 will replace these
with real host paths.

**Memory:** Return `ProxmoxImportError::MissingMemory` if `vm_conf.memory` is `None`.
Construct `Memory::new(mb as usize)` from the parsed MB value.

**Disk type classification:** A `ProxmoxDiskConf` becomes:
- `Cdrom`  when `options.get("media") == Some("cdrom")`
- `Ssd`    when `options.get("ssd") == Some("1")`  (and not cdrom)
- `Hdd`    otherwise

The `resource` field for all three is set to `disk_conf.volume.clone()` (the raw
`"pool:volume"` string) — this is explicitly a placeholder until `StorageResolver` is wired
in Plan 04-03.

**`ide` skip rule:** Before classifying any `ide` entry, check whether `disk_conf.volume` contains
a `:` character.  If it does **not** (e.g. `"none"`), skip that entry entirely — do not
attempt to create a device for it and do not call the resolver (no resolver exists yet).

**Scsi disks → `PvScsi` on PCIe slot 16:**

```
let mut pvscsi_builder = PvScsiBuilder::new();
for (idx, disk_conf) in &self.vm_conf.scsi {
    if !disk_conf.volume.contains(':') { continue; }
    let device: Arc<dyn ScsiDevice> = classify_disk(disk_conf);
    pvscsi_builder = pvscsi_builder
        .with_scsi_device(Some(ScsiAddress::new(*idx, 0)), device);
}
// Attach PvScsi to PCIe slot 16 to avoid colliding with hostpci at slots 0..N
chipset_builder = chipset_builder
    .with_pcie_device(Some(PcieAddress::new(16, 0)), Arc::new(pvscsi_builder.build()));
```

Use `PcieAddress::new(16, 0)` specifically so that the PvScsi controller does not overlap
with `hostpci` entries that are placed at `PcieAddress::new(idx, 0)` for idx = 0, 1, 2, …

**Sata disks → sata bus (port = idx, device = 0):**

```
for (idx, disk_conf) in &self.vm_conf.sata {
    if !disk_conf.volume.contains(':') { continue; }
    let device: Arc<dyn SataDevice> = classify_disk_sata(disk_conf);
    chipset_builder = chipset_builder
        .with_sata_device(Some(SataAddress::new(*idx, 0)), device);
}
```

**Ide disks → ide bus (channel = idx, device = 0), after skipping "none" volumes:**

```
for (idx, disk_conf) in &self.vm_conf.ide {
    if !disk_conf.volume.contains(':') { continue; }
    let device: Arc<dyn IdeDevice> = classify_disk_ide(disk_conf);
    chipset_builder = chipset_builder
        .with_ide_device(Some(IdeAddress::new(*idx, 0)), device);
}
```

**Virtio disks → sata bus (port = 32 + idx, device = 0) to keep them distinct from sata:**

```
for (idx, disk_conf) in &self.vm_conf.virtio {
    if !disk_conf.volume.contains(':') { continue; }
    let device: Arc<dyn SataDevice> = classify_disk_sata(disk_conf);
    chipset_builder = chipset_builder
        .with_sata_device(Some(SataAddress::new(32 + idx, 0)), device);
}
```

**Machine type guard** (before building chipset): Return an error for non-Q35 machines:

```rust
if let Some(m) = &self.vm_conf.machine {
    if !m.contains("q35") {
        return Err(ProxmoxImportError::UnsupportedMachine { machine: m.clone() });
    }
}
```

**Chipset wrapping:** Wrap the built Q35 chipset inside `Chipset::Q35(...)` before passing to
`RuntimeBuilder::with_chipset`.

**RuntimeBuilder — use a mutable variable** (required because 04-02 extends it conditionally):

```rust
let mut builder = RuntimeBuilder::new()
    .with_memory(memory)
    .with_chipset(Chipset::Q35(chipset_builder.build()));
// 04-02 will extend builder here with .with_efidisk(), .with_tpmstate(), etc.
let runtime = builder.build()
    .map_err(|_| unreachable!("RuntimeBuilder::build() never fails"))?;
Ok(runtime)
```

> **Note for 04-02-A**: When implementing EfiDisk/TpmState, restructure this to
> `let mut builder = RuntimeBuilder::new()…;` and then conditionally call
> `builder = builder.with_efidisk(…);` etc. before the final `.build()`.


**Imports needed** in `importer.rs`:

```rust
use std::sync::Arc;
use crate::runtime::{
    Chipset, Cdrom, Hdd, IdeAddress, IdeDevice, Memory, PcieAddress,
    Q35ChipsetBuilder, RuntimeBuilder, SataAddress, SataDevice, ScsiAddress,
    ScsiDevice, Ssd,
};
use crate::runtime::{PvScsi, PvScsiBuilder};
```

(Add more as the compiler demands; the list above covers the base conversion.)

**Verification:** `cargo check` exits 0.

**Commit manually with:**
```
feat(proxmox-import/04-01): scaffold ProxmoxImporter, resource field, base conversion
```

---

## Plan 04-02 — EfiDisk, TpmState, HostPci, AudioDevice, RawArgs

**Wave 2 — depends on Plan 04-01.**

Implement the remaining five root-device conversions inside `into_runtime()` so that all
seven v1 `RootDeviceKind` variants are reachable through the importer.

---

### Task 04-02-A — EfiDisk and TpmState conversion

**File:** `src/config/proxmox/importer.rs`

**What to implement:**

Extend the `RuntimeBuilder` chain in `into_runtime()` to conditionally attach `EfiDisk` and
`TpmState`.

**EfiDisk** — if `vm_conf.efidisk` is `Some(conf)`:

```
storage_volume  = conf.volume.clone()          // raw "pool:volume" placeholder
efitype         = conf.options.get("efitype").cloned()
pre_enrolled    = conf.options.get("pre-enrolled-keys").map_or(false, |v| v == "1")
ms_cert         = conf.options.get("ms-cert").cloned()
logical_size    = conf.options.get("size").cloned().unwrap_or_else(|| "4M".to_string())
block_device_size_bytes = None                 // always None during import (DUAL-SIZE rule)
```

Construct via `EfiDisk::new(storage_volume, efitype, pre_enrolled, ms_cert, logical_size, None)`.

The `block_device_size_bytes` is `None` during import because Proxmox conf carries only the
logical size string; the actual block-device byte count is only known after the disk is
physically probed on the host.  See the dual-size comment in `src/runtime/efidisk.rs`.

**TpmState** — if `vm_conf.tpmstate` is `Some(conf)`:

```
storage_volume  = conf.volume.clone()          // raw "pool:volume" placeholder
version         = conf.options.get("version").cloned().unwrap_or_else(|| "v2.0".to_string())
```

Construct via `TpmState::new(storage_volume, version)`.

Add `with_efidisk` and `with_tpmstate` calls to the `RuntimeBuilder` chain **only** when the
respective `Option` is `Some`.

**Imports to add:**

```rust
use crate::runtime::{EfiDisk, TpmState};
```

**Verification:** Add a unit test inside `importer.rs` (gated with `#[cfg(test)]`):

```rust
#[test]
fn test_04_02_efidisk_from_proxmox_conf() {
    use std::str::FromStr;
    use crate::config::proxmox::storage::ProxmoxStorageConf;
    use crate::config::proxmox::conf::ProxmoxVmConf;

    let vm_conf: ProxmoxVmConf = concat!(
        "efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M\n",
        "memory: 4096\n"
    ).parse().unwrap();

    let importer = ProxmoxImporter::new(vm_conf, ProxmoxStorageConf::default(), 108);
    let runtime = importer.into_runtime().unwrap();

    let efidisk = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<EfiDisk>())
        .expect("EfiDisk not found in Runtime");

    assert_eq!(efidisk.logical_size(), "4M");
    assert_eq!(efidisk.efitype().as_deref(), Some("4m"));
    assert!(*efidisk.pre_enrolled_keys());
    assert_eq!(efidisk.ms_cert().as_deref(), Some("2023"));
    assert_eq!(*efidisk.block_device_size_bytes(), None::<u64>);
}

#[test]
fn test_04_02_tpmstate_from_proxmox_conf() {
    use std::str::FromStr;
    use crate::config::proxmox::storage::ProxmoxStorageConf;
    use crate::config::proxmox::conf::ProxmoxVmConf;

    let vm_conf: ProxmoxVmConf = "tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0\nmemory: 4096\n"
        .parse().unwrap();

    let importer = ProxmoxImporter::new(vm_conf, ProxmoxStorageConf::default(), 108);
    let runtime = importer.into_runtime().unwrap();

    let tpm = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<TpmState>())
        .expect("TpmState not found in Runtime");

    assert_eq!(tpm.storage_volume(), "vm1-pool:vm-108-tpmstate");
    assert_eq!(tpm.version(), "v2.0");
}
```

Run with `cargo test test_04_02_`.

---

### Task 04-02-B — HostPci multi-function expansion

**File:** `src/config/proxmox/importer.rs`

**What to implement:**

For each entry in `vm_conf.hostpci`, construct a `HostPci` and attach it to the Q35 chipset
builder via `with_host_pci(idx, Arc::new(host_pci))`.

**BDF normalisation:** The Proxmox BDF may carry a function suffix (e.g. `"0000:03:00.0"`).
Strip it:

```rust
let base_bdf = conf.bdf
    .rsplit_once('.')
    .map(|(prefix, _)| prefix.to_string())
    .unwrap_or_else(|| conf.bdf.clone());
```

**Multi-function rule:**

```rust
let x_vga   = conf.options.get("x-vga").map_or(false, |v| v == "1");
let pcie    = conf.options.get("pcie").map_or(false, |v| v == "1");
let rombar  = conf.options.get("rombar").map(|v| v == "1");
let romfile = conf.options.get("romfile").cloned();
let functions = if x_vga { vec![0u8, 1] } else { vec![0u8] };
```

**Wire into chipset builder** (must happen before the chipset is `.build()`-ed):

```rust
chipset_builder = chipset_builder
    .with_host_pci(*idx, Arc::new(HostPci::new(base_bdf, functions, pcie, x_vga, rombar, romfile)));
```

Note: `with_host_pci(idx, device)` places the device at `PcieAddress::new(idx, 0)`.  The
PvScsi controller is at slot 16 (Task 04-01-C), so indices 0–15 are free for passthrough
devices.

**Imports to add:**

```rust
use crate::runtime::HostPci;
```

**Verification:** Add a unit test:

```rust
#[test]
fn test_04_02_hostpci_x_vga_expands_functions() {
    use std::str::FromStr;
    use crate::config::proxmox::storage::ProxmoxStorageConf;
    use crate::config::proxmox::conf::ProxmoxVmConf;
    use crate::runtime::{Chipset, Q35Chipset, PcieBusDeviceKind};

    let vm_conf: ProxmoxVmConf =
        "hostpci0: 0000:03:00,pcie=1,x-vga=1\nmemory: 4096\n".parse().unwrap();

    let importer = ProxmoxImporter::new(vm_conf, ProxmoxStorageConf::default(), 108);
    let runtime = importer.into_runtime().unwrap();

    let chipset = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<Chipset>())
        .expect("Chipset not found");

    let q35 = match chipset { Chipset::Q35(q) => q, _ => panic!("expected Q35") };

    let host_pci_entry = q35.pcie_bus().values()
        .find(|dev| dev.device_kind() == PcieBusDeviceKind::HostPci)
        .expect("HostPci not found on PCIe bus");

    let host_pci = host_pci_entry.as_any().downcast_ref::<HostPci>().unwrap();
    assert_eq!(host_pci.base_bdf(), "0000:03:00");
    assert_eq!(host_pci.functions(), &[0u8, 1]);
    assert!(*host_pci.x_vga());
    assert!(*host_pci.pcie());
}
```

Run with `cargo test test_04_02_`.

---

### Task 04-02-C — AudioDevice and RawArgs conversion

**File:** `src/config/proxmox/importer.rs`

**What to implement:**

**AudioDevice** — if `vm_conf.audio` is `Some(conf)`:

```rust
let device_type = conf.device.clone().unwrap_or_else(|| "ich9-intel-hda".to_string());
let driver      = conf.driver.clone().unwrap_or_else(|| "spice".to_string());
builder = builder.with_audio_device(AudioDevice::new(device_type, driver));
```

**RawArgs** — if `vm_conf.args` is `Some(ref args)`:

```rust
builder = builder.with_raw_args(RawArgs(args.clone()));
```

The `RawArgs` inner string is stored verbatim — do not split, tokenise, or reorder it.

**Imports to add:**

```rust
use crate::runtime::{AudioDevice, RawArgs};
```

**Verification:** Add a unit test:

```rust
#[test]
fn test_04_02_audio_and_rawargs() {
    use std::str::FromStr;
    use crate::config::proxmox::storage::ProxmoxStorageConf;
    use crate::config::proxmox::conf::ProxmoxVmConf;

    let vm_conf: ProxmoxVmConf = concat!(
        "memory: 4096\n",
        "audio0: device=ich9-intel-hda,driver=spice\n",
        "args: -device vfio-pci\n",
    ).parse().unwrap();

    let importer = ProxmoxImporter::new(vm_conf, ProxmoxStorageConf::default(), 108);
    let runtime = importer.into_runtime().unwrap();

    let audio = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<AudioDevice>())
        .expect("AudioDevice missing");
    assert_eq!(audio.device_type(), "ich9-intel-hda");
    assert_eq!(audio.driver(), "spice");

    let raw = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<RawArgs>())
        .expect("RawArgs missing");
    assert_eq!(raw.0, "-device vfio-pci");
}
```

Run with `cargo test test_04_02_`.

**Commit manually with:**
```
feat(proxmox-import/04-02): EfiDisk, TpmState, HostPci, AudioDevice, RawArgs conversion
```

---

## Plan 04-03 — StorageResolver

**Wave 3 — depends on Plans 04-01 and 04-02.**

Implement `StorageResolver` and wire it into `into_runtime()` so all volume strings become
real host paths.

---

### Task 04-03-A — Implement `StorageResolver`

**File:** `src/config/proxmox/storage.rs`

**What to implement:**

Add a `StorageResolver` struct and its `resolve` method to the bottom of `storage.rs`
(same file as `ProxmoxStorageConf` — no new file needed).

```rust
use crate::config::proxmox::error::ProxmoxImportError;

pub struct StorageResolver<'a> {
    storage_conf: &'a ProxmoxStorageConf,
    vmid: u32,
}

impl<'a> StorageResolver<'a> {
    pub fn new(storage_conf: &'a ProxmoxStorageConf, vmid: u32) -> Self {
        StorageResolver { storage_conf, vmid }
    }

    /// Resolve a Proxmox `pool:volume` reference to an absolute host path.
    ///
    /// Returns `Err(ProxmoxImportError::InvalidVolumeRef)` if `pool_volume` has
    /// no `:` separator (e.g. the caller forgot to skip `"none"`).
    pub fn resolve(&self, pool_volume: &str) -> Result<String, ProxmoxImportError> {
        let (pool_name, volume) = pool_volume
            .split_once(':')
            .ok_or_else(|| ProxmoxImportError::InvalidVolumeRef {
                raw: pool_volume.to_string(),
            })?;

        let entry = self.storage_conf.entries.get(pool_name)
            .ok_or_else(|| ProxmoxImportError::UnknownStorage {
                pool: pool_name.to_string(),
            })?;

        match &entry.storage_type {
            StorageType::LvmThin => {
                let vgname = entry.properties.get("vgname")
                    .ok_or_else(|| ProxmoxImportError::MissingStorageProperty {
                        pool: pool_name.to_string(),
                        property: "vgname".to_string(),
                    })?;
                Ok(format!("/dev/{}/{}", vgname, volume))
            }
            StorageType::Dir => {
                let path = entry.properties.get("path")
                    .ok_or_else(|| ProxmoxImportError::MissingStorageProperty {
                        pool: pool_name.to_string(),
                        property: "path".to_string(),
                    })?;
                Ok(format!("{}/images/{}/{}", path, self.vmid, volume))
            }
            StorageType::Lvm => {
                // LVM (non-thin): same path convention as LvmThin
                let vgname = entry.properties.get("vgname")
                    .ok_or_else(|| ProxmoxImportError::MissingStorageProperty {
                        pool: pool_name.to_string(),
                        property: "vgname".to_string(),
                    })?;
                Ok(format!("/dev/{}/{}", vgname, volume))
            }
            StorageType::Unknown(t) => {
                Err(ProxmoxImportError::UnsupportedStorageType {
                    pool: pool_name.to_string(),
                    storage_type: t.clone(),
                })
            }
        }
    }
}
```

Add `pub use storage::StorageResolver;` to `src/config/proxmox.rs` so the resolver is
accessible from outside the module.

**Verification:** Add unit tests at the bottom of `storage.rs` inside `#[cfg(test)]`:

```rust
#[test]
fn test_storage_resolver_lvmthin() {
    use std::str::FromStr;
    let storage: ProxmoxStorageConf =
        "lvmthin: vm1-pool\n\tvgname vm1\n\tthinpool pool\n\tcontent images,rootdir\n"
        .parse().unwrap();
    let resolver = StorageResolver::new(&storage, 108);
    assert_eq!(
        resolver.resolve("vm1-pool:vm-108-boot").unwrap(),
        "/dev/vm1/vm-108-boot"
    );
}

#[test]
fn test_storage_resolver_dir() {
    use std::str::FromStr;
    let storage: ProxmoxStorageConf =
        "dir: local\n\tpath /var/lib/vz\n\tcontent iso,vztmpl\n"
        .parse().unwrap();
    let resolver = StorageResolver::new(&storage, 108);
    assert_eq!(
        resolver.resolve("local:iso/virtio-win-0.1.248.iso").unwrap(),
        "/var/lib/vz/images/108/iso/virtio-win-0.1.248.iso"
    );
}

#[test]
fn test_storage_resolver_invalid_volume_ref() {
    use std::str::FromStr;
    let storage = ProxmoxStorageConf::default();
    let resolver = StorageResolver::new(&storage, 108);
    let err = resolver.resolve("none").unwrap_err();
    assert!(matches!(err, crate::config::proxmox::error::ProxmoxImportError::InvalidVolumeRef { .. }));
}

#[test]
fn test_storage_resolver_unknown_pool() {
    use std::str::FromStr;
    let storage = ProxmoxStorageConf::default();
    let resolver = StorageResolver::new(&storage, 108);
    let err = resolver.resolve("ghost-pool:some-volume").unwrap_err();
    assert!(matches!(err, crate::config::proxmox::error::ProxmoxImportError::UnknownStorage { .. }));
}
```

Run with `cargo test test_storage_resolver_`.

---

### Task 04-03-B — Wire `StorageResolver` into `into_runtime()`

**File:** `src/config/proxmox/importer.rs`

**What to implement:**

Construct a `StorageResolver` at the top of `into_runtime()`:

```rust
let resolver = StorageResolver::new(&self.storage_conf, self.vmid);
```

Replace every place that uses `disk_conf.volume.clone()` as a resource string with a
`resolver.resolve(&disk_conf.volume)?` call.  Apply to:

- **scsi** disks: `resolver.resolve(&disk_conf.volume)?`
- **sata** disks: `resolver.resolve(&disk_conf.volume)?`
- **ide** disks: (already skips entries without `:`, so all remaining volumes are safe to resolve) `resolver.resolve(&disk_conf.volume)?`
- **virtio** disks: `resolver.resolve(&disk_conf.volume)?`
- **efidisk**: `storage_volume = resolver.resolve(&conf.volume)?`
- **tpmstate**: `storage_volume = resolver.resolve(&conf.volume)?`

The ide "none" skip guard from Task 04-01-C (`if !disk_conf.volume.contains(':') { continue; }`)
MUST remain in place — it prevents `"none"` from ever reaching the resolver.

Add to imports:

```rust
use crate::config::proxmox::storage::StorageResolver;
```

**Verification:** `cargo test test_storage_resolver_` passes (existing tests from 04-03-A).
`cargo check` exits 0.

**Commit manually with:**
```
feat(proxmox-import/04-03): StorageResolver + wire into into_runtime
```

---

## Plan 04-04 — Integration test

**Wave 4 — depends on Plans 04-01, 04-02, and 04-03.**

Write an end-to-end test that imports `input/felucia/108.conf` + `input/felucia/storage.cfg`
with `vmid = 108` and asserts the full converted `Runtime` is correct.

---

### Task 04-04-A — Write `tests/proxmox_import.rs`

**File:** `tests/proxmox_import.rs`

**What to implement:**

```rust
use std::str::FromStr;
use ezkvm::config::proxmox::{ProxmoxImporter, ProxmoxStorageConf, ProxmoxVmConf};
use ezkvm::runtime::{AudioDevice, Chipset, EfiDisk, HostPci, PcieBusDeviceKind, RawArgs, TpmState};

fn load_felucia() -> (ProxmoxVmConf, ProxmoxStorageConf) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = std::path::Path::new(&manifest);

    let vm_raw = std::fs::read_to_string(base.join("input/felucia/108.conf"))
        .expect("cannot read 108.conf");
    let storage_raw = std::fs::read_to_string(base.join("input/felucia/storage.cfg"))
        .expect("cannot read storage.cfg");

    let vm_conf = ProxmoxVmConf::from_str(&vm_raw).unwrap();
    let storage_conf = ProxmoxStorageConf::from_str(&storage_raw).unwrap();
    (vm_conf, storage_conf)
}

#[test]
fn test_proxmox_import_felucia_108() {
    let (vm_conf, storage_conf) = load_felucia();
    let importer = ProxmoxImporter::new(vm_conf, storage_conf, 108);
    let runtime = importer.into_runtime()
        .expect("into_runtime() must succeed for felucia/108.conf");

    // ── EfiDisk ─────────────────────────────────────────────────────────────
    let efidisk = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<EfiDisk>())
        .expect("EfiDisk not found");
    assert_eq!(efidisk.logical_size(), "4M");
    assert_eq!(efidisk.efitype().as_deref(), Some("4m"));
    assert_eq!(*efidisk.block_device_size_bytes(), None::<u64>);
    // EfiDisk storage_volume must be resolved to a host path, not a pool reference
    assert_eq!(efidisk.storage_volume(), "/dev/vm1/vm-108-efidisk");

    // ── TpmState ────────────────────────────────────────────────────────────
    let tpm = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<TpmState>())
        .expect("TpmState not found");
    assert_eq!(tpm.version(), "v2.0");
    assert_eq!(tpm.storage_volume(), "/dev/vm1/vm-108-tpmstate");

    // ── HostPci ─────────────────────────────────────────────────────────────
    let chipset = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<Chipset>())
        .expect("Chipset not found");
    let q35 = match chipset { Chipset::Q35(q) => q, _ => panic!("expected Q35") };

    let host_pci = q35.pcie_bus().values()
        .find(|dev| dev.device_kind() == PcieBusDeviceKind::HostPci)
        .expect("HostPci not on PCIe bus")
        .as_any()
        .downcast_ref::<HostPci>()
        .unwrap();

    assert_eq!(host_pci.base_bdf(), "0000:03:00");
    assert_eq!(host_pci.functions(), &[0u8, 1]);   // x-vga=1 → multi-function
    assert!(*host_pci.x_vga());

    // ── AudioDevice ─────────────────────────────────────────────────────────
    let audio = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<AudioDevice>())
        .expect("AudioDevice not found");
    assert_eq!(audio.device_type(), "ich9-intel-hda");
    assert_eq!(audio.driver(), "spice");

    // ── RawArgs ─────────────────────────────────────────────────────────────
    let raw = runtime.root_devices().iter()
        .find_map(|d| d.as_any().downcast_ref::<RawArgs>())
        .expect("RawArgs not found");
    assert!(raw.0.contains("-spice"), "raw args must contain spice config");

    // ── scsi0 resolved host path ─────────────────────────────────────────────
    // scsi0: vm1-pool:vm-108-boot → lvmthin → /dev/vm1/vm-108-boot
    use ezkvm::runtime::{PvScsi, PcieBusDeviceKind as PKind, ScsiAddress};
    let pvscsi = q35.pcie_bus().values()
        .find(|dev| dev.device_kind() == PKind::PvScsi)
        .expect("PvScsi not on PCIe bus")
        .as_any()
        .downcast_ref::<PvScsi>()
        .unwrap();

    let boot_disk_key = ScsiAddress::new(0, 0);
    let boot_disk = pvscsi.scsi_bus().get(&boot_disk_key)
        .expect("scsi0 (target=0, lun=0) not found in PvScsi");

    // Access resource via Ssd (scsi0 has ssd=1 in 108.conf)
    use ezkvm::runtime::Ssd;
    let ssd = boot_disk.as_any().downcast_ref::<Ssd>()
        .expect("scsi0 should be Ssd (ssd=1 in conf)");
    assert_eq!(ssd.resource, "/dev/vm1/vm-108-boot");
}
```

**Notes on imports:** `PvScsi` and `ScsiAddress` must be re-exported from `ezkvm::runtime`
for the test to compile.  Check `src/runtime.rs` — `PvScsi` is already re-exported via
`pub use devices::{..., PvScsi, ...}`.  `ScsiAddress` is exported via
`pub use scsi::{ScsiAddress, ScsiDevice}`.  If either is missing from `src/runtime.rs`,
add it.

The `Ssd::resource` field is `pub` (added in Task 04-01-A) so the direct field access works
without a getter.  If `derive_getters` is later added to `Ssd`, use `.resource()` instead.

**Verification:** `cargo test test_proxmox_import_felucia_108` passes.

**Commit manually with:**
```
feat(proxmox-import/04-04): integration test for felucia/108 full import
```

---

## Source coverage checklist

All items from the phase goal are mapped:

| Item | Covered by |
|------|-----------|
| `ProxmoxImporter` struct + `into_runtime()` | 04-01-B, 04-01-C |
| Memory conversion | 04-01-C |
| Q35 chipset (seven v1 root-device types: Memory, Chipset, EfiDisk, TpmState, AudioDevice, RawArgs + SpiceDisplay stub) | 04-01-C, 04-02-A/C |
| Scsi disks via PvScsi | 04-01-C |
| Sata / IDE / virtio disks | 04-01-C |
| `resource: String` on Hdd/Ssd/Cdrom | 04-01-A |
| EfiDisk (logical_size, block_device_size_bytes=None) | 04-02-A |
| TpmState | 04-02-A |
| HostPci multi-function (x-vga=1 → [0,1]) | 04-02-B |
| AudioDevice | 04-02-C |
| RawArgs verbatim passthrough | 04-02-C |
| `StorageResolver` (lvmthin + dir) | 04-03-A |
| `"none"` cdrom skip | 04-01-C (guard) + 04-03-B (retained) |
| Resolver wired for all disk types + efidisk + tpmstate | 04-03-B |
| `ProxmoxImportError` descriptive variants | 04-01-B |
| `ProxmoxVmConf` re-exported | 04-01-B |
| Integration test (felucia/108) | 04-04-A |

**SpiceDisplay note:** `RootDeviceKind::SpiceDisplay` is a v1 device type but `ProxmoxVmConf`
does not carry a `spice:` key — Proxmox encodes Spice as part of `args`.  The importer does
not attempt to parse `SpiceDisplay` out of `RawArgs`; that is deferred to a later phase.
All other six v1 root-device types are fully handled.
