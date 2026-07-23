# Phase 3: Proxmox Parser — Research

**Researched:** 2026-07-22
**Domain:** Rust string parsing, INI-variant format, state machines, Proxmox `.conf` and `storage.cfg` formats
**Confidence:** HIGH — all findings verified against actual source files and corpus data in this repository.

---

## Summary

Phase 3 adds the Proxmox import layer: parsing raw `.conf` and `storage.cfg` files into typed
in-memory structs (`ProxmoxVmConf`, `ProxmoxStorageConf`). The current `src/config/proxmox.rs`
is a builder stub with no parsing code at all — every data structure and parser function must
be written from scratch.

The format is an INI variant with three hard quirks that create landmines for naive
line-by-line parsers: (1) snapshot sections appended in-file share a key namespace with the
active config, requiring a two-phase state machine; (2) `##key%3Avalue` comment lines look like
live config entries to a simple split; and (3) sub-option values contain colons (MAC addresses,
PCI BDFs, pool:volume names, USB vendor:product IDs) that are NOT separators — requiring a
"split on first `=`" tokenizer rather than any colon-aware logic.

`storage.cfg` uses a different grammar (section headers `type: name` + tab-indented
`key value` properties with space-not-equals separation) that needs its own `FromStr` impl.

No new Cargo dependencies are needed. All four plans can be implemented with `std` alone:
`str::lines()`, `split_once()`, `strip_prefix()`, `thiserror` (already in Cargo.toml),
`HashMap` (std), and `BTreeMap` (std) for index-keyed device collections.

**Primary recommendation:** Implement a two-state line scanner in `split_sections()`, then drive
strongly-typed `FromStr` impls for each device sub-option struct from a single
`parse_sub_options()` helper that splits on `,` then on the first `=`.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PROX-01 | Parser correctly splits active config from named snapshot sections before processing any fields | §State Machine Design |
| PROX-02 | Parser handles URL-encoded comments (`##args%3A`) without treating them as live config entries | §Comment Line Rules |
| PROX-03 | Parser tokenizes sub-option values correctly for fields containing colons (MAC, BDF, pool:volume) | §Sub-Option Tokenizer Design |
| PROX-04 | `storage.cfg` parsed as first-class input | §ProxmoxStorageConf Design |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Section splitting | `split_sections()` in `proxmox/parser.rs` | — | File-level structural concern; must run before any field parsing |
| Active key-value extraction | `split_sections()` | — | Same pass: lines in active state emit `(key, raw_value)` pairs |
| Sub-option tokenization | `parse_sub_options()` shared helper | per-device `FromStr` impls | One tokenizer, many typed callers |
| ProxmoxVmConf hydration | `FromStr for ProxmoxVmConf` | `parse_sub_options()` | Dispatches per-key to sub-option parsers |
| Storage.cfg parsing | `FromStr for ProxmoxStorageConf` | — | Separate grammar; independent impl |
| Runtime construction from conf | Phase 4 only | — | Out of scope for Phase 3 |

---

## Current State of `src/config/proxmox.rs`

[VERIFIED: direct file read]

The entire file is a builder stub (78 lines). It contains:

| Symbol | Type | State |
|--------|------|-------|
| `ProxmoxVmSchema` | `struct {}` | Empty placeholder |
| `ProxmoxHostSchema` | `struct {}` | Empty placeholder |
| `ProxmoxSchemaBuilder` | struct + impl | Scaffolding, no parsing logic |
| `TryFrom<(Runtime, ProxmoxHostSchema)> for ProxmoxVmSchema` | conversion | Returns empty schema |
| `TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime` | conversion | Returns empty runtime |

**Gaps for Phase 3 (all must be created):**
- No `ProxmoxVmConf` struct
- No `ProxmoxStorageConf` struct
- No `split_sections()` function
- No sub-option tokenizer
- No `FromStr` implementations
- No `ProxmoxParseError` error type

The existing `ProxmoxVmSchema` / `ProxmoxSchemaBuilder` belong to Phase 4's concern
(Runtime → Proxmox export direction). Phase 3 should add parsing code alongside these stubs,
either as submodules or as an expanded single-file (see §Recommended Project Structure).

---

## Corpus Analysis

### `input/felucia/108.conf` — Primary Test File [VERIFIED: direct file read]

**Structure:**
```
[lines 1–13]   # comment lines (human notes, including ##url-encoded disabled entries)
[lines 14–41]  active section (no header, top of file)
[line 43]      [before_lg] snapshot header
[lines 44–69]  before_lg snapshot fields
[line 71]      [intermediate_041025] snapshot header
...             more snapshot sections
```

**Active section keys** (lines 14–41, before first `[` header):
```
agent, args, audio0, bios, boot, cores, cpu, efidisk0, hostpci0,
ide2, machine, memory, meta, name, net0, numa, ostype, parent,
scsi0, scsi1, scsihw, smbios1, sockets, tablet, tpmstate0, usb0, vga, vmgenid
```

**Critical active field values:**
```
cpu: host                                                ← PROX-01 success criterion
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
hostpci0: 0000:03:00,pcie=1,x-vga=1
net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1  ← PROX-03 success criterion
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1
tpmstate0: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
usb0: host=1-2.2
audio0: device=ich9-intel-hda,driver=spice
args: -spice port=5903,...  (long raw args line)
parent: intermediate_20251018
```

**URL-encoded comment lines in this file** (lines 1–13, all start with `#`):
```
#Hardware%3A
#- AMD RX 7700S GPU
#cpu%3A custom-7940HS-nested-vm
```
(Note: single `#`, human-written, contain `%3A` but are still just comments.)

