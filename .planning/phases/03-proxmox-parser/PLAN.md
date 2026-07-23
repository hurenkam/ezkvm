---
phase: 03-proxmox-parser
plans: [03-01, 03-02, 03-03, 03-04]
type: execute
requirements: [PROX-01, PROX-02, PROX-03, PROX-04]
depends_on: ["Phase 1 (Foundation)"]
autonomous: true
files_modified:
  - src/config/proxmox.rs          # extended with mod declarations
  - src/config/proxmox/error.rs      # new: ProxmoxParseError
  - src/config/proxmox/parser.rs     # new: split_sections(), parse_sub_options()
  - src/config/proxmox/conf.rs       # new: ProxmoxVmConf + sub-structs + FromStr
  - src/config/proxmox/storage.rs    # new: ProxmoxStorageConf + FromStr

must_haves:
  truths:
    - "108.conf active section parses to ProxmoxVmConf where cpu == Some(\"host\")"
    - "No snaptime, x86-64-v2-AES, or any other snapshot-only value appears in active ProxmoxVmConf"
    - "Lines starting with '#' (including ##url-encoded and ####multi-hash) produce zero config entries"
    - "net0 MAC address BC:24:11:3A:21:B7 is preserved verbatim — colons not treated as separators"
    - "storage.cfg vm1-pool entry has storage_type == StorageType::LvmThin and properties[\"vgname\"] == \"vm1\""
  artifacts:
    - src/config/proxmox/parser.rs   # split_sections() + parse_sub_options()
    - src/config/proxmox/conf.rs     # ProxmoxVmConf, all sub-structs, FromStr
    - src/config/proxmox/storage.rs  # ProxmoxStorageConf, ProxmoxStorageEntry, StorageType, FromStr
    - src/config/proxmox/error.rs    # ProxmoxParseError
    - src/config/proxmox.rs      # re-exports all public types
  key_links:
    - "split_sections() returns only pre-[header] lines in active Vec — snapshot lines never enter active"
    - "parse_sub_options() splits on ',' then split_once('=') — never touches ':' as a separator"
    - "FromStr for ProxmoxVmConf feeds active_entries from split_sections() into per-key dispatch"
    - "StorageConf parser is fully independent — its own state machine, no shared code with VM conf parser"
---

# Phase 3: Proxmox Parser

## Goal

Raw Proxmox `.conf` and `storage.cfg` files parse into typed `ProxmoxVmConf` and
`ProxmoxStorageConf` structs, correctly handling every known format quirk present in the corpus.

## Execution Wave Order

| Wave | Plans | Run in parallel? | Why |
|------|-------|------------------|-----|
| 1 | **03-01** | — | Tracer: module scaffold + `split_sections()` — everything depends on it |
| 2 | **03-03**, **03-04** | ✓ yes | `parse_sub_options()` touches only `parser.rs`; storage parser touches only `storage.rs` |
| 3 | **03-02** | — | `ProxmoxVmConf` + `FromStr` requires both `split_sections()` (03-01) and `parse_sub_options()` (03-03) |

> **Note on numbering vs. execution:** Plans are numbered by conceptual grouping (data structures
> in 02, helpers in 03). Execution order follows dependency, not number: 03-01 → {03-03 ∥ 03-04} → 03-02.

> **Note on RESEARCH.md divergences (PLAN.md is authoritative):**
> 1. RESEARCH.md shows `HashMap` in the `split_sections()` return type; this PLAN uses `BTreeMap` throughout for deterministic ordering — follow PLAN.md.
> 2. RESEARCH.md sketches `ProxmoxStorageEntry` with a typed enum with embedded fields; this PLAN uses a flat `StorageType` tag + `properties: BTreeMap<String, String>` — follow PLAN.md. The must_haves truths reference `properties["vgname"]`.

---

## Plan 03-01 — Module Scaffold + `split_sections()` State Machine  *(Tracer, Wave 1)*


> Expand `src/config/proxmox.rs` with submodule declarations and implement the snapshot-aware
> `split_sections()` state machine that is the foundation for every downstream plan.
> This is the tracer: it wires the full parse path for a single field (cpu) end-to-end so that
> architectural dead-ends are caught after one commit, not after ten.
>
> Purpose: Prove that the two-state machine correctly separates active section from snapshots
>          before any struct construction logic is written.
> Output:  src/config/proxmox.rs (extended) + proxmox/ directory with error.rs, parser.rs stubs; first test green.


### Task 03-01-A — Expand `proxmox.rs` to module directory

```
wave: 1
type: auto
files:
  src/config/proxmox.rs
  src/config/proxmox/error.rs
  src/config.rs (NO CHANGE NEEDED — Rust resolves mod proxmox to directory automatically)
```

**Task 03-01-A: Expand proxmox.rs to module + create error.rs**
  
