# Architecture Patterns

**Project:** ezkvm — QEMU/Proxmox VM configuration conversion tool
**Researched:** 2026-07-22
**Source:** Codebase analysis (direct read of src/, input/ corpus, planning docs)
**Confidence:** HIGH — all findings are from direct codebase inspection

---

## Recommended Architecture

The overall conversion pipeline forms a hub-and-spoke design where **Runtime is the canonical in-memory hub** and all external formats are spokes that convert to/from it. This is already the pattern established by the ezkvm layer; it must be extended consistently to Proxmox and QEMU layers.

```
Proxmox .conf + storage.cfg
         ↕  (ProxmoxFileParser / ProxmoxFileEmitter)
   ProxmoxVmConf + ProxmoxStorageConf
         ↕  (TryFrom impls in src/config/proxmox/runtime/)
                                                          ┌──────────────────────┐
                                      ┌── Runtime ────────┤  canonical in-memory │
                                      │  (hub)            │  Arc<dyn Trait> tree  │
                                      └──────────────────-└──────────────────────┘
         ↕  (TryFrom impls in src/config/ezkvm/runtime/)
     ConfigSchema (ezkvm)
         ↕  (EzkvmFileParser / EzkvmFileBuilder)
       ezkvm YAML file

Runtime ──→ QemuCommandLine  (one-way, no parse-back needed for now)
               ↓  (Display)
         QEMU cmdline string
```

---

## Component Boundaries

| Component | Responsibility | Location | Communicates With |
|-----------|---------------|----------|-------------------|
| **Runtime** | In-memory VM topology; trait-based polymorphism | `src/runtime/` | ← all schema layers via `TryFrom` |
| **ezkvm Schema** | YAML-serializable config structs | `src/config/ezkvm/schema/` | ↔ Runtime via `src/config/ezkvm/runtime/` |
| **ezkvm File I/O** | Parse/emit ezkvm YAML | `src/config/ezkvm/file/` | ↔ ezkvm Schema |
| **Proxmox Schema** | Typed representation of Proxmox `.conf` key-value format | `src/config/proxmox/schema/` *(new)* | ↔ Runtime via `src/config/proxmox/runtime/` |
| **Proxmox File I/O** | Parse/emit Proxmox `.conf` and `storage.cfg` format | `src/config/proxmox/file/` *(new)* | ↔ Proxmox Schema |
| **QEMU Schema** | Typed QEMU commandline argument list | `src/config/qemu/schema/` *(new)* | ← Runtime via `src/config/qemu/runtime/` |
| **QEMU File I/O** | Emit/parse QEMU cmdline string | `src/config/qemu/file/` *(new)* | ↔ QEMU Schema |
| **serde_yaml** | Custom YAML serde adapter (saphyr) | `src/serde_yaml/` | ← ezkvm File I/O only |

**Dependency rule:** Runtime has zero upward dependencies. Schema layers depend on Runtime for conversions but not for their struct definitions. File I/O layers depend only on their schema layer and serde infrastructure.

---

## Current State vs. Target State

### What exists (✓)
- Runtime layer: Memory, Q35/I440FX chipsets, PvScsi, VirtioNet, basic storage (SSD/HDD/CDROM on SATA/SCSI/IDE/PCI), GenericPciDevice, GenericUsbDevice
- ezkvm Schema: Comprehensive — covers all device types including EFI, TPM, PCI passthrough, ivshmem, audio, SPICE, SMBIOS, vmgenid
- ezkvm File I/O: Complete (FromStr + Display + compact YAML styling)
- Runtime ↔ ezkvm Schema conversions: Working for existing devices, incomplete for advanced types
- Proxmox stub: `ProxmoxVmSchema` (empty struct), handler pattern skeleton in `src/config/proxmox.rs`
- QEMU stub: `QemuSchema` (empty struct), handler pattern skeleton in `src/config/qemu.rs`