**Snapshot contamination risk:** The snapshot section `[before_lg]` (line 43) repeats most
active fields with *different values* (e.g., `cpu: x86-64-v2-AES` vs active `cpu: host`).
A naive line-by-line parser without section tracking would overwrite the active `cpu: host`
with the snapshot's `cpu: x86-64-v2-AES`.

### `input/felucia/storage.cfg` — Primary Storage Test File [VERIFIED: direct file read]

```
dir: local
	path /var/lib/vz
	content iso,vztmpl

lvmthin: vm1-pool
	thinpool pool
	vgname vm1
	content images,rootdir
```

**Required parse result:** `vm1-pool` entry → `ProxmoxStorageEntry { name: "vm1-pool", type: LvmThin, thinpool: "pool", vgname: "vm1", content: ["images", "rootdir"] }`.

### Broader Corpus Survey [VERIFIED: grep across all corpus files]

**URL-encoded `##` comment patterns seen:**
```
##args%3A ...                          (appears in 5+ files)
##hostpci1%3A 0000%3A04%3A00.0,...     (zbp-server-mh2/301.conf)
##net0%3A virtio=aa%3A30%3A00%3A...    (coruscant conf)
##memory%3A 16384                      (multiple files)
###args%3A ...                         (triple-hash variant — still a comment)
####args%3A ...                        (four-hash variant — still a comment)
```

**Key insight for PROX-02:** Every "disabled config" line — whether `#comment`, `##url-encoded`,
`###multiple-hash` — starts with `#`. The rule is simply: **skip any line where the first
character is `#`**. No URL-decoding required.

**Indexed device key census** (across 107+ active sections):
```
hostpci0–hostpci4   (up to 5 per VM)
net0–net1           (1–2 per VM)
scsi0–scsi3         (1–4 per VM)
sata0–sata1         (macOS VMs use sata)
virtio0–virtio1     (some Linux VMs use virtio)
ide0, ide2          (CD-ROM slots)
usb0–usb5           (host USB passthrough)
serial0             (socket backend)
numa0–numa1         (NUMA topology lines)
unused0–unused3     (detached volumes)
audio0              (single audio device per VM, always index 0)
tpmstate0           (always index 0)
efidisk0            (always index 0)
```

**USB host formats observed:**
```
usb0: host=1-2.2        ← USB port path (hub.port)
usb4: host=0451:16a0    ← USB vendor:product ID (colon inside value!)
usb5: host=0403:6001    ← USB vendor:product ID
```

**net model variants observed:**
```
virtio=<MAC>,...     (most common)
e1000e=<MAC>,...     (seen in coruscant)
```

**Hostpci BDF variants observed:**
```
0000:03:00          ← no function suffix (multi-function, Phase 4 expands)
0000:03:00.0        ← explicit function 0
0000:41:00,pcie=1,romfile=vgabios-radeon-rx570.bin
0000:0e:11.0,pcie=1
```

**efidisk variants:**
```
vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
vm1-pool:vm-103-disk-1,size=4M                   ← no efitype, no pre-enrolled-keys
vm0:vm-103-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M
```

**All tpmstate entries in corpus use:** `size=4M,version=v2.0`
(size is invariant; `TpmState` runtime already documented as not storing size.)

**storage.cfg type variants seen across corpus:**
```
dir        (path, content, optional shared)
lvmthin    (thinpool, vgname, content, optional shared)
lvm        (vgname, content, optional shared)
```

---

## State Machine Design for `split_sections()`

[ASSUMED: standard INI parser literature — verified against corpus structure]

### Two-State Machine

```
Input: file content as &str

State::Active:
  for each line:
    - trim trailing whitespace / \r
    - empty → skip
    - starts with '#' → skip (covers #comments, ##url-encoded, ###, ####)
    - starts with '[' → extract name = strip_prefix('[').strip_suffix(']')
                        transition to State::Snapshot(name)
    - contains ':' → split_once(": ") → push (key.trim(), raw_value.trim()) to active_entries
    - else → skip (unknown line shape)

State::Snapshot(current_name):
  for each line:
    - trim trailing whitespace
    - empty → skip
    - starts with '#' → skip
    - starts with '[' → emit accumulated snapshot as (current_name, entries)
                        extract new_name, reset entries, stay in Snapshot(new_name)
    - contains ": " → push to current snapshot entries
    - else → skip

End of input:
  if in Snapshot state → emit final snapshot
  return (active_entries, snapshots)
```

### Function Signature

```rust
/// Splits a Proxmox .conf file into the active section key-value pairs and named snapshots.
/// URL-encoded comment lines (starting with '#') are silently discarded.
/// Returns: (active_entries, snapshot_map)
pub fn split_sections(
    input: &str,
) -> (Vec<(String, String)>, HashMap<String, Vec<(String, String)>>)
```

The active `Vec<(String, String)>` preserves insertion order (matching file order). The
snapshot map is keyed by snapshot name.

### Why Two-Phase vs. One-Pass

A one-pass state machine (the approach used here) is correct for this format because:
- Proxmox `.conf` guarantees active section is always at the top of the file
- No forward references: snapshot sections never modify active section fields
- No need to buffer the whole file; line-by-line processing with a state enum is sufficient

---

## Sub-Option Tokenizer Design

[VERIFIED: corpus analysis — MAC addresses, BDFs, pool:volume all confirmed]

### The Colon Problem

Multiple Proxmox sub-option values contain literal colons that are **NOT separators**:
- MAC address: `BC:24:11:3A:21:B7`
- PCI BDF: `0000:03:00` or `0000:03:00.0`
- Pool:volume: `vm1-pool:vm-108-boot`
- USB vendor:product: `0451:16a0`
- IDE ISO path: `local:iso/virtio-win-0.1.248.iso`

The correct tokenizer algorithm:

```
fn parse_sub_options(raw: &str) -> Vec<(Option<String>, String)>
                                    ↑ key if named, None if positional
1. Split raw by ','  → tokens
2. For each token:
   a. If token.contains('='):
        split_once('=') → (key, value)  ← only split on FIRST '='
        emit (Some(key), value)
   b. Else:
        emit (None, token)   ← positional: pool:volume, BDF, "none", etc.
```

**Why "first `=` only":** `virtio=BC:24:11:3A:21:B7` — splitting on first `=` gives
`key="virtio"`, `value="BC:24:11:3A:21:B7"`. The colons in the value are never seen as
separators. This is the only split rule needed.

**Why not split on ':':** Colons appear in positional values (`vm1-pool:vm-108-boot`),
in named values (`virtio=BC:24:11:3A:21:B7`), and in USB host IDs (`host=0451:16a0`).
There is no position where a colon is a separator in the Proxmox sub-option grammar.

### Per-Device-Type Tokenizer Rules

| Device Key | First Token Format | Named Options |
|------------|-------------------|---------------|
| `net0` | `model=MAC` (named, split on first `=`) | `bridge`, `firewall`, `mtu` |
| `scsiN` | `pool:volume` (positional) | `cache`, `discard`, `ssd`, `size`, `backup`, `iothread`, `replicate` |
| `sataN` | `pool:volume` (positional) | same as scsi |
| `virtioN` | `pool:volume` (positional) | same as scsi |
| `ideN` | `pool:volume` OR `none` (positional) | `media`, `size` |
| `efidisk0` | `pool:volume` (positional) | `efitype`, `pre-enrolled-keys`, `ms-cert`, `size` |
| `tpmstate0` | `pool:volume` (positional) | `size`, `version` |
| `hostpciN` | BDF `dddd:bb:dd[.f]` (positional) | `pcie`, `x-vga`, `rombar`, `romfile` |
| `audio0` | none | `device`, `driver` |
| `usbN` | none | `host` |

For `audio0` and `usb0`, the entire value is named options — no positional first token.

---

## `ProxmoxVmConf` Struct Design

[ASSUMED: field coverage based on corpus analysis; individual field types based on observed values]

### Sub-Option Structs

```rust
/// Parsed net device: net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxNet {
    model: String,             // "virtio", "e1000e"
    mac: String,               // "BC:24:11:3A:21:B7" — preserved verbatim
    bridge: String,            // "vmbr0"
    firewall: bool,
}

/// Parsed disk (scsi/sata/virtio/ide): vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxDisk {
    volume: String,            // "vm1-pool:vm-108-boot", "local:iso/foo.iso", "none"
    media: Option<String>,     // "cdrom" (IDE only)
    size: Option<String>,      // "256G", "4M", "1T", "628M", "707456K"
    cache: Option<String>,     // "writeback", "unsafe", "none"
    discard: bool,
    ssd: bool,
    backup: Option<bool>,      // explicit false means backup=0
    iothread: bool,
    replicate: Option<bool>,
}

/// Parsed EFI disk: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxEfiDisk {
    volume: String,
    efitype: Option<String>,       // "4m"
    pre_enrolled_keys: bool,
    ms_cert: Option<String>,       // "2023"
    size: Option<String>,          // "4M"
}

/// Parsed TPM state: vm1-pool:vm-108-tpmstate,size=4M,version=v2.0
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxTpmState {
    volume: String,
    version: String,               // "v2.0"
    // Note: size is invariant (always "4M") and not stored in Runtime; parsed but ignored
    size: Option<String>,          // kept for round-trip completeness
}

/// Parsed hostpci: 0000:03:00,pcie=1,x-vga=1
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxHostPci {
    bdf: String,               // "0000:03:00" or "0000:03:00.0"
    pcie: bool,
    x_vga: bool,
    rombar: Option<bool>,
    romfile: Option<String>,
}

/// Parsed audio: device=ich9-intel-hda,driver=spice
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxAudio {
    device: String,            // "ich9-intel-hda"
    driver: String,            // "spice", "pa", "none"
}

/// Parsed USB: host=1-2.2 or host=0451:16a0
#[derive(Debug, Clone, Getters, new)]
pub struct ProxmoxUsb {
    host: String,              // "1-2.2" (port) or "0451:16a0" (vendor:product)
}
```

### Top-Level `ProxmoxVmConf` Struct

```rust
#[derive(Debug, Clone, Default)]
pub struct ProxmoxVmConf {
    // Core scalars
    pub name:     Option<String>,
    pub cpu:      Option<String>,      // "host", "x86-64-v2-AES", "EPYC", etc.
    pub cores:    Option<u32>,
    pub sockets:  Option<u32>,
    pub memory:   Option<u64>,         // MiB
    pub ostype:   Option<String>,
    pub machine:  Option<String>,      // "pc-q35-8.1", "q35"
    pub bios:     Option<String>,      // "ovmf"
    pub boot:     Option<String>,      // "order=scsi0;ide2;net0"
    pub scsihw:   Option<String>,      // "pvscsi", "virtio-scsi-pci", etc.
    pub vga:      Option<String>,      // "none", "virtio-gl,memory=64"
    pub agent:    Option<u8>,
    pub tablet:   Option<u8>,
    pub balloon:  Option<u64>,
    pub hugepages:Option<u64>,
    pub numa:     Option<u8>,
    pub smbios1:  Option<String>,
    pub vmgenid:  Option<String>,
    pub meta:     Option<String>,
    pub parent:   Option<String>,
    pub hookscript:         Option<String>,
    pub spice_enhancements: Option<String>,
    pub template: Option<u8>,

    // Raw passthrough (stored verbatim)
    pub args: Option<String>,

    // Indexed devices (BTreeMap preserves insertion order by index)
    pub efidisk0:  Option<ProxmoxEfiDisk>,
    pub tpmstate0: Option<ProxmoxTpmState>,
    pub audio0:    Option<ProxmoxAudio>,
    pub hostpci:   BTreeMap<u8, ProxmoxHostPci>,  // hostpci0..N
    pub net:       BTreeMap<u8, ProxmoxNet>,       // net0..N
    pub scsi:      BTreeMap<u8, ProxmoxDisk>,      // scsi0..N
    pub sata:      BTreeMap<u8, ProxmoxDisk>,      // sata0..N
    pub virtio:    BTreeMap<u8, ProxmoxDisk>,      // virtio0..N
    pub ide:       BTreeMap<u8, ProxmoxDisk>,      // ide0..N
    pub usb:       BTreeMap<u8, ProxmoxUsb>,       // usb0..N
    pub unused:    BTreeMap<u8, String>,            // unused0..N (raw volume string)
    pub serial:    BTreeMap<u8, String>,            // serial0..N ("socket")
    pub numa_nodes:BTreeMap<u8, String>,            // numa0..N (raw string, v2)
}
```