**Files:**
- `src/config/proxmox.rs`
- `src/config/proxmox/error.rs`

  
**Action:**

    Step 1 — Create directory `src/config/proxmox/` (the existing `proxmox.rs` stays in place
    as the module entry point — do NOT create a mod.rs). Rust resolves `mod proxmox;` in
    `src/config.rs` to `proxmox.rs` when a `proxmox/` sibling directory also exists.

    Step 2 — Add module declarations at the TOP of `src/config/proxmox.rs` (after any
    existing `use` imports, before any struct/impl blocks), following the project convention
    of bare `mod` at the top and separate `pub use` for re-exports:
      mod error;
      mod parser;
      mod conf;
      mod storage;
      pub use error::ProxmoxParseError;

    Step 3 — Create `src/config/proxmox/error.rs` with the following thiserror enum.
    Use `thiserror` (already in Cargo.toml as version "2") — no new dependencies.

    Define `ProxmoxParseError` with exactly these four variants:
      - `InvalidLine { line: String }` — malformed key-value line (missing ": " separator in
        a non-comment, non-empty line in the active section)
      - `InvalidSection { header: String }` — snapshot header line is present but malformed
        (starts with '[' but does not end with ']')
      - `InvalidSubOption { field: String, raw: String }` — a sub-option value could not be
        parsed for the named field (e.g., integer index suffix could not be parsed)
      - `MissingRequiredField { field: String }` — a required scalar field was absent from
        the active section after full parse

    The error display messages should include the offending data. Example:
      InvalidLine: "invalid key-value line: {line}"
      InvalidSection: "malformed snapshot header: {header}"
      InvalidSubOption: "invalid sub-option in field '{field}': {raw}"
      MissingRequiredField: "required field missing: {field}"

    Also add stub empty files for parser.rs, conf.rs, storage.rs so the `mod` declarations
    in proxmox.rs compile. Each stub file must have at least one blank line (empty file compiles
    but is fine; stub comment also acceptable).
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo check 2>&1
```

  

  
**Done when:**

    `cargo check` passes with no errors. `src/config/proxmox.rs` exists with the original
    content plus module declarations. `src/config/proxmox/error.rs` defines ProxmoxParseError
    with four variants. Stub files for parser.rs, conf.rs, storage.rs exist.
  


---

### Task 03-01-B — Implement `split_sections()` + first test

**Task 03-01-B: Implement split_sections() state machine in parser.rs**
  
**Files:**
- `src/config/proxmox/parser.rs`

  
**Action:**

    Implement the following function in `src/config/proxmox/parser.rs`:

      pub fn split_sections(input: &str)
          -> (Vec<(String, String)>, std::collections::BTreeMap<String, Vec<(String, String)>>)

    The function returns `(active_entries, snapshot_map)` where:
      - active_entries: key-value pairs from lines BEFORE the first [header] line
      - snapshot_map: BTreeMap keyed by snapshot name, each value a Vec of key-value pairs

    Two-state machine (use a private enum `ParserState { Active, Snapshot(String) }`):

    STATE: Active (initial state)
      For each line:
        1. Trim trailing whitespace (but NOT leading whitespace — active lines have no indent)
        2. If line is empty → skip
        3. If first character is '#' → skip (covers #comment, ##url-encoded, ###, ####)
           CRITICAL: check the RAW byte, NEVER URL-decode before this check
        4. If line starts with '[' and ends with ']' → transition to Snapshot(name) where
           name = line[1..line.len()-1].trim().to_string()
           If line starts with '[' but does NOT end with ']' → the line is malformed;
           for robustness, skip it (do not return an error from split_sections itself —
           callers will validate)
        5. Otherwise: split on the LITERAL two-character sequence ": " (colon followed by
           exactly one space) using split_once(": "). This gives (key, value).
           - key = left side trimmed
           - value = right side (do NOT trim — raw values may have leading spaces in args)
           Push (key.to_string(), value.to_string()) to active_entries.
           If the line contains no ": " and is non-empty and non-comment → skip silently
           (these are malformed lines; callers deal with them at a higher level)

    STATE: Snapshot(current_name)
      For each line:
        1. If empty → skip (blank lines between snapshot sections are separators only)
        2. If first character is '#' → skip
        3. If line starts with '[' and ends with ']' → emit current snapshot to map under
           current_name, then transition to Snapshot(new_name) for new header
        4. Otherwise: split_once(": ") → push to current snapshot's entry Vec

    After all lines processed:
      If still in Snapshot(name) state → emit the final accumulated snapshot to map.

    Use `str::lines()` for iteration (handles both LF and CRLF).

    Add the following #[cfg(test)] module at the bottom of parser.rs:

      Test 1 — `test_split_sections_active_has_cpu_host`:
        Read the file at path "input/felucia/108.conf" relative to CARGO_MANIFEST_DIR
        using std::fs::read_to_string. Call split_sections() on the content.
        Assert:
          - active_entries contains a pair where key == "cpu" and value == "host"
          - active_entries does NOT contain any pair where key == "snaptime"
          - active_entries does NOT contain any pair where value contains "x86-64-v2-AES"
          - snapshot_map contains key "before_lg"
          - snapshot_map contains key "intermediate_20251018"
          - snapshot_map["intermediate_20251018"] contains a pair where key == "cpu"
            and value == "x86-64-v2-AES"

      Test 2 — `test_split_sections_skips_hash_comments`:
        Use an inline string containing:
          "##args%3A some disabled config\n"
          "##hostpci1%3A 0000%3A04%3A00.0\n"
          "####also_disabled: value\n"
          "cpu: host\n"
        Call split_sections() and assert:
          - active_entries has exactly ONE entry
          - That entry has key == "cpu" and value == "host"
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo test -p ezkvm test_split_sections 2>&1
```

  

  
**Done when:**

    Both tests pass. `split_sections()` is exported from parser.rs. cargo check passes.
    The tracer slice is complete: one parse path (cpu) through the state machine, end-to-end.
  


---

**Plan 03-01 UAT:**
- `cargo test test_split_sections` → both tests pass
- `cargo check` on the whole crate → no errors
- Snapshot fields do NOT appear in active_entries

---

## Plan 03-02 — `ProxmoxVmConf` Struct + `FromStr`  *(Wave 3, after 03-01 and 03-03)*


> Define the complete `ProxmoxVmConf` struct with all sub-structs and implement `FromStr`
> using `split_sections()` and the per-device helpers provided by Plan 03-03.
>
> Purpose: Deliver the typed in-memory representation that Phase 4 (Proxmox→Runtime) will
>          consume. `FromStr` is the public API surface for the whole parser phase.
> Output:  src/config/proxmox/conf.rs with all types + FromStr; construction test green.


**Task 03-02-A: Define sub-structs in conf.rs**
  