### What is missing (→ to build)
1. Runtime completeness — missing types for: EFI/OVMF disk, TPM, PCI passthrough (hostpci), ivshmem shared memory, audio (ich9-intel-hda), SPICE display, SMBIOS, vmgenid, NUMA, serial ports, raw `args` passthrough, `efidisk`, `balloon`
2. Proxmox Schema structs — typed key-value representation of `.conf` format
3. Proxmox parser — `.conf` + `storage.cfg` → Proxmox Schema
4. Proxmox → Runtime conversion
5. Runtime → Proxmox conversion (for round-trip export)
6. Proxmox emitter — Proxmox Schema → `.conf` string
7. QEMU Schema — `QemuCommandLine` as typed argument segments
8. Runtime → QEMU conversion
9. QEMU emitter — `QemuCommandLine` → cmdline string

---

## Data Flow

### Path 1: Proxmox .conf → Runtime (import)

```
1. ProxmoxFileParser::parse(conf_path, storage_cfg_path)
   → reads file bytes, splits on [snapshot_name] sections
   → active config = lines before first [snapshot] header
   → parse each "key: value" line → HashMap<String, String>

2. ProxmoxConfParser::build(raw_map) → ProxmoxVmConf
   → dispatch on key prefix: "scsi\d+" → ProxmoxDiskConf
   → dispatch on key prefix: "net\d+" → ProxmoxNetConf
   → dispatch on key prefix: "hostpci\d+" → ProxmoxHostPciConf
   → etc.
   → value format: "storage:volume,opt1=val,opt2=val"

3. ProxmoxStorageConfParser::parse(storage_cfg_path) → ProxmoxStorageConf
   → parse stanza format (type: id, then indented key: val)

4. Runtime::try_from((ProxmoxVmConf, ProxmoxStorageConf))
   → RuntimeBuilder::new()
   → add Memory from conf.memory
   → build Q35ChipsetBuilder from conf.machine, conf.scsihw
   → add storage devices, PCI passthrough, USB, network, EFI, TPM, etc.
   → resolve "vm1-pool:vm-108-boot" via StorageConf for host-side paths
```

### Path 2: Runtime → Proxmox .conf (export)

```
1. ProxmoxVmConf::try_from(Runtime)
   → walk root_devices, downcast each type
   → Memory → conf.memory = size
   → Chipset::Q35 → conf.machine = "pc-q35-{version}", conf.scsihw = "pvscsi"
   → PvScsi on PCIe → scsi_hw_addr + scsi disk entries
   → VirtioNet → net_devices[n]
   → EfiDisk → efidisk0
   → TpmState → tpmstate0
   → HostPci → hostpci_devices[n]
   → etc.

2. ProxmoxFileEmitter::emit(ProxmoxVmConf) → String
   → emit keys in Proxmox-canonical order
   → format values as "storage:volume,opt1=val,opt2=val"
```

### Path 3: Runtime → QEMU cmdline (generate)

```
1. QemuCommandLine::try_from((Runtime, QemuContext))
   → QemuContext carries: vm_name, socket_paths, storage_paths, host-specific config
   → emitter walks runtime components, accumulates argument segments

2. QemuArgs segmented builder:
   - machine_args:  -name, -machine, -cpu, -m, -smp, -readconfig
   - firmware_args: -drive if=pflash (EFI firmware + nvram)
   - device_groups: per-device (-drive + -device pairs, -netdev + -device, etc.)
   - chardev_args:  -chardev, -tpmdev, -mon
   - object_args:   -object (memory backends)
   - misc_args:     -boot, -rtc, -enable-kvm, -nodefaults, -vga, -nographic

3. QemuCommandLine::to_string() → join all segments in correct order
```

### Cross-reference tracking in QEMU generation
QEMU cmdline has named cross-references (`drive=drive-scsi0`, `netdev=net0`, `chardev=tpmchar`). These must be managed with an **ID allocator** that:
- Assigns stable IDs based on device type + index
- Ensures `-drive` / `-netdev` / `-chardev` args precede their `-device` consumers
- Segment-based approach (collect all drives first, then devices) handles ordering automatically

---

## Patterns to Follow

### Pattern 1: Key-indexed device map (Proxmox Schema)

Proxmox `.conf` uses `key\d+` notation for indexed devices. The schema struct should use ordered maps:

```rust
pub struct ProxmoxVmConf {
    // scalar fields
    pub memory: Option<u64>,
    pub machine: Option<String>,
    pub name: Option<String>,
    pub bios: Option<String>,         // "ovmf" | "seabios"
    pub cpu: Option<String>,
    pub cores: Option<u32>,
    pub sockets: Option<u32>,
    pub scsihw: Option<String>,       // "pvscsi" | "virtio-scsi-pci" | "lsi"
    pub boot: Option<String>,
    pub agent: Option<u8>,
    pub numa: Option<bool>,
    pub ostype: Option<String>,
    pub smbios1: Option<String>,
    pub vmgenid: Option<String>,
    pub args: Option<String>,         // raw QEMU args passthrough
    pub vga: Option<String>,
    pub tablet: Option<u8>,
    pub balloon: Option<u64>,

    // indexed device maps
    pub scsi:   BTreeMap<u8, ProxmoxDiskConf>,
    pub sata:   BTreeMap<u8, ProxmoxDiskConf>,
    pub ide:    BTreeMap<u8, ProxmoxDiskConf>,
    pub virtio: BTreeMap<u8, ProxmoxDiskConf>,
    pub net:    BTreeMap<u8, ProxmoxNetConf>,
    pub hostpci: BTreeMap<u8, ProxmoxHostPciConf>,
    pub usb:    BTreeMap<u8, ProxmoxUsbConf>,
    pub audio:  BTreeMap<u8, ProxmoxAudioConf>,
    pub numa_nodes: BTreeMap<u8, ProxmoxNumaConf>,

    // special single-indexed or typed entries
    pub efidisk: Option<(u8, ProxmoxDiskConf)>,
    pub tpmstate: Option<(u8, ProxmoxTpmConf)>,

    // snapshots: kept as raw key-value maps, not fully parsed
    pub snapshots: BTreeMap<String, HashMap<String, String>>,
}
```

### Pattern 2: Value tokenizer (Proxmox parser)

Proxmox values follow `positional,key=val,key=val` grammar:

```rust
fn parse_proxmox_value(raw: &str) -> (Option<String>, HashMap<String, String>) {
    let mut parts = raw.splitn(2, ',');
    let first = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("");
    
    let positional = if first.contains('=') { None } else { Some(first.to_string()) };
    let opts = rest.split(',')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    (positional, opts)
}
```

### Pattern 3: Segment-based QEMU arg builder

Avoids ordering problems by grouping argument categories separately:

```rust
#[derive(Default)]
pub struct QemuCommandLine {
    pub machine:   Vec<String>,   // -name, -machine, -cpu, -m, -smp
    pub readconfig: Vec<String>,  // -readconfig
    pub firmware:  Vec<String>,   // -drive if=pflash
    pub drives:    Vec<String>,   // -drive if=none
    pub netdevs:   Vec<String>,   // -netdev
    pub chardevs:  Vec<String>,   // -chardev
    pub tpm:       Vec<String>,   // -tpmdev, -chardev for tpm
    pub objects:   Vec<String>,   // -object
    pub devices:   Vec<String>,   // -device
    pub misc:      Vec<String>,   // -boot, -rtc, -enable-kvm, -nodefaults, etc.
}

impl Display for QemuCommandLine {
    fn fmt(...) {
        // emit: machine → readconfig → firmware → drives → netdevs →
        //       chardevs → tpm → objects → devices → misc
    }
}
```

### Pattern 4: Snapshot section handling

The Proxmox `.conf` parser MUST split on section headers before any key parsing:

```rust
fn split_sections(input: &str) -> (Vec<(String, String)>, BTreeMap<String, Vec<(String, String)>>) {
    let mut active = Vec::new();
    let mut snapshots = BTreeMap::new();
    let mut current_section: Option<String> = None;
    
    for line in input.lines() {
        if let Some(snap_name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current_section = Some(snap_name.to_string());
            snapshots.insert(snap_name.to_string(), Vec::new());
        } else if let Some((key, val)) = line.split_once(": ") {
            if let Some(ref sec) = current_section {
                snapshots.get_mut(sec).unwrap().push((key.to_string(), val.to_string()));
            } else {
                active.push((key.to_string(), val.to_string()));
            }
        }
    }
    (active, snapshots)
}
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Direct Runtime → QEMU without segment grouping

**What:** Emitting QEMU args inline as each device is processed (one pass, ad-hoc order).
**Why bad:** QEMU requires specific ordering: `-netdev` before `-device virtio-net-pci`, `-drive` before `-device scsi-hd`. Single-pass inline emission produces incorrect ordering without complex look-ahead.
**Instead:** Use the `QemuCommandLine` segment builder — collect into categories, emit in fixed category order.

### Anti-Pattern 2: Parsing Proxmox .conf without section-splitting first

**What:** Naively parsing all lines as `key: value` pairs.
**Why bad:** The file contains snapshot sections (`[snapshot_name]`) with identical key names. Merging active config and snapshot config produces garbage.
**Instead:** Always split into active section and snapshot sections first; parse each independently.

### Anti-Pattern 3: Expanding the existing `ProxmoxVmSchema` empty struct inline

**What:** Adding all Proxmox fields into the existing single `src/config/proxmox.rs` file.
**Why bad:** The file already mixes schema, builder, and conversion concerns. It will become unmaintainable. The ezkvm layer's success comes from its clear subdirectory separation.
**Instead:** Expand `src/config/proxmox/` into the same subdirectory layout as `src/config/ezkvm/` — `schema/`, `file/`, `runtime/` subdirs.

### Anti-Pattern 4: Completing Runtime before any Proxmox parsing

**What:** Building all missing Runtime types (EFI, TPM, hostpci, etc.) before touching the Proxmox parser.
**Why bad:** You won't know the correct Runtime API for each device type until you see what the Proxmox parser needs to produce. Building in isolation risks mismatches.
**Instead:** Interleave — build the Proxmox parser enough to produce typed `ProxmoxVmConf`, then drive Runtime completeness from what the conversion code needs.

### Anti-Pattern 5: Treating `args` as structured data

**What:** Parsing the raw `args` Proxmox field into individual QEMU flags.
**Why bad:** The `args` value is arbitrary QEMU commandline fragments; parsing it is fragile and error-prone.
**Instead:** Treat `args` as an opaque passthrough in both Runtime (a `RawArgs(String)` root device or field on a future `RuntimeConfig`) and QEMU emission (append verbatim at end of cmdline).

---

## Suggested Directory Structure (new modules)

```
src/config/proxmox/           ← expand from proxmox.rs stub
├── proxmox.rs                ← re-exports (mirrors ezkvm/ezkvm.rs)
├── schema/
│   ├── conf.rs               ← ProxmoxVmConf (main struct)
│   ├── disk.rs               ← ProxmoxDiskConf
│   ├── net.rs                ← ProxmoxNetConf
│   ├── hostpci.rs            ← ProxmoxHostPciConf
│   ├── usb.rs                ← ProxmoxUsbConf
│   ├── audio.rs              ← ProxmoxAudioConf
│   ├── tpm.rs                ← ProxmoxTpmConf
│   └── storage.rs            ← ProxmoxStorageConf (storage.cfg)
├── file/
│   ├── parser.rs             ← FromStr for ProxmoxVmConf
│   ├── builder.rs            ← Display for ProxmoxVmConf
│   └── storage_parser.rs     ← FromStr for ProxmoxStorageConf
└── runtime/
    ├── builder.rs            ← ProxmoxVmConf → Runtime (TryFrom)
    └── parser.rs             ← Runtime → ProxmoxVmConf (TryFrom)

src/config/qemu/              ← expand from qemu.rs stub
├── qemu.rs                   ← re-exports
├── schema/
│   └── cmdline.rs            ← QemuCommandLine (segmented arg builder)
├── file/
│   ├── builder.rs            ← Display for QemuCommandLine
│   └── parser.rs             ← FromStr for QemuCommandLine (for .qemu.cmd import)
└── runtime/
    └── builder.rs            ← Runtime → QemuCommandLine (TryFrom)
```

---

## Suggested Build Order

Dependencies flow upward; each phase unblocks the next:

```
Phase 1: Runtime Completeness
  ├─ Add missing Runtime types: EfiDisk, TpmState, HostPci, Ivshmem, Audio
  ├─ Add to Q35ChipsetBuilder or as new RootDevice types as appropriate
  └─ Prerequisite for: ALL conversion phases

Phase 2: Proxmox Schema structs
  ├─ ProxmoxVmConf + sub-structs (disk, net, hostpci, usb, audio, tpm)
  ├─ ProxmoxStorageConf
  └─ Prerequisite for: Proxmox File I/O, Proxmox ↔ Runtime conversions