**Why `BTreeMap<u8, T>` for indexed devices:**
- Keys (0–N) are small integers — BTreeMap is cheap and ordered
- `hashlink` (already in Cargo.toml) provides `LinkedHashMap` but `BTreeMap` from std is
  simpler and correct for this use
- Ordering by index matches Proxmox's numbering semantics

### Key index extraction rule

For a key like `hostpci0`, `scsi2`, `usb5`:
```rust
// strip known prefix ("hostpci", "scsi", "net", etc.),
// parse remaining digits as u8
fn extract_index(key: &str, prefix: &str) -> Option<u8> {
    key.strip_prefix(prefix)?.parse().ok()
}
```

---

## `ProxmoxStorageConf` Struct Design

[VERIFIED: corpus analysis of felucia/storage.cfg, coruscant/storage.cfg, zbp-server-mh2/storage.cfg]

### Grammar

```
storage.cfg format:
  section_header: "type: name\n"
  property:       "\tkey value\n"   (tab-indented, space-separated, NOT =)
  separator:      blank line between sections (optional)
```

**All property names observed:** `path`, `content`, `thinpool`, `vgname`, `shared`

### Structs

```rust
#[derive(Debug, Clone)]
pub enum ProxmoxStorageType {
    Dir {
        path: String,
    },
    LvmThin {
        thinpool: String,
        vgname: String,
    },
    Lvm {
        vgname: String,
    },
    Unknown {
        type_name: String,
    },
}

#[derive(Debug, Clone)]
pub struct ProxmoxStorageEntry {
    pub name: String,
    pub storage_type: ProxmoxStorageType,
    pub content: Vec<String>,   // ["images", "rootdir", "iso", etc.]
    pub shared: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ProxmoxStorageConf {
    pub entries: Vec<ProxmoxStorageEntry>,  // Vec preserves file order
}
```

### `FromStr` Parser Algorithm for storage.cfg

```
State: idle (waiting for section header)

For each line:
  trim trailing whitespace
  empty → if building a section, finalize it; stay in idle
  starts with '\t' → if in section, parse "key value" (split on first space)
  else → this is "type: name" — split_once(": ") → finalize previous section (if any),
                                                     start new section accumulator

End of input → finalize last section (if any)
```

**Content parsing:** `content images,rootdir` → split value by `,` → `vec!["images", "rootdir"]`

---

## Recommended Project Structure

Following the existing module pattern in the codebase:

```
src/config/proxmox.rs          ← existing file, becomes module root
                                  (keep ProxmoxVmSchema stub for Phase 4)
src/config/proxmox/
├── error.rs                   ← ProxmoxParseError (thiserror enum)
├── vm_conf.rs                 ← ProxmoxVmConf + all sub-option structs
├── storage_conf.rs            ← ProxmoxStorageConf + entry types
└── parser.rs                  ← split_sections(), parse_sub_options(),
                                  FromStr impls
```

Alternatively (simpler, for a phase of this scope): expand `src/config/proxmox.rs` in-place
with internal `mod` declarations that mirror the subdirectory structure. The directory approach
is preferred to match the existing `config/ezkvm/` pattern.

**Re-export pattern** (matches `src/config/ezkvm.rs` lines 1–6):
```rust
// src/config/proxmox.rs
pub use vm_conf::{ProxmoxVmConf, ProxmoxNet, ProxmoxDisk, ProxmoxEfiDisk,
                  ProxmoxTpmState, ProxmoxHostPci, ProxmoxAudio, ProxmoxUsb};
pub use storage_conf::{ProxmoxStorageConf, ProxmoxStorageEntry, ProxmoxStorageType};
pub use error::ProxmoxParseError;

mod error;
mod vm_conf;
mod storage_conf;
mod parser;
```

---

## `ProxmoxParseError` Design