**Files:**
- `src/config/proxmox/conf.rs`

  
**Action:**

    Create `src/config/proxmox/conf.rs` with the following type definitions.
    All types derive `#[derive(Debug, Clone, PartialEq)]`.
    Use `std::collections::BTreeMap` throughout — no HashMap in this file.

    SUB-STRUCTS:

    pub struct ProxmoxDiskConf {
        pub volume: String,                          // "vm1-pool:vm-108-boot" (pool:volume verbatim)
        pub options: BTreeMap<String, String>,      // "discard"→"on", "size"→"256G", "ssd"→"1"
    }

    pub struct ProxmoxNetConf {
        pub model: String,                           // "virtio", "e1000", "e1000e", "rtl8139", etc.
        pub mac: String,                             // "BC:24:11:3A:21:B7" — colons preserved
        pub options: BTreeMap<String, String>,      // "bridge"→"vmbr0", "firewall"→"1"
    }

    pub struct ProxmoxHostPciConf {
        pub bdf: String,                             // "0000:03:00" — colons preserved
        pub options: BTreeMap<String, String>,      // "pcie"→"1", "x-vga"→"1"
    }

    pub struct ProxmoxUsbConf {
        pub host: String,                            // "1-2.2" or "0451:16a0" verbatim
    }

    pub struct ProxmoxAudioConf {
        pub device: Option<String>,                 // "ich9-intel-hda"
        pub driver: Option<String>,                 // "spice"
    }

    pub struct ProxmoxTpmConf {
        pub volume: String,                          // "vm1-pool:vm-108-tpmstate"
        pub options: BTreeMap<String, String>,      // "size"→"4M", "version"→"v2.0"
    }

    pub struct ProxmoxEfiDiskConf {
        pub volume: String,                          // "vm1-pool:vm-108-efidisk"
        pub options: BTreeMap<String, String>,      // "efitype"→"4m", "ms-cert"→"2023",
                                                    // "pre-enrolled-keys"→"1", "size"→"4M"
    }

    pub struct ProxmoxSerialConf {
        pub socket: String,                          // verbatim value (e.g., "socket")
    }

    MAIN STRUCT:

    pub struct ProxmoxVmConf {
        // Indexed device fields (BTreeMap so indices are sorted and gap-free is not required)
        pub scsi:    BTreeMap<u8, ProxmoxDiskConf>,
        pub sata:    BTreeMap<u8, ProxmoxDiskConf>,
        pub ide:     BTreeMap<u8, ProxmoxDiskConf>,
        pub virtio:  BTreeMap<u8, ProxmoxDiskConf>,   // virtio disk (virtio0, virtio1, ...)
        pub net:     BTreeMap<u8, ProxmoxNetConf>,
        pub hostpci: BTreeMap<u8, ProxmoxHostPciConf>,
        pub usb:     BTreeMap<u8, ProxmoxUsbConf>,
        pub serial:  BTreeMap<u8, ProxmoxSerialConf>,

        // Single-entry optional devices
        pub efidisk: Option<ProxmoxEfiDiskConf>,
        pub tpmstate: Option<ProxmoxTpmConf>,
        pub audio:   Option<ProxmoxAudioConf>,

        // Scalar fields
        pub memory:   Option<u64>,
        pub machine:  Option<String>,
        pub bios:     Option<String>,
        pub cpu:      Option<String>,
        pub args:     Option<String>,
        pub vga:      Option<String>,
        pub cores:    Option<u8>,
        pub sockets:  Option<u8>,
        pub name:     Option<String>,
        pub ostype:   Option<String>,
        pub numa:     Option<bool>,
        pub boot:     Option<String>,
        pub scsihw:   Option<String>,
        pub parent:   Option<String>,
        pub meta:     Option<String>,
        pub smbios1:  Option<String>,
        pub tablet:   Option<bool>,
        pub vmgenid:  Option<String>,
        pub agent:    Option<String>,
    }

    impl Default for ProxmoxVmConf — all BTreeMaps default to empty, all Options to None.

    At this point, DO NOT implement FromStr yet. Leave a comment:
      // FromStr implemented in Task 03-02-B after parse_sub_options() is available (Plan 03-03)
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo check 2>&1
```

  

  
**Done when:**

    `cargo check` passes. All sub-structs and ProxmoxVmConf are defined and exported from
    conf.rs. No FromStr yet — that is Task 03-02-B.
  


---


  **Task 03-02-B: Implement FromStr for ProxmoxVmConf**
  
**Files:**
- `src/config/proxmox/conf.rs`

  
**Behavior:**

    - Parse 108.conf active section → cpu == Some("host")
    - Parse 108.conf active section → efidisk.is_some() == true
    - Parse 108.conf active section → tpmstate.is_some() == true
    - Parse 108.conf active section → hostpci[0].bdf == "0000:03:00"
    - Parse 108.conf active section → net[0].mac == "BC:24:11:3A:21:B7"
    - Parse 108.conf active section → scsi[0].volume == "vm1-pool:vm-108-boot"
    - No snaptime in active ProxmoxVmConf (snapshot gate)
  

  
**Action:**

    Add `use std::str::FromStr;` and implement `impl FromStr for ProxmoxVmConf`.

    The `from_str` method:
      1. Call `crate::config::proxmox::parser::split_sections(s)` to get
         `(active_entries, _snapshots)`. Snapshots are discarded — only active_entries is used.

      2. Iterate active_entries as `(key, raw_val)`:

         SCALAR DISPATCH (exact key match):
           "cpu"      → conf.cpu = Some(raw_val)
           "memory"   → conf.memory = Some(raw_val.parse::<u64>()
                          .map_err(|_| ProxmoxParseError::InvalidSubOption {
                              field: "memory".into(), raw: raw_val.clone() })?)
           "machine"  → conf.machine = Some(raw_val)
           "bios"     → conf.bios = Some(raw_val)
           "args"     → conf.args = Some(raw_val)
           "vga"      → conf.vga = Some(raw_val)
           "cores"    → conf.cores = Some(raw_val.parse::<u8>()
                          .map_err(|_| ProxmoxParseError::InvalidSubOption {
                              field: "cores".into(), raw: raw_val.clone() })?)
           "sockets"  → conf.sockets = Some(raw_val.parse::<u8>()
                          .map_err(|_| ProxmoxParseError::InvalidSubOption {
                              field: "sockets".into(), raw: raw_val.clone() })?)
           "name"     → conf.name = Some(raw_val)
           "ostype"   → conf.ostype = Some(raw_val)
           "numa"     → conf.numa = Some(&raw_val != "0")
           "boot"     → conf.boot = Some(raw_val)
           "scsihw"   → conf.scsihw = Some(raw_val)
           "parent"   → conf.parent = Some(raw_val)
           "meta"     → conf.meta = Some(raw_val)
           "smbios1"  → conf.smbios1 = Some(raw_val)
           "tablet"   → conf.tablet = Some(&raw_val != "0")
           "vmgenid"  → conf.vmgenid = Some(raw_val)
           "agent"    → conf.agent = Some(raw_val)
           "efidisk0" → conf.efidisk = Some(parse_efidisk_raw(&raw_val)?)
           "tpmstate0"→ conf.tpmstate = Some(parse_tpmstate_raw(&raw_val)?)
           "audio0"   → conf.audio = Some(parse_audio_raw(&raw_val)?)

         PREFIX DISPATCH (pattern match on key prefix + numeric suffix):
           key starts_with "scsi" (but not "scsihw") →
             idx = key["scsi".len()..].parse::<u8>() map_err InvalidSubOption{field: key}
             conf.scsi.insert(idx, parse_disk_raw(&raw_val)?)
           key starts_with "sata" →
             idx = key["sata".len()..].parse::<u8>() map_err ...
             conf.sata.insert(idx, parse_disk_raw(&raw_val)?)
           key starts_with "ide" →
             idx = key["ide".len()..].parse::<u8>() map_err ...
             conf.ide.insert(idx, parse_disk_raw(&raw_val)?)
           key starts_with "virtio" →
             idx = key["virtio".len()..].parse::<u8>() map_err ...
             conf.virtio.insert(idx, parse_disk_raw(&raw_val)?)
           key starts_with "net" →
             idx = key["net".len()..].parse::<u8>() map_err ...
             conf.net.insert(idx, parse_net_raw(&raw_val)?)
           key starts_with "hostpci" →
             idx = key["hostpci".len()..].parse::<u8>() map_err ...
             conf.hostpci.insert(idx, parse_hostpci_raw(&raw_val)?)
           key starts_with "usb" →
             idx = key["usb".len()..].parse::<u8>() map_err ...
             conf.usb.insert(idx, parse_usb_raw(&raw_val)?)
           key starts_with "serial" →
             idx = key["serial".len()..].parse::<u8>() map_err ...
             conf.serial.insert(idx, parse_serial_raw(&raw_val)?)

         DEFAULT: any unrecognized key → silently skip (forward-compatible).

      3. Return Ok(conf).

    IMPORTANT — prefix ordering: match "scsihw" before "scsi" in the dispatch; or use
    explicit guard: `s if s.starts_with("scsi") && s != "scsihw"`.

    The per-device helper functions (`parse_disk_raw`, `parse_net_raw`, etc.) are defined in
    `parser.rs` (Plan 03-03). Import them with:
      use crate::config::proxmox::parser::{parse_disk_raw, parse_net_raw, parse_hostpci_raw,
                                            parse_usb_raw, parse_audio_raw, parse_serial_raw,
                                            parse_efidisk_raw, parse_tpmstate_raw};

    Add the following #[cfg(test)] module:

      Test — `test_proxmox_vm_conf_from_108_conf`:
        Read "input/felucia/108.conf" via CARGO_MANIFEST_DIR.
        Parse with ProxmoxVmConf::from_str(&content).unwrap().
        Assert ALL of:
          conf.cpu == Some("host".to_string())
          conf.efidisk.is_some()
          conf.efidisk.as_ref().unwrap().volume == "vm1-pool:vm-108-efidisk"
          conf.tpmstate.is_some()
          conf.tpmstate.as_ref().unwrap().volume == "vm1-pool:vm-108-tpmstate"
          conf.hostpci.contains_key(&0)
          conf.hostpci[&0].bdf == "0000:03:00"
          conf.net.contains_key(&0)
          conf.net[&0].mac == "BC:24:11:3A:21:B7"
          conf.net[&0].model == "virtio"
          conf.scsi.contains_key(&0)
          conf.scsi[&0].volume == "vm1-pool:vm-108-boot"
          conf.memory == Some(16384)
          // Snapshot gate: none of these should be present in active
          conf.scsi.values().all(|d| !d.volume.contains("x86-64-v2-AES"))
          // snaptime is not a struct field — its absence is implicit if conf parsed cleanly
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo test -p ezkvm test_proxmox_vm_conf_from_108_conf 2>&1
```

  

  
**Done when:**

    `test_proxmox_vm_conf_from_108_conf` passes. FromStr for ProxmoxVmConf is complete.
    All four phase success criteria touchable from this struct.
  