Phase 3: Proxmox File I/O (Parser)
  ├─ FromStr for ProxmoxVmConf (parse .conf → ProxmoxVmConf)
  ├─ FromStr for ProxmoxStorageConf (parse storage.cfg)
  └─ Prerequisite for: Proxmox → Runtime conversion

Phase 4: Proxmox → Runtime Conversion
  ├─ TryFrom<(ProxmoxVmConf, ProxmoxStorageConf)> for Runtime
  └─ Prerequisite for: end-to-end import pipeline

Phase 5: Runtime → Proxmox Conversion + Emitter
  ├─ TryFrom<Runtime> for ProxmoxVmConf
  ├─ Display for ProxmoxVmConf (emit .conf format)
  └─ Completes: Proxmox round-trip

Phase 6: QEMU Schema + Emission
  ├─ QemuCommandLine segmented struct
  ├─ TryFrom<(Runtime, QemuContext)> for QemuCommandLine
  ├─ Display for QemuCommandLine
  └─ Completes: Runtime → QEMU cmdline generation

Phase 7: QEMU .qemu.cmd Import (optional)
  ├─ FromStr for QemuCommandLine
  ├─ TryFrom<QemuCommandLine> for Runtime
  └─ Completes: full round-trip import from QEMU cmdline files
```

**Rationale for this order:**
- Phase 1 (Runtime) must come first because all conversion logic targets Runtime types. Trying to write `TryFrom<ProxmoxVmConf> for Runtime` before `EfiDisk` exists means rewriting the conversion code later.
- Phase 2 (Schema structs) before Phase 3 (File I/O) because the parser needs to know what types it's populating.
- Phases 3+4 before 5 because the parser validates whether the schema design is correct — often the schema needs adjustment based on parse experience.
- Phase 6 (QEMU) can proceed after Phase 1 since QEMU emission only needs Runtime, not Proxmox.
- Phase 7 is deferred since `.qemu.cmd` import is lower priority than export.

---

## Scalability Considerations

| Concern | Now (felucia/108) | At full corpus coverage |
|---------|-------------------|------------------------|
| Key dispatch in Proxmox parser | ~25 distinct keys + indexed | ~50 keys; regex-based dispatch scales fine |
| Storage path resolution | Direct LVM path | May need storage plugin abstraction |
| QEMU arg ordering | Fixed category segments | Categories may need subcategory ordering |
| Snapshot handling | Parse but ignore | May need to represent in Runtime for round-trip |

---

## Key Proxmox Format Facts

From analysis of `input/felucia/108.conf` and the full corpus:

| Key Pattern | Value Format | Notes |
|-------------|-------------|-------|
| `scsi\d+` | `storage:vol,opt=val,...` | `ssd=1` signals SSD type; `discard=on` |
| `ide\d+` | `storage:vol,media=cdrom` or `none,media=cdrom` | `none` = empty cdrom |
| `sata\d+` | `storage:vol,opt=val,...` | same as scsi |
| `net\d+` | `virtio=MAC,bridge=br,firewall=1` | driver type is key prefix |
| `hostpci\d+` | `0000:03:00,pcie=1,x-vga=1` | BDF notation; multi-function from `.0,.1` scan |
| `usb\d+` | `host=1-2.2` or `host=vendorid:productid` | |
| `audio\d+` | `device=ich9-intel-hda,driver=spice` | |
| `efidisk\d+` | `storage:vol,efitype=4m,ms-cert=2023,...` | |
| `tpmstate\d+` | `storage:vol,size=4M,version=v2.0` | |
| `machine` | `pc-q35-8.1` | includes version suffix |
| `bios` | `ovmf` or `seabios` | maps to EFI vs BIOS boot |
| `scsihw` | `pvscsi` or `virtio-scsi-pci` | affects SCSI controller type |
| `args` | raw QEMU flags | passthrough; do not parse |
| `[snapshot_name]` | section header | must be split off before key parsing |

---

## Sources

- Direct analysis of `src/runtime/`, `src/config/ezkvm/`, `src/config/proxmox.rs`, `src/config/qemu.rs`
- Input corpus: `input/felucia/108.conf`, `108.qemu.cmd`, `108.ezkvm.qemu.cmd`, `storage.cfg`
- Planning docs: `.planning/PROJECT.md`, `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/STRUCTURE.md`

---

*Architecture analysis: 2026-07-22*