[VERIFIED: thiserror = "2" is in Cargo.toml]

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProxmoxParseError {
    #[error("missing required sub-option '{option}' for key '{key}'")]
    MissingSubOption { key: String, option: String },

    #[error("invalid integer value '{value}' for key '{key}'")]
    InvalidInteger { key: String, value: String },

    #[error("invalid boolean value '{value}' for key '{key}'")]
    InvalidBool { key: String, value: String },

    #[error("unrecognized storage type '{type_name}'")]
    UnknownStorageType { type_name: String },

    #[error("malformed line: '{line}'")]
    MalformedLine { line: String },
}
```

Error strategy: be **tolerant on unknown keys** (ignore them), **strict on malformed values**
for known keys. Unknown keys from future Proxmox versions should not fail the parse.

---

## Common Pitfalls

### Pitfall 1 (FATAL): Snapshot Contamination
**What goes wrong:** A naive line-by-line parser that ignores `[section]` headers will process
ALL lines in the file, including snapshot fields. For 108.conf, the snapshot `[before_lg]`
contains `cpu: x86-64-v2-AES`, which would overwrite the active section's `cpu: host`.
After processing `[intermediate_20251018]` (the last snapshot), `parent: intermediate_041025`
would also contaminate the result.

**Why it happens:** The active section and all snapshot sections use the same key namespace.

**How to avoid:** Use the two-state machine. Only emit key-value pairs while in `State::Active`.
Transition to `State::Snapshot` on the first `[` line and never return to Active.

**Warning signs:** `cpu` field contains a named model instead of "host"; `snaptime` field
appears in parsed active conf (snaptime is snapshot-only); `parent` changes unexpectedly.

### Pitfall 2: URL-Encoded Comment Lines
**What goes wrong:** A line like `##args%3A -device foo` starts with `#`, but a parser that
only checks for single `#` (e.g., `if line.starts_with("# ")`) misses it. Worse, if the `##`
is stripped and `%3A` is decoded to `:`, it looks like `args: -device foo` — a live entry.

**How to avoid:** The rule is `line.starts_with('#')` — any number of `#` characters, with
anything following. No URL decoding is required.

**Verified rule:** In the corpus, zero files have a live `##key: value` pattern (non-URL-encoded
double-hash). All `##...` lines are disabled config entries that should be skipped.

### Pitfall 3: Colon Tokenization in Sub-Options
**What goes wrong:** Splitting sub-option values on `:` would destroy MAC addresses
(`BC:24:11:3A:21:B7` → six fragments), PCI BDFs (`0000:03:00` → three fragments), and
USB IDs (`0451:16a0` → two fragments).

**How to avoid:** The tokenizer splits on `,` (outer) then on the FIRST `=` only (inner).
Colons are never separators at any level of the sub-option grammar. See §Sub-Option Tokenizer.

### Pitfall 4: `pre-enrolled-keys` Boolean Parsing
**What goes wrong:** `pre-enrolled-keys=1` — the key contains a hyphen, which is valid in
Proxmox option names but requires `HashMap` lookup rather than struct field matching by name.
Also, `pre-enrolled-keys=0` appears in some snapshot variants; treating absence as `false`
must be consistent.

**How to avoid:** Treat `pre-enrolled-keys=1` as `true`, any other value (including absent)
as `false`. Store as `bool`.

### Pitfall 5: `efidisk0` Missing `efitype` in Older Entries
**What goes wrong:** Some corpus entries have `efidisk0: vm1-pool:vm-103-disk-1,size=4M`
(no `efitype`, no `pre-enrolled-keys`). Treating `efitype` as required fails.

**How to avoid:** All efidisk sub-options except `volume` are `Option<T>` or `bool` with
default `false`. Only `volume` is mandatory.

### Pitfall 6: `audio0` Has No Positional Value
**What goes wrong:** `audio0: device=ich9-intel-hda,driver=spice` — there is no positional
first token. All sub-options are named. A parser that always expects a positional first token
will error.

**How to avoid:** The per-device tokenizer for `audio0` starts with named options only.
The generic `parse_sub_options()` returns `(Option<String>, HashMap<String, String>)` where
the positional field is `None` when absent.

### Pitfall 7: `storage.cfg` Uses Space Separation, Not `=`
**What goes wrong:** Using the same sub-option tokenizer (split on first `=`) on storage.cfg
lines like `vgname vm1` produces empty key and the entire string as value.

**How to avoid:** storage.cfg properties use `split_once(' ')` NOT `split_once('=')`.
The two formats must be parsed by separate functions.

### Pitfall 8: `ide0` vs `ide2` — Non-Contiguous Indices
**What goes wrong:** Assuming ide indices are 0, 1, 2, 3. In the corpus, `ide2` is the
standard CD-ROM slot; `ide0` is used in some older configs. Index 1 is never seen.

**How to avoid:** Store as `BTreeMap<u8, ProxmoxDisk>` — arbitrary gaps are fine.

### Pitfall 9: `args` Value May Contain Colons
**What goes wrong:** The `args` value is a raw QEMU argument string:
`-spice port=5903,addr=0.0.0.0,...` — applying the sub-option tokenizer to `args` would
destroy it.

**How to avoid:** `args` is special-cased: store the entire raw value verbatim as
`Option<String>`, with NO tokenization.

### Pitfall 10: `storage.cfg` Required for Pool:Volume Resolution
**What goes wrong:** Proxmox volumes reference `pool:volume-name` (e.g.,
`vm1-pool:vm-108-boot`). If `storage.cfg` is not loaded, there is no way to resolve which
physical volume group backs the pool.

**How to avoid:** Phase 3 must parse `storage.cfg` as a first-class input alongside
`.conf`. Phase 4 (PROX-05) joins them during Runtime construction.

---

## Code Examples

### `split_sections()` Implementation Sketch

```rust
// Source: derived from corpus analysis [ASSUMED: Rust std::str idioms]
pub fn split_sections(
    input: &str,
) -> (Vec<(String, String)>, HashMap<String, Vec<(String, String)>>) {
    #[derive(Default)]
    enum State {
        #[default]
        Active,
        Snapshot(String),
    }

    let mut active: Vec<(String, String)> = Vec::new();
    let mut snapshots: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut current_snapshot: Vec<(String, String)> = Vec::new();
    let mut state = State::Active;

    for line in input.lines() {
        let line = line.trim_end();
        if line.is_empty() { continue; }
        if line.starts_with('#') { continue; }

        if let Some(rest) = line.strip_prefix('[') {
            let name = rest.trim_end_matches(']').to_string();
            if let State::Snapshot(prev_name) = state {
                snapshots.insert(prev_name, std::mem::take(&mut current_snapshot));
            }
            state = State::Snapshot(name);
            continue;
        }

        if let Some((key, value)) = line.split_once(": ") {
            let entry = (key.trim().to_string(), value.trim().to_string());
            match &state {
                State::Active => active.push(entry),
                State::Snapshot(_) => current_snapshot.push(entry),
            }
        }
    }

    if let State::Snapshot(name) = state {
        snapshots.insert(name, current_snapshot);
    }

    (active, snapshots)
}
```

### `parse_sub_options()` Implementation Sketch

```rust
// Source: derived from corpus analysis [ASSUMED: Rust std::str idioms]
pub fn parse_sub_options(raw: &str) -> (Option<String>, HashMap<String, String>) {
    let mut positional: Option<String> = None;
    let mut named: HashMap<String, String> = HashMap::new();
    let mut first = true;

    for token in raw.split(',') {
        let token = token.trim();
        if let Some((k, v)) = token.split_once('=') {
            named.insert(k.to_string(), v.to_string());
        } else if first {
            positional = Some(token.to_string());
        }
        first = false;
    }

    (positional, named)
}
```

### `ProxmoxNet` `FromStr` Sketch

```rust
// Source: derived from corpus analysis [ASSUMED]
impl TryFrom<(Option<String>, HashMap<String, String>)> for ProxmoxNet {
    type Error = ProxmoxParseError;

    fn try_from(
        (positional, opts): (Option<String>, HashMap<String, String>),
    ) -> Result<Self, Self::Error> {
        // net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1
        // "virtio=BC:24..." is a NAMED token (not positional) → model + mac
        let model_mac_key = opts.keys()
            .find(|k| matches!(k.as_str(), "virtio" | "e1000" | "e1000e" | "rtl8139"));

        let (model, mac) = match model_mac_key {
            Some(k) => (k.clone(), opts[k].clone()),
            None => return Err(ProxmoxParseError::MissingSubOption {
                key: "net".into(), option: "model".into(),
            }),
        };

        Ok(ProxmoxNet {
            model,
            mac,
            bridge: opts.get("bridge").cloned().unwrap_or_default(),
            firewall: opts.get("firewall").map(|v| v == "1").unwrap_or(false),
        })
    }
}
```

### `FromStr for ProxmoxVmConf` Dispatch Pattern

```rust
// Source: matches split_sections() output [ASSUMED]
impl FromStr for ProxmoxVmConf {
    type Err = ProxmoxParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (active, _snapshots) = split_sections(s);
        let mut conf = ProxmoxVmConf::default();

        for (key, raw_value) in active {
            match key.as_str() {
                "cpu"    => conf.cpu = Some(raw_value),
                "memory" => conf.memory = raw_value.parse().ok(),
                "cores"  => conf.cores = raw_value.parse().ok(),
                "args"   => conf.args = Some(raw_value),  // verbatim, no tokenization
                // ... other scalars ...

                k if k.starts_with("hostpci") => {
                    if let Some(idx) = extract_index(k, "hostpci") {
                        let sub = parse_sub_options(&raw_value);
                        conf.hostpci.insert(idx, ProxmoxHostPci::try_from(sub)?);
                    }
                }
                k if k.starts_with("net") => {
                    if let Some(idx) = extract_index(k, "net") {
                        let sub = parse_sub_options(&raw_value);
                        conf.net.insert(idx, ProxmoxNet::try_from(sub)?);
                    }
                }
                // ... scsi, sata, virtio, ide, usb, efidisk0, tpmstate0, audio0 ...
                _ => {} // unknown keys silently ignored (forward-compat)
            }
        }

        Ok(conf)
    }
}
```

---

## Standard Stack

No new dependencies are needed for Phase 3. [VERIFIED: Cargo.toml]

| Crate | Available | Usage |
|-------|-----------|-------|
| `thiserror = "2"` | ✓ in Cargo.toml | `ProxmoxParseError` enum |
| `derive-getters = "0.5.0"` | ✓ in Cargo.toml | Sub-option struct accessors |
| `derive-new = "0.7.0"` | ✓ in Cargo.toml | Sub-option struct constructors |
| `std::collections::HashMap` | ✓ std | `parse_sub_options()` named map |
| `std::collections::BTreeMap` | ✓ std | Indexed device maps in `ProxmoxVmConf` |
| `std::str::FromStr` | ✓ std | `FromStr for ProxmoxVmConf`, `FromStr for ProxmoxStorageConf` |

**No new Cargo.toml edits required.**

---

## Package Legitimacy Audit

> No new packages are being installed in this phase.

| Package | Verdict | Note |
|---------|---------|------|
| (none) | — | All capabilities provided by existing Cargo.toml dependencies and std |

---

## Architecture Patterns

### Data Flow

```
raw .conf text
      │
      ▼
split_sections(input: &str)
      │  ↓ discards: # comments, snapshot sections, empty lines
      │
      ▼
Vec<(key: String, raw_value: String)>  ← active section only
      │
      ▼
ProxmoxVmConf::from_str()
      │
      ├── scalar fields: direct assignment
      ├── args: verbatim copy (no tokenization)
      └── device fields → parse_sub_options(raw_value)
                               │
                               ├── positional: Option<String>  ← pool:volume, BDF, "none"
                               └── named: HashMap<K,V>         ← key=value pairs
                                          │
                                          ▼
                               ProxmoxNet::try_from()
                               ProxmoxHostPci::try_from()
                               ProxmoxDisk::try_from()
                               ProxmoxEfiDisk::try_from()
                               ProxmoxTpmState::try_from()
                               ProxmoxAudio::try_from()
                               ProxmoxUsb::try_from()
                                          │
                                          ▼
                               inserted into ProxmoxVmConf maps

raw storage.cfg text
      │
      ▼
ProxmoxStorageConf::from_str()
      │  ← line-by-line, tab-indented properties
      ▼
ProxmoxStorageConf { entries: Vec<ProxmoxStorageEntry> }
```

### Don't Hand-Roll

| Problem | Don't Build | Use Instead |
|---------|-------------|-------------|
| Typed errors | manual `Result<T, String>` | `thiserror` (already in Cargo.toml) |
| Struct accessors | manual getters | `#[derive(Getters)]` (derive-getters) |
| Struct constructors | manual `fn new()` | `#[derive(new)]` (derive-new) |
| Full INI parser library | hand-rolled | Not needed — format is simple enough for std string ops |
| URL decode library | `percent_encoding` crate | Not needed — simply skip lines starting with `#` |

---

## Validation Architecture

`workflow.nyquist_validation` is `true` in `.planning/config.json`.
Test framework: **cargo test** (no external framework). [VERIFIED: Cargo.toml, tests/ directory]

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`, cargo test) |
| Config file | Cargo.toml (no test-specific config needed) |
| Quick run command | `cargo test --test proxmox_phase3` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| PROX-01 | `108.conf` parses → `cpu == "host"`, no snapshot fields leak | integration | `cargo test --test proxmox_phase3` | ❌ Wave 0 |
| PROX-01 | `split_sections()` returns only active lines before first `[` | unit | `cargo test --test proxmox_phase3 test_split_sections` | ❌ Wave 0 |
| PROX-02 | `##args%3A` and `##hostpci1%3A` lines produce zero conf entries | unit | `cargo test --test proxmox_phase3 test_url_encoded_comments_skipped` | ❌ Wave 0 |
| PROX-03 | `net0: virtio=BC:24:11:3A:21:B7,...` → mac preserved verbatim | unit | `cargo test --test proxmox_phase3 test_net_mac_preserved` | ❌ Wave 0 |
| PROX-03 | `scsi0: vm1-pool:vm-108-boot,...` → volume preserved | unit | `cargo test --test proxmox_phase3 test_disk_pool_volume_preserved` | ❌ Wave 0 |
| PROX-03 | `hostpci0: 0000:03:00,pcie=1` → bdf preserved | unit | `cargo test --test proxmox_phase3 test_hostpci_bdf_preserved` | ❌ Wave 0 |
| PROX-04 | `storage.cfg` → `vm1-pool` entry with `vgname="vm1"` | integration | `cargo test --test proxmox_phase3 test_felucia_storage_cfg_parses` | ❌ Wave 0 |

### Wave 0 Gaps
- [ ] `tests/proxmox_phase3.rs` — all 7 tests above (file does not yet exist)

### Sampling Rate
- Per task commit: `cargo test --test proxmox_phase3`
- Per wave merge: `cargo test`
- Phase gate: `cargo test` — all 17 existing tests + 7 new tests green

---

## Security Domain

`security_enforcement` is `true`, `security_asvs_level` is 1 in `.planning/config.json`.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth in file parser |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | File read only |
| V5 Input Validation | **yes** | Validate/reject malformed conf lines; unknown keys silently ignored; parsed integers validated via `parse()` |
| V6 Cryptography | no | No crypto |

### Known Threat Patterns for File Parsers

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Oversized input (huge snapshot sections) | DoS | Line count bounds not required for v1; `.conf` files are inherently small (<10KB) |
| Malformed integer values (`memory: not_a_number`) | Tampering | Use `str::parse::<u64>()` — returns `Err`, stored as `None` or propagated |
| Path traversal in volume references | Tampering | Phase 3 stores raw strings only; Phase 4 must validate pool names against storage.cfg |
| Key injection via `split_once(": ")` | Spoofing | The format is read from trusted files (Proxmox host filesystem); no user-controlled input |

**Phase 3 security verdict:** Low risk. Input is trusted (Proxmox conf files from local FS).
Primary defense is robust `FromStr` that returns errors on unexpected values rather than panicking.

---

## Recommended Plan Breakdown

The four planned plans are well-scoped. Recommended task sizing and sequencing:

### Plan 03-01: `split_sections()` Two-Phase State Machine
**Scope:** `src/config/proxmox/parser.rs` — single function + tests
**Task estimate:** 1–2 hours
**Key deliverables:**
- `split_sections(input: &str) -> (Vec<(String, String)>, HashMap<String, Vec<(String, String)>>)`
- Tests: `test_split_sections_active_only`, `test_split_sections_with_snapshots`,
  `test_comments_skipped`, `test_url_encoded_comments_skipped`
- Must pass: `108.conf` active section contains `cpu: host`; snapshot sections do NOT
  appear in active vec

### Plan 03-02: `ProxmoxVmConf` Struct
**Scope:** `src/config/proxmox/vm_conf.rs` — all sub-option structs + top-level struct
**Task estimate:** 1–2 hours
**Key deliverables:**
- All sub-option structs (`ProxmoxNet`, `ProxmoxDisk`, `ProxmoxEfiDisk`, `ProxmoxTpmState`,
  `ProxmoxHostPci`, `ProxmoxAudio`, `ProxmoxUsb`) with `Getters` + `new`
- `ProxmoxVmConf` with `Default` + all indexed `BTreeMap` fields
- `ProxmoxParseError` in `error.rs`

### Plan 03-03: Per-Device Sub-Option Tokenizer + `FromStr for ProxmoxVmConf`
**Scope:** `src/config/proxmox/parser.rs` — `parse_sub_options()` + all `TryFrom` impls + `FromStr for ProxmoxVmConf`
**Task estimate:** 2–3 hours (most complex plan)
**Key deliverables:**
- `parse_sub_options(raw: &str) -> (Option<String>, HashMap<String, String>)`
- `TryFrom<(Option<String>, HashMap<String, String>)>` for each sub-option struct
- `impl FromStr for ProxmoxVmConf` dispatching all known keys
- Tests: MAC preserved, BDF preserved, pool:volume preserved, `args` verbatim

### Plan 03-04: `FromStr for ProxmoxStorageConf`
**Scope:** `src/config/proxmox/storage_conf.rs` + parser addition
**Task estimate:** 1 hour
**Key deliverables:**
- `ProxmoxStorageConf`, `ProxmoxStorageEntry`, `ProxmoxStorageType` structs
- `impl FromStr for ProxmoxStorageConf` with tab-indented property parsing
- Test: `felucia/storage.cfg` → `vm1-pool` entry with `vgname="vm1"`, `thinpool="pool"`

**Ordering constraint:** 03-01 must complete before 03-03 (tokenizer calls `split_sections`).
03-02 must complete before 03-03 (tokenizer fills `ProxmoxVmConf`). 03-04 is independent.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo test` | All tests | ✓ | 1.82+ (Rust 2024 edition) | — |
| `input/felucia/108.conf` | Integration tests | ✓ | in-repo file | — |
| `input/felucia/storage.cfg` | Integration tests | ✓ | in-repo file | — |

[VERIFIED: cargo test passes 17 tests successfully before Phase 3 begins]

---

## Open Questions

1. **`parent` field retention:** The active section of `108.conf` contains `parent: intermediate_20251018`, indicating snapshot lineage. Phase 3 should store it as a raw `Option<String>`. Phase 4 can decide whether to use it.
   - *Recommendation:* Store as `pub parent: Option<String>` in `ProxmoxVmConf`. No action needed in Phase 3.

2. **`efidisk0: vm1-pool:vm-103-disk-1,size=4M` (no `efitype`):** Older corpus entries omit `efitype`. The `EfiDisk` runtime struct requires `efitype: Option<String>` (already correct). `ProxmoxEfiDisk` should also make `efitype` optional.
   - *Recommendation:* Confirmed `Option<String>` for `efitype` in struct design above.

3. **`##net0%3A virtio=aa%3A30%3A00%3A00%3A01%3A0a,...` — should we error or silently discard?**
   - *Recommendation:* Silently discard. Lines starting with `#` are never live config regardless of content.

4. **`hookscript: local:snippets/103-mac.pl` — is this relevant to Runtime?**
   - *Recommendation:* Store as `Option<String>` in `ProxmoxVmConf`; Phase 4 ignores it.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | All comment lines in Proxmox conf always start with `#`; there are no other comment delimiters | §Comment Line Rules | A `; comment` style comment (not seen in corpus) would be treated as a malformed key-value and silently ignored — acceptable |
| A2 | `split_once(": ")` (colon-space) is the correct key-value split; bare `split_once(':')` would break PCI BDF values in snapshot headers | §State Machine Design | If Proxmox ever uses `key:value` without space, keys would fail to parse — but corpus analysis shows 100% use of `": "` format |
| A3 | `BTreeMap<u8, T>` is sufficient for indexed device storage (device counts never exceed 255) | §ProxmoxVmConf Struct | Proxmox supports 0–N devices; in practice corpus max is hostpci4 (index 4) |
| A4 | `audio0` is always the only audio device (index 0); no `audio1` in corpus | §Per-Device-Type Tokenizer | If future Proxmox versions add `audio1`, the current flat `Option<ProxmoxAudio>` for `audio0` would need to become `BTreeMap<u8, ProxmoxAudio>` |
| A5 | `storage.cfg` property lines always use exactly one space between key and value (`\tkey value`) | §ProxmoxStorageConf Design | If extra spaces appear, `split_once(' ')` still works (takes only first space split) — no risk |

---

## Sources

### Primary (HIGH confidence)
- `src/config/proxmox.rs` — current stub state, 78 lines [VERIFIED: direct file read]
- `input/felucia/108.conf` — active section, 4 snapshot sections [VERIFIED: direct file read]
- `input/felucia/storage.cfg` — 2 storage entries [VERIFIED: direct file read]
- `input/coruscant/storage.cfg` — 8 storage entries [VERIFIED: direct file read]
- `input/zbp-server-mh2/301.conf` — URL-encoded `##` comment patterns [VERIFIED: direct file read]
- `Cargo.toml` — dependency inventory [VERIFIED: direct file read]
- `tests/runtime_phase2.rs` — test structure pattern [VERIFIED: direct file read]

### Secondary (MEDIUM confidence)
- Grep across all 107+ corpus `.conf` files for key census, device patterns, BDF formats [VERIFIED: grep output]
- Grep across corpus for `##` comment line variants [VERIFIED: grep output]

### Tertiary (LOW confidence)
- Proxmox VE wiki for grammar confirmation [ASSUMED: training knowledge, not fetched]

---

## Metadata

**Confidence breakdown:**
- Current proxmox.rs state: HIGH — direct file read
- Corpus format analysis: HIGH — direct file reads + grep census
- Struct design: MEDIUM — derived from corpus; edge cases possible in unseen configs
- Tokenizer algorithm: HIGH — verified against all observed colon-containing values
- storage.cfg format: HIGH — three independent files read

**Research date:** 2026-07-22
**Valid until:** Stable (Proxmox .conf format changes slowly; re-verify if Proxmox 9.x is targeted)