---

**Plan 03-02 UAT:**
- `cargo test test_proxmox_vm_conf` → construction test passes
- `cargo check` → no errors
- `conf.cpu == Some("host")` and `conf.hostpci[0].bdf == "0000:03:00"` both asserted

---

## Plan 03-03 — Per-Device-Type Sub-Option Tokenizer  *(Wave 2, parallel with 03-04)*


> Implement `parse_sub_options()` and all per-device-type helper functions in `parser.rs`.
> These helpers are the contract between the raw string values produced by `split_sections()`
> and the typed sub-structs consumed by Plan 03-02's FromStr.
>
> Purpose: Isolate the colon-safe tokenization logic so MAC addresses, BDFs, pool:volume
>          references, and USB VID:PID strings are NEVER split on ':'.
> Output:  parser.rs with parse_sub_options + 8 per-device helpers; 6 unit tests green.



  **Task 03-03-A: Implement parse_sub_options() + per-device helpers**
  
**Files:**
- `src/config/proxmox/parser.rs`

  
**Behavior:**

    - parse_sub_options("virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1")
        → [(Some("virtio"), "BC:24:11:3A:21:B7"), (Some("bridge"), "vmbr0"), (Some("firewall"), "1")]
    - parse_sub_options("vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1")
        → [(None, "vm1-pool:vm-108-boot"), (Some("discard"), "on"), (Some("size"), "256G"), (Some("ssd"), "1")]
    - parse_sub_options("0000:03:00,pcie=1,x-vga=1")
        → [(None, "0000:03:00"), (Some("pcie"), "1"), (Some("x-vga"), "1")]
    - parse_net_raw("virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1")
        → ProxmoxNetConf { model: "virtio", mac: "BC:24:11:3A:21:B7", options: {"bridge": "vmbr0", "firewall": "1"} }
    - parse_disk_raw("vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1")
        → ProxmoxDiskConf { volume: "vm1-pool:vm-108-boot", options: {"discard": "on", ...} }
    - parse_hostpci_raw("0000:03:00,pcie=1,x-vga=1")
        → ProxmoxHostPciConf { bdf: "0000:03:00", options: {"pcie": "1", "x-vga": "1"} }
    - parse_usb_raw("host=1-2.2") → ProxmoxUsbConf { host: "1-2.2" }
    - parse_usb_raw("host=0451:16a0") → ProxmoxUsbConf { host: "0451:16a0" }
    - parse_efidisk_raw("vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M")
        → ProxmoxEfiDiskConf { volume: "vm1-pool:vm-108-efidisk", options: {"efitype": "4m", "pre-enrolled-keys": "1", "size": "4M"} }
  

  
**Action:**

    Add the following to `src/config/proxmox/parser.rs`. Import sub-struct types at top:
      use crate::config::proxmox::conf::{
          ProxmoxDiskConf, ProxmoxNetConf, ProxmoxHostPciConf,
          ProxmoxUsbConf, ProxmoxAudioConf, ProxmoxTpmConf,
          ProxmoxEfiDiskConf, ProxmoxSerialConf,
      };
      use crate::config::proxmox::error::ProxmoxParseError;
      use std::collections::BTreeMap;

    ─── CORE TOKENIZER ───

    pub fn parse_sub_options(raw: &str) -> Vec<(Option<String>, String)> {
        raw.split(',')
           .map(|token| {
               match token.split_once('=') {
                   Some((k, v)) => (Some(k.to_string()), v.to_string()),
                   None         => (None, token.to_string()),
               }
           })
           .collect()
    }

    CRITICAL RULES for parse_sub_options:
      - Split on ',' FIRST to produce tokens
      - Then split each token on the FIRST '=' using split_once('=')
      - NEVER split on ':' at any point — colons are literal in MAC, BDF, pool:volume, VID:PID
      - This is the ONLY tokenization function; all per-device helpers call it

    ─── PER-DEVICE HELPERS ───

    pub fn parse_disk_raw(raw: &str)
        -> Result<ProxmoxDiskConf, ProxmoxParseError>
      Tokens = parse_sub_options(raw)
      First token must be positional (key = None); its value is volume.
      Remaining tokens with key = Some(k) go into options BTreeMap.
      If first token has key = Some(...) → return Err(InvalidSubOption { field: "disk", raw: raw.into() })
      If "none" is the volume (e.g., ide2: none,media=cdrom) → volume = "none", this is valid.

    pub fn parse_net_raw(raw: &str)
        -> Result<ProxmoxNetConf, ProxmoxParseError>
      Tokens = parse_sub_options(raw)
      Find the first token where key == Some(k) and k matches any of:
        ["virtio", "e1000", "e1000e", "e1000-82540em", "rtl8139", "vmxnet3", "i82551",
         "i82557b", "i82559er", "ne2k_pci", "ne2k_isa", "pcnet", "rocker"]
      That token's key is model, its value is mac.
      All other tokens with key = Some(k) go into options.
      If no model token found → return Err(InvalidSubOption { field: "net", raw: raw.into() })

    pub fn parse_hostpci_raw(raw: &str)
        -> Result<ProxmoxHostPciConf, ProxmoxParseError>
      Tokens = parse_sub_options(raw)
      First positional token (key = None) → bdf value.
      Remaining named tokens → options BTreeMap.
      If first token is named → Err(InvalidSubOption { field: "hostpci", raw: raw.into() })

    pub fn parse_usb_raw(raw: &str)
        -> Result<ProxmoxUsbConf, ProxmoxParseError>
      Tokens = parse_sub_options(raw)
      Find token with key == Some("host") → ProxmoxUsbConf { host: value }
      The value may contain ':' (USB VID:PID) — do NOT split it.
      If no "host=" token → Err(InvalidSubOption { field: "usb", raw: raw.into() })

    pub fn parse_audio_raw(raw: &str)
        -> Result<ProxmoxAudioConf, ProxmoxParseError>
      Tokens = parse_sub_options(raw)
      NOTE: audio0 has NO positional first token — all tokens are named.
      Collect: device = token with key "device", driver = token with key "driver"
      Return ProxmoxAudioConf { device, driver } with Option wrapping.

    pub fn parse_efidisk_raw(raw: &str)
        -> Result<ProxmoxEfiDiskConf, ProxmoxParseError>
      Same structure as parse_disk_raw but returns ProxmoxEfiDiskConf.
      First positional token → volume (e.g., "vm1-pool:vm-108-efidisk")
      Remaining named tokens → options (will include "efitype", "ms-cert",
        "pre-enrolled-keys", "size" — all go into options BTreeMap verbatim)

    pub fn parse_tpmstate_raw(raw: &str)
        -> Result<ProxmoxTpmConf, ProxmoxParseError>
      Same structure as parse_disk_raw but returns ProxmoxTpmConf.
      First positional token → volume. Remaining → options.

    pub fn parse_serial_raw(raw: &str)
        -> Result<ProxmoxSerialConf, ProxmoxParseError>
      ProxmoxSerialConf { socket: raw.to_string() }
      Verbatim — the value of serial0 is typically "socket" or a path.
      Always succeeds.

    ─── UNIT TESTS ───

    Add to the #[cfg(test)] mod in parser.rs:

    test_parse_sub_options_net_mac:
      Input: "virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1"
      Assert tokens[0] == (Some("virtio"), "BC:24:11:3A:21:B7")
      Assert tokens[1] == (Some("bridge"), "vmbr0")
      Assert tokens[2] == (Some("firewall"), "1")

    test_parse_sub_options_disk_pool_volume:
      Input: "vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1"
      Assert tokens[0] == (None, "vm1-pool:vm-108-boot")
      Assert tokens[1] == (Some("discard"), "on")

    test_parse_net_raw_mac_preserved:
      Input: "virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1"
      Result = parse_net_raw(input).unwrap()
      Assert result.mac == "BC:24:11:3A:21:B7"
      Assert result.model == "virtio"
      Assert result.options["bridge"] == "vmbr0"
      Assert result.options["firewall"] == "1"

    test_parse_hostpci_raw_bdf_preserved:
      Input: "0000:03:00,pcie=1,x-vga=1"
      Result = parse_hostpci_raw(input).unwrap()
      Assert result.bdf == "0000:03:00"
      Assert result.options["pcie"] == "1"
      Assert result.options["x-vga"] == "1"

    test_parse_usb_raw_port:
      Input: "host=1-2.2"
      Result = parse_usb_raw(input).unwrap()
      Assert result.host == "1-2.2"

    test_parse_usb_raw_vid_pid:
      Input: "host=0451:16a0"
      Result = parse_usb_raw(input).unwrap()
      Assert result.host == "0451:16a0"   // colon in vid:pid preserved

    test_parse_efidisk_raw:
      Input: "vm1-pool:vm-108-efidisk,efitype=4m,pre-enrolled-keys=1,size=4M"
      Result = parse_efidisk_raw(input).unwrap()
      Assert result.volume == "vm1-pool:vm-108-efidisk"
      Assert result.options["efitype"] == "4m"
      Assert result.options["pre-enrolled-keys"] == "1"
      Assert result.options["size"] == "4M"
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo test -p ezkvm test_parse_ 2>&1
```

  

  
**Done when:**

    All seven parser unit tests pass. `parse_sub_options` and all eight per-device helpers
    are exported from parser.rs. No ':' is treated as a separator anywhere in this file.
    Phase success criterion SC-3 (MAC address preserved) is validated by test_parse_net_raw_mac_preserved.
  


---

**Plan 03-03 UAT:**
- `cargo test test_parse_` → all 7 tests pass
- `parse_net_raw("virtio=BC:24:11:3A:21:B7,...")` → mac == "BC:24:11:3A:21:B7"
- `parse_usb_raw("host=0451:16a0")` → host == "0451:16a0" (colon preserved in VID:PID)

---

## Plan 03-04 — `FromStr for ProxmoxStorageConf`  *(Wave 2, parallel with 03-03)*


> Implement the storage.cfg parser with its own state machine completely independent from
> the VM conf parser. `storage.cfg` uses a different grammar: stanza headers are `type: name`,
> properties are tab-indented `key value` lines (NOT `key: value` or `key=value`).
>
> Purpose: Phase 4 (Proxmox→Runtime) needs storage volume resolution to map `pool:volume`
>          references to host device paths. This struct provides that data.
> Output:  src/config/proxmox/storage.rs with all types + FromStr; test parsing felucia/storage.cfg.



  **Task 03-04-A: Define ProxmoxStorageConf types and implement FromStr**
  
**Files:**
- `src/config/proxmox/storage.rs`
- `src/config/proxmox.rs`

  
**Behavior:**

    - Parse "lvmthin: vm1-pool\n\tthinpool pool\n\tvgname vm1\n\tcontent images,rootdir\n"
        → entry with key "vm1-pool", storage_type == StorageType::LvmThin,
          properties["vgname"] == "vm1", properties["thinpool"] == "pool"
    - Parse "dir: local\n\tpath /var/lib/vz\n\tcontent iso,vztmpl\n"
        → entry with key "local", storage_type == StorageType::Dir,
          properties["path"] == "/var/lib/vz", properties["content"] == "iso,vztmpl"
    - Parse full felucia/storage.cfg → both "local" and "vm1-pool" entries present
    - felucia/storage.cfg → vm1-pool entry has storage_type == LvmThin and properties["vgname"] == "vm1"
  

  
**Action:**

    Create `src/config/proxmox/storage.rs`:

    TYPES (all derive Debug, Clone, PartialEq):

      pub enum StorageType {
          Dir,
          LvmThin,
          Lvm,
          Unknown(String),   // forward-compatible: unknown type names stored verbatim
      }

      pub struct ProxmoxStorageEntry {
          pub storage_type: StorageType,
          pub properties: std::collections::BTreeMap<String, String>,
      }

      pub struct ProxmoxStorageConf {
          pub entries: std::collections::BTreeMap<String, ProxmoxStorageEntry>,
                       // keyed by storage name, e.g. "vm1-pool", "local"
      }

    GRAMMAR NOTES (from direct inspection of input/felucia/storage.cfg):
      - Section header: `type: name\n` where type and name are separated by ": " (colon-space)
        Example: "lvmthin: vm1-pool"  →  type_str = "lvmthin", name = "vm1-pool"
      - Property lines: `\tkey value\n` — TAB-indented, key and value separated by the FIRST
        space (split_once(' ') on the tab-stripped line)
        Example: "\tvgname vm1"  →  strip leading tab  →  "vgname vm1"  →  ("vgname", "vm1")
      - Properties do NOT use '=' or ':' as separator — this is different from VM conf
      - Blank lines between stanzas are separators; skip them
      - No comment lines observed in storage.cfg corpus; skip '#' lines for safety

    PARSER STATE MACHINE:

    enum StorageParserState { Idle, Building { name: String, entry: ProxmoxStorageEntry } }

    impl FromStr for ProxmoxStorageConf:
      Initial state: Idle.
      conf = ProxmoxStorageConf { entries: BTreeMap::new() }

      For each line (use str::lines()):
        1. If line is empty or starts with '#' → skip

        2. If line starts with '\t' (TAB character) → PROPERTY LINE
           If state is Idle → skip (property before any header — ignore gracefully)
           If state is Building → strip leading tab, then split_once(' ') on the stripped line:
             If Some((key, value)) → entry.properties.insert(key.to_string(), value.to_string())
             If None → skip (malformed property — key with no value)

        3. Otherwise → HEADER LINE
           If state is Building → finalize: conf.entries.insert(name, entry)
           Parse header: split_once(": ") on the line →
             Some((type_str, name)) →
               storage_type = match type_str {
                   "dir"     => StorageType::Dir,
                   "lvmthin" => StorageType::LvmThin,
                   "lvm"     => StorageType::Lvm,
                   other     => StorageType::Unknown(other.to_string()),
               }
               Transition to Building { name: name.to_string(),
                                         entry: ProxmoxStorageEntry {
                                             storage_type,
                                             properties: BTreeMap::new() } }
             None → skip (malformed header line)

      After loop: if state is Building → finalize remaining entry.
      Return Ok(conf).

    IMPORTANT: This state machine is COMPLETELY SEPARATE from split_sections().
    Do NOT call split_sections() from the storage parser — the grammars are different.

    EXPORTS in proxmox.rs: Add a `pub use` line alongside the existing re-exports at the top
    of `src/config/proxmox.rs` (after the `mod storage;` declaration):
      pub use storage::{ProxmoxStorageConf, ProxmoxStorageEntry, StorageType};

    Add #[cfg(test)] module:

      test_parse_storage_cfg_felucia:
        Read "input/felucia/storage.cfg" via CARGO_MANIFEST_DIR.
        Parse with ProxmoxStorageConf::from_str(&content).unwrap().
        Assert ALL of:
          conf.entries.contains_key("vm1-pool")
          conf.entries["vm1-pool"].storage_type == StorageType::LvmThin
          conf.entries["vm1-pool"].properties["vgname"] == "vm1"
          conf.entries["vm1-pool"].properties["thinpool"] == "pool"
          conf.entries.contains_key("local")
          conf.entries["local"].storage_type == StorageType::Dir
          conf.entries["local"].properties["path"] == "/var/lib/vz"

      test_parse_storage_inline_lvmthin:
        Use inline string: "lvmthin: vm1-pool\n\tthinpool pool\n\tvgname vm1\n\tcontent images,rootdir\n"
        Assert: entry "vm1-pool" exists, storage_type == LvmThin, vgname == "vm1"

      test_parse_storage_inline_dir:
        Use inline string: "dir: local\n\tpath /var/lib/vz\n\tcontent iso,vztmpl\n"
        Assert: entry "local" exists, storage_type == Dir, path == "/var/lib/vz"
  

  
**Verify:**

    
```sh
cd /home/hurenkam/Workspace/ezkvm_v4 && cargo test -p ezkvm test_parse_storage 2>&1
```

  

  
**Done when:**

    All three storage tests pass. `ProxmoxStorageConf::from_str()` correctly parses
    felucia/storage.cfg. Phase success criterion SC-4 (vm1-pool LvmThin with vgname "vm1")
    is validated by test_parse_storage_cfg_felucia.
  


---

**Plan 03-04 UAT:**
- `cargo test test_parse_storage` → all 3 tests pass
- `conf.entries["vm1-pool"].storage_type == StorageType::LvmThin`
- `conf.entries["vm1-pool"].properties["vgname"] == "vm1"`

---

## Source Coverage Audit

| Source | Item | Covered by | Status |
|--------|------|------------|--------|
| **GOAL** | Active `.conf` → `ProxmoxVmConf` with no snapshot leakage | 03-01, 03-02 | ✓ COVERED |
| **GOAL** | `storage.cfg` → `ProxmoxStorageConf` with typed entries | 03-04 | ✓ COVERED |
| **PROX-01** | Two-state machine separates active from snapshot | 03-01 | ✓ COVERED |
| **PROX-02** | URL-encoded `##` comment lines produce no config entries | 03-01 | ✓ COVERED |
| **PROX-03** | Colon-safe sub-option tokenization (MAC, BDF, pool:volume) | 03-03 | ✓ COVERED |
| **PROX-04** | `storage.cfg` parsed as first-class input | 03-04 | ✓ COVERED |
| **SC-1** | `cpu == "host"` and no `snaptime`/`x86-64-v2-AES` in active | 03-01 test, 03-02 test | ✓ COVERED |
| **SC-2** | `##args%3A` lines → zero config entries | 03-01 test | ✓ COVERED |
| **SC-3** | `BC:24:11:3A:21:B7` MAC preserved verbatim | 03-03 test | ✓ COVERED |
| **SC-4** | `vm1-pool` LvmThin with correct vgname | 03-04 test | ✓ COVERED |
| **D: serial** | `ProxmoxSerialConf` with BTreeMap field on struct | 03-02 struct, 03-03 parse_serial_raw | ✓ COVERED |
| **D: ProxmoxParseError** | 4-variant thiserror enum in error.rs | 03-01 | ✓ COVERED |
| **D: no new deps** | Only `thiserror` (already in Cargo.toml) + `std` | All plans | ✓ COVERED |

---


**Threat model:**

## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Filesystem → parser | `.conf` and `storage.cfg` files are read from disk; content is UNTRUSTED if the Proxmox host is compromised or if ezkvm is run against an adversarial file |
| Parser → caller | Parsed structs passed to Phase 4 importer; corrupted parse results would silently produce wrong QEMU commandlines |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation |
|-----------|----------|-----------|----------|-------------|------------|
| T-03-01 | Tampering | `split_sections()` state machine | high | mitigate | Two-state enum with explicit `Active`/`Snapshot(name)` states — snapshot lines physically cannot enter `active_entries`; test `test_split_sections_active_has_cpu_host` asserts zero leakage |
| T-03-02 | Spoofing | `##url-encoded` comment bypass | high | mitigate | Comment check on RAW byte before any decoding — first-char `#` check in split_sections; test `test_split_sections_skips_hash_comments` covers ##, ###, #### |
| T-03-03 | Tampering | Colon confusion in sub-option values | medium | mitigate | `parse_sub_options()` never splits on `:` — only `,` then first `=`; enforced by 7 unit tests in Plan 03-03 |
| T-03-04 | Denial of Service | Malformed `.conf` with no `: ` separator on active lines | low | accept | Malformed lines are silently skipped; no panic path in `split_sections()`; Proxmox-generated files always have valid separators |
| T-03-05 | Information Disclosure | Snapshot secrets (e.g., passwords in args) leaking into active section | high | mitigate | Active section terminates at first `[header]` line — snapshot content is isolated in `snapshot_map` which `FromStr for ProxmoxVmConf` discards entirely |
| T-03-06 | Tampering | storage.cfg property line with `: ` instead of space separator | low | accept | Storage parser uses split_once(' ') on tab-stripped lines; `key: value` in storage would parse as key="key:" value="value" — detectable by Phase 4 resolver as invalid key name |


---

## Phase Verification

All four success criteria must pass after Plan 03-02 completes:

```bash
# SC-1 and SC-2: Snapshot isolation and comment skipping
cargo test -p ezkvm test_split_sections

# SC-3: MAC preserved verbatim, all colon-safe tokenization
cargo test -p ezkvm test_parse_

# SC-1 full: Complete ProxmoxVmConf construction from 108.conf
cargo test -p ezkvm test_proxmox_vm_conf_from_108_conf

# SC-4: Storage.cfg parses to typed structs with correct vgname
cargo test -p ezkvm test_parse_storage

# Full suite must be green
cargo test -p ezkvm 2>&1 | tail -5
```

Expected final output:
```
test result: ok. N passed; 0 failed; 0 ignored
```

---

## Risks and Pitfalls

| # | Risk | Impact | Mitigation in Plan |
|---|------|--------|-------------------|
| R-1 | **Snapshot contamination (FATAL)** — naive single-pass parser overwrites `cpu: host` with snapshot's `cpu: x86-64-v2-AES` | Wrong QEMU args silently generated in Phase 7 | Explicit `ParserState` enum; 03-01-B test asserts no `x86-64-v2-AES` in active entries |
| R-2 | **URL-encoded comment bypass** — `##args%3A value` decoded before '#' check looks like valid config | Silent config injection | '#' checked on RAW bytes before any processing; 03-01-B test with `##args%3A` input |
| R-3 | **Colon as separator** — `split(':')` on `BC:24:11:3A:21:B7` produces 6 garbage tokens | MAC/BDF/pool:volume data corrupted | `parse_sub_options()` uses ONLY ',' and '=' (first occurrence); never touches ':'; 6 unit tests in 03-03 |
| R-4 | **`scsihw` key caught by `scsi` prefix** — key `scsihw` starts with `scsi` and would be parsed as disk index | Corrupt BTreeMap entry, `scsihw` field lost | 03-02-B dispatch: match "scsihw" explicitly BEFORE the `starts_with("scsi")` prefix guard |
| R-5 | **`audio0` positional confusion** — `audio0: device=ich9-intel-hda,driver=spice` has no positional first token | parse_audio_raw incorrectly requires a positional → parse failure | `parse_audio_raw()` starts with named options only; 03-03 documents this explicitly |
| R-6 | **storage.cfg uses space not '=' for properties** — `vgname vm1` not `vgname=vm1` | Using split_once('=') produces wrong results | Storage parser uses split_once(' ') on tab-stripped line; completely separate from VM conf parser |
| R-7 | **`virtio` key collision** — `virtio0` is a disk device, `virtio=<MAC>` inside net0 is a NIC model | Wrong dispatch: `virtio0` in key dispatch caught as NIC | Key dispatch in 03-02-B: `starts_with("virtio")` at TOP LEVEL is the disk (virtio0, virtio1); `virtio` inside net value sub-options is the model identifier in `parse_net_raw()` — these are different levels of parsing |

---

## Success Criteria

All four criteria must be TRUE after Phase 3 completes:

1. ✅ **SC-1 Snapshot isolation**: `input/felucia/108.conf` parses to `ProxmoxVmConf` where
   `conf.cpu == Some("host".to_string())` and no snapshot value `"x86-64-v2-AES"` or
   `snaptime` key is present in any active field.

2. ✅ **SC-2 Comment safety**: `##args%3A` and `##hostpci1%3A` lines produce zero entries in
   the active `Vec<(String, String)>` returned by `split_sections()`.

3. ✅ **SC-3 MAC verbatim**: `net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1` parses
   to `ProxmoxNetConf` where `mac == "BC:24:11:3A:21:B7"`.

4. ✅ **SC-4 Storage entry**: `input/felucia/storage.cfg` parses to `ProxmoxStorageConf` where
   `entries["vm1-pool"].storage_type == StorageType::LvmThin` and
   `entries["vm1-pool"].properties["vgname"] == "vm1"`.


**Output:**

After completing all four plans, create `.planning/phases/03-proxmox-parser/03-SUMMARY.md`
documenting:
- Files created and their public API surface
- Key design decisions (two-state machine enum, colon-safe tokenizer, independent storage parser)
- Test inventory (list all tests by name and what success criterion they verify)
- Any deviations from this plan with rationale


---

## Commit Checklist (per plan)

After each plan's verification passes, **commit manually** with a message following this convention:
```
feat(proxmox-parser/03-01): module scaffold + split_sections state machine
feat(proxmox-parser/03-02): ProxmoxVmConf structs + FromStr dispatch
feat(proxmox-parser/03-03): parse_sub_options + per-device tokenizers
feat(proxmox-parser/03-04): ProxmoxStorageConf + FromStr
```

`cargo test` must pass after each commit. No plan should leave the codebase in a non-compiling state.
