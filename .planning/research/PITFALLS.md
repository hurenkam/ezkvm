# Domain Pitfalls

**Domain:** QEMU/Proxmox VM configuration conversion tool (Rust)
**Researched:** 2026-07-22
**Confidence:** HIGH — based on direct analysis of input corpus, existing codebase, and known Proxmox/QEMU format semantics

---

## Critical Pitfalls

Mistakes that cause silent data loss, VM boot failures, or rewrites.

---

### Pitfall 1: Snapshot Sections Contaminate the Active Config

**What goes wrong:** The Proxmox `.conf` format embeds named snapshot sections directly in the same file, separated by `[snapshot-name]` headers. A naive line-by-line parser that doesn't track section context will mix snapshot fields into the active config, producing a corrupted Runtime.

**Why it happens:** The 108.conf example has four sections (`[before_lg]`, `[intermediate_041025]`, `[intermediate_20251018]`, plus the unnamed active section at the top). Each section repeats the full VM config as it was at that point in time. Without explicit section tracking, a parser accumulates all values and later sections overwrite earlier ones — or worse, the last snapshot's `parent` key replaces the active config's.

**Evidence from corpus:**
```
# 108.conf active state:
parent: intermediate_20251018   ← snapshot reference, not a VM property
cpu: host                        ← active value

[before_lg]
cpu: x86-64-v2-AES               ← snapshot value — must NOT bleed into active
snaptime: 1713825087             ← only appears in snapshots
```

**Consequences:** Runtime builds with wrong CPU, wrong memory, wrong devices. Boot fails or VM runs with wrong hardware. `snaptime` leaking into active config causes parse errors.

**Prevention:**
- Parse `.conf` with a two-phase state machine: phase 1 accumulates the active section (lines before the first `[...]`); phase 2 accumulates snapshots into a `Vec<Snapshot>` keyed by name
- The active Runtime is built only from phase 1 data
- Snapshots are stored on a separate `Snapshot` struct with their own fields (including `parent`, `snaptime`, `vmgenid`)
- Add a test that parses 108.conf and asserts `runtime.cpu == "host"` (not `"x86-64-v2-AES"`)

**Detection:** Parse 108.conf and check that the resulting Runtime has `cpu: host` and no `snaptime` field.

**Phase assignment:** Proxmox import phase (`.conf` → Runtime)

---

### Pitfall 2: URL-Encoded Commented-Out Keys (`##key%3Avalue`)

**What goes wrong:** Proxmox uses a non-standard convention to "comment out" config lines: it double-hashes the line and URL-encodes the `:` separator as `%3A`. A parser that URL-decodes all values before checking for comment markers will treat these as live config entries.

**Evidence from corpus (301.conf):**
```
##args%3A -device virtio-vga,id=vga1,max_hostmem=33554432,bus=pcie.0
##hostpci1%3A 0000%3A04%3A00.0,pcie=1
##memory%3A 16384
```
If decoded: `args: -device virtio-vga,...` and `hostpci1: 0000:04:00.0,pcie=1` and `memory: 16384` — all valid-looking keys that would override the actual active config.

**Consequences:** Phantom devices appear in the Runtime. Memory is silently overridden. VMs boot with devices the user intentionally disabled.

**Prevention:**
- Strip comment lines (starting with `#`) **before** any value decoding
- Comment detection must happen on the raw bytes, not after URL-decoding
- A line starting with `#` is always a comment regardless of what follows
- The distinction between `#` (single-hash comment) and `##` (commented-out config) is cosmetic; both are comments

**Detection:** Parse 301.conf and assert `runtime.hostpci.len() == 1` (only `hostpci0`), not 2.

**Phase assignment:** Proxmox import phase

---

### Pitfall 3: Sub-Option Grammar — Colons Inside Values

**What goes wrong:** Proxmox sub-option values use comma-separated `key=value` pairs, but the first token is often a bare value containing colons (storage pool references, PCI BDFs, MAC addresses). Simple splitting on `,` then `=` breaks on these.

**Evidence from corpus:**
```
net0: virtio=BC:24:11:3A:21:B7,bridge=vmbr0,firewall=1
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1
hostpci0: 0000:03:00,pcie=1,x-vga=1
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M
usb0: host=1-2.2
```
Traps:
- `BC:24:11:3A:21:B7` — MAC address inside `virtio=<mac>`, colons are part of the value
- `vm1-pool:vm-108-boot` — first positional token (storage pool:volume), no `=`, contains `:`
- `0000:03:00` — PCI BDF as first positional token, no `=`, contains `::`
- `host=1-2.2` — USB port path using `.` notation

**Consequences:** MAC addresses are truncated. Storage volumes are mis-identified. PCI BDFs are corrupted. Silent data loss on re-export.

**Prevention:**
- Parse sub-options with a proper tokenizer, not a generic split: first token is always the positional value (everything before the first `,` that is not inside a `key=<value-with-colons>` context)
- For each device type, define the grammar explicitly: `net` has `<driver>=<mac>` as first token; `scsi`/`efidisk` have `<pool>:<volume>` as first token; `hostpci` has `<domain>:<bus>:<slot>` as first token
- Write a sub-option parser test for each device type in the corpus before implementing the full parser

**Detection:** Parse `net0: virtio=BC:24:11:3A:21:B7,...` and assert `mac_address == "BC:24:11:3A:21:B7"`.

**Phase assignment:** Proxmox import phase + export phase (round-trip test)

---

### Pitfall 4: Multi-Function PCI Passthrough Expansion

**What goes wrong:** A single `hostpci0: 0000:03:00,pcie=1,x-vga=1` entry in the `.conf` expands to **two** `-device vfio-pci` entries in the QEMU commandline — one for function `.0` with `multifunction=on`, one for function `.1`. Treating it as a single device produces a VM where the GPU audio (function 1) is missing.

**Evidence from corpus:**
```
# .conf:
hostpci0: 0000:03:00,pcie=1,x-vga=1

# .qemu.cmd:
-device vfio-pci,host=0000:03:00.0,id=hostpci0.0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on
-device vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1
```
The BDF `0000:03:00` has an implied `.0`; function `.1` is the GPU audio companion.

**Consequences:** GPU passthrough works but audio doesn't. The `x-vga=1` flag tells Proxmox to pass the primary display — missing function 1 loses HDMI audio for Looking Glass setups.

**How to detect more functions:** Proxmox probes all functions on the slot. The Runtime model must either store a list of functions or query the host (via `/sys/bus/pci/devices/`) to enumerate them.

**Prevention:**
- The `hostpci` Runtime device must store the **base BDF** (domain:bus:slot) and a **function list** (default: `[0, 1]` if multiple functions exist on the slot)
- During QEMU cmd export, emit one `-device vfio-pci` per function, with `.function=0` getting `multifunction=on` if more than one function
- Add a test: import `hostpci0: 0000:03:00,pcie=1,x-vga=1` → assert QEMU cmd contains exactly two `vfio-pci` entries

**Phase assignment:** Runtime model (hostpci representation) + QEMU export phase

---

### Pitfall 5: Drive/Device Pairing Order in QEMU Commandline

**What goes wrong:** QEMU requires `-drive if=none,id=drive-<X>,...` to appear **before** the corresponding `-device ...,drive=drive-<X>,...`. Emitting them in the wrong order (e.g., sorted alphabetically by device type) produces `Drive 'drive-scsi0' not found` errors at boot.

**Evidence from corpus (108.qemu.cmd.split):**
```
-drive if=none,id=drive-ide2,media=cdrom,aio=io_uring       ← drive first
-device ide-cd,bus=ide.1,unit=0,drive=drive-ide2,...         ← device references it

-drive file=/dev/vm1/vm-108-boot,if=none,id=drive-scsi0,...  ← drive first
-device scsi-hd,...,drive=drive-scsi0,...                    ← device references it
```

**Consequences:** VM fails to boot with cryptic QEMU error. The bug is invisible in the model layer — it only manifests at commandline emission time.

**Prevention:**
- The QEMU commandline builder must emit the `-drive` argument immediately before its paired `-device` argument, not in a separate pass
- Alternatively, use a two-pass emitter but with a strict ordering contract: drives section always before devices section
- Add integration test: parse the emitted commandline and assert every `drive=<id>` reference appears after a `-drive ...,id=<id>,...` argument

**Phase assignment:** QEMU commandline export phase

---

### Pitfall 6: `netdev` Must Precede Its `virtio-net-pci` Device

**What goes wrong:** Same ordering issue as drive/device, but for network. The `-netdev type=tap,id=net0,...` must come before `-device virtio-net-pci,...,netdev=net0,...`.

**Evidence from corpus:**
```
-netdev type=tap,id=net0,ifname=tap108i0,...  ← netdev first
-device virtio-net-pci,mac=...,netdev=net0,... ← device references it
```

For the ezkvm commandline (ezkvm.qemu.cmd), the bridge helper variant is used:
```
-netdev type=bridge,id=net0,br=br0,helper=/usr/lib/qemu/qemu-bridge-helper
```
This is different from Proxmox's tap-based netdev — the ezkvm variant must use the bridge helper path appropriate for the target system.

**Prevention:**
- Same strategy as Pitfall 5: emit netdev immediately before its device
- Keep `tap` vs `bridge` netdev type as a host-level configuration option, not baked into the Runtime

**Phase assignment:** QEMU commandline export phase

---

### Pitfall 7: `args` Passthrough Contains State-Specific QEMU Fragments

**What goes wrong:** The `args` field in Proxmox `.conf` is raw QEMU commandline text appended verbatim. It often contains devices that have no Proxmox representation (ivshmem, SPICE virtio serial, virtio input devices). If these are parsed naively as structured data, their internal dependencies are lost.

**Evidence from corpus (108.conf `args`):**
```
args: -spice port=5903,addr=0.0.0.0,disable-ticketing=on
      -device virtio-serial-pci
      -chardev spicevmc,id=vdagent,name=vdagent
      -device virtserialport,chardev=vdagent,name=com.redhat.spice.0
      -device virtio-mouse
      -device virtio-keyboard
      -device ivshmem-plain,memdev=ivshmem0,bus=pcie.0
      -object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
```
The `ivshmem-plain` device references `ivshmem0` which is defined by the `-object` later in the same `args` string. The chardev `vdagent` is referenced by a virtserialport. These cross-reference constraints must survive round-trip.

**Consequences:** If `args` is split into individual devices and re-assembled in different order, cross-references break. `vdagent` chardev referenced before it's defined → QEMU error.

**Prevention:**
- Phase 1: implement `args` as an opaque passthrough blob (store as `Vec<String>` of raw QEMU tokens; emit verbatim at end of commandline)
- Phase 2: implement structured parsing of known `args` patterns (ivshmem, SPICE) once the Runtime model has dedicated types for them
- Never reorder individual args tokens without understanding their cross-references

**Phase assignment:** Runtime model (args representation) + QEMU export phase

---

## Moderate Pitfalls

---

### Pitfall 8: Attribute Order in `.conf` Sub-Options Affects Proxmox Acceptance

**What goes wrong:** When re-exporting a `.conf` file, Proxmox is sensitive to the ordering of sub-option attributes within a key's value. Proxmox's own parser is generally order-insensitive, but `pvesh` (the Proxmox API) and config auditing tools perform textual diff comparisons. Emitting `size=256G,discard=on` instead of `discard=on,size=256G` makes the config "look changed" even though it's semantically identical.

**Evidence from corpus:**
```
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1   ← canonical Proxmox order
```
Proxmox's canonical order for scsi devices: `<pool>:<vol>`, then `cache`, `discard`, `iothread`, `size`, `ssd`.

**Prevention:**
- Define canonical attribute ordering for each device type in the export serializer
- Write round-trip tests that do byte-level comparison of the exported `.conf` against the original for the known corpus
- Accept that 100% byte-identical output is aspirational; document known acceptable divergences (e.g., `meta:` timestamp may differ)

**Phase assignment:** Proxmox `.conf` export phase

---

### Pitfall 9: EFI Disk Size Mismatch Between `.conf` and QEMU Commandline

**What goes wrong:** The `.conf` shows `efidisk0: ...,size=4M` but the QEMU commandline emits `size=540672` (bytes). These refer to the same disk but in different units. A round-trip test that compares the `.conf` `size` field to the QEMU `size` field will falsely fail.

**Evidence:**
```
# .conf:
efidisk0: vm1-pool:vm-108-efidisk,efitype=4m,ms-cert=2023,pre-enrolled-keys=1,size=4M

# .qemu.cmd:
-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=/dev/vm1/vm-108-efidisk,size=540672
```
`540672 bytes = 528 KiB ≠ 4 MiB`. The QEMU size is the actual block device size; the `.conf` size is the logical/allocated size.

**Prevention:**
- Store two separate size representations in the EFI disk Runtime: `logical_size` (from `.conf`, human-readable) and `block_device_size` (from QEMU cmd, in bytes)
- During `.conf` export, emit `logical_size`; during QEMU cmd export, query or store `block_device_size`
- Do not attempt to convert between them programmatically — the relationship is opaque

**Phase assignment:** EFI/OVMF Runtime model + Proxmox import phase

---

### Pitfall 10: `storage.cfg` Required to Resolve Volume Paths

**What goes wrong:** The `.conf` volume references like `vm1-pool:vm-108-boot` are logical names that require `storage.cfg` to resolve to block device paths (`/dev/vm1/vm-108-boot`). If only the `.conf` is imported without `storage.cfg`, volume-to-path mapping fails silently and QEMU receives a non-existent file path.

**Evidence:**
```
# storage.cfg:
lvmthin: vm1-pool
    thinpool pool
    vgname vm1
    content images,rootdir
```
From this, `vm1-pool:vm-108-boot` → `/dev/vm1/vm-108-boot` (LVM thin: vgname/volume).

**Prevention:**
- The Proxmox import phase must accept `storage.cfg` as a required companion to `.conf`
- Implement a `StorageResolver` that maps `<pool>:<volume>` to paths using storage.cfg rules
- Different storage types have different path templates: `lvmthin` → `/dev/<vg>/<vol>`, `dir` → `<path>/images/<vmid>/<vol>`
- Fail loudly if `storage.cfg` is absent and the `.conf` has storage references

**Phase assignment:** Proxmox import phase

---

### Pitfall 11: `bootindex` Numbering Must Match Boot Order

**What goes wrong:** The `boot: order=scsi0;ide2;net0` in `.conf` maps to `bootindex=100,101,102` on the respective QEMU devices. The numbers themselves are arbitrary but their relative order determines boot priority. If new devices are added between export passes and `bootindex` is not recalculated, the boot order silently changes.

**Evidence:**
```
# .conf:
boot: order=scsi0;ide2;net0

# .qemu.cmd:
-device scsi-hd,...,bootindex=100      ← scsi0 boots first
-device ide-cd,...,bootindex=101       ← ide2 second
-device virtio-net-pci,...,bootindex=102  ← net0 third
```

**Prevention:**
- The QEMU export phase must derive `bootindex` values from the Runtime's boot order list, not from static values
- Start at 100, increment by 1 for each boot device in order
- Devices not in the boot order get no `bootindex` attribute

**Phase assignment:** QEMU commandline export phase

---

### Pitfall 12: `virtio-scsi-single` vs `pvscsi` SCSI Controller Differences

**What goes wrong:** The corpus contains both `scsihw: pvscsi` (felucia/108.conf) and `scsihw: virtio-scsi-single` (zbp-server-mh2/103.conf). These are different controllers with different QEMU device models and different QEMU bus types. Treating them as interchangeable breaks disk addressing.

**Evidence:**
```
# pvscsi (felucia/108):
-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5
-device scsi-hd,bus=scsihw0.0,scsi-id=0,...

# virtio-scsi-single (would be):
-device virtio-scsi-pci,id=scsihw0,...
-device scsi-hd,bus=scsihw0.0,...
```

**Prevention:**
- The Runtime SCSI model must carry the controller type as an enum: `PvScsi | VirtioScsiSingle | VirtioScsiPci | Lsi | MegaSas`
- Each controller type has a different QEMU device name and potentially different bus addressing semantics
- Current codebase already has `PvScsi` but is missing `VirtioScsiSingle` — add it before processing zbp-server-mh2 configs

**Phase assignment:** Runtime model + QEMU export phase

---

### Pitfall 13: `iothread=1` per-Disk Option for virtio-scsi-single

**What goes wrong:** The `scsihw: virtio-scsi-single` configuration creates one SCSI controller per disk (not one shared controller for all disks). The `iothread=1` option on individual disks in this mode generates separate QEMU devices. A shared-controller model cannot represent this correctly.

**Evidence (zbp-server-mh2/301.conf):**
```
scsihw: virtio-scsi-single
scsi0: ...,iothread=1,...
scsi1: ...,iothread=1,...
scsi2: ...,iothread=1,...
scsi3: ...,iothread=1,...
```
Each `scsihw: virtio-scsi-single` disk gets its own controller in QEMU.

**Prevention:**
- When importing `virtio-scsi-single`, create a separate `VirtioScsiSingleController` Runtime object per disk
- Do not attempt to share a controller across disks in this mode

**Phase assignment:** Runtime model + Proxmox import phase

---

## Minor Pitfalls

---

### Pitfall 14: `hugepages: 1024` Requires Host-Level Memory Setup

**What goes wrong:** The `hugepages: 1024` config option in `301.conf` adds `-mem-prealloc -mem-path /dev/hugepages` to the QEMU commandline. If the host doesn't have hugepages configured, the VM will fail to start. This is a host-side concern, not a config concern, but the ezkvm output must still emit the correct QEMU flags.

**Prevention:**
- Track `hugepages` as a VM-level option in the Runtime
- During QEMU export, emit `-mem-prealloc -mem-path /dev/hugepages/libvirt/qemu/<name>` when hugepages is set
- Document that hugepage availability is a host prerequisite; do not validate it at conversion time

**Phase assignment:** Runtime model + QEMU export phase

---

### Pitfall 15: `serial0: socket` Generates a Chardev + Serial Device Pair

**What goes wrong:** `serial0: socket` in `301.conf` generates both a chardev socket and a serial device. It's not a standalone device; the serial and chardev are coupled. A model that represents only the serial device loses the chardev binding.

**Prevention:**
- Model `serial<N>` as a struct containing both the serial device type and the chardev type
- For `socket`, emit `-chardev socket,id=serial0,...` and `-device isa-serial,chardev=serial0`

**Phase assignment:** Runtime model

---

### Pitfall 16: `usb` Host Passthrough — Multiple Address Formats

**What goes wrong:** USB passthrough accepts three different address formats in Proxmox:
- `host=1-2.2` (bus-port notation)
- `host=1-4` (bus only, port implicit)
- `host=0451:16a0` (USB VID:PID)

Evidence from zbp-server-mh2/301.conf has all three formats. Each maps to different QEMU `-device usb-host` parameters.

**Prevention:**
- Parse USB `host=` values into a typed enum: `BusPort(u8, String)`, `Bus(u8)`, `VidPid(u16, u16)`
- Each variant emits different QEMU `-device usb-host` arguments
- The `hostbus`/`hostport` vs `vendorid`/`productid` distinction must be preserved

**Phase assignment:** Runtime model + Proxmox import phase

---

### Pitfall 17: `vga: none` vs `vga: virtio-gl,memory=128`

**What goes wrong:** `vga: none` (in felucia/108.conf) means `-vga none -nographic` in QEMU — no display device at all (used with PCI passthrough + Looking Glass). `vga: virtio-gl,memory=128` (in zbp-server-mh2 configs) adds a VirtGL-accelerated display. These have completely different QEMU representations. Treating `none` as "no field" misses the `-vga none -nographic` flags.

**Prevention:**
- Model display as an enum: `None | Std | Cirrus | Vmware | Qxl | Virtio { memory_mb }| VirtioGl { memory_mb } | Spice` etc.
- `None` variant must actively emit `-vga none -nographic`
- Default (absent `vga` key) should also emit `-vga std`

**Phase assignment:** Runtime model + QEMU export phase

---

## Rust-Specific Pitfalls

---

### Pitfall 18: `as_any()` + `downcast_ref()` Pattern Does Not Scale

**What goes wrong:** The current codebase uses `as_any().downcast_ref::<T>()` in 54 places. Every new device type requires updating all these sites. There is no compile-time exhaustiveness check — adding a new type silently skips it in all existing visitors.

**Evidence (existing codebase):**
```rust
// parser.rs — must be updated for every new SCSI device type:
if scsi_device.as_any().downcast_ref::<Hdd>().is_some() { ... }
else if scsi_device.as_any().downcast_ref::<Ssd>().is_some() { ... }
else if scsi_device.as_any().downcast_ref::<Cdrom>().is_some() { ... }
// Missing new type? Silent error, returns "unsupported" — no compiler warning.
```

**Why it happens:** The initial design used `Arc<dyn Trait>` for shared ownership, which requires dynamic dispatch, which requires `dyn`. The natural solution to "what type is this?" then becomes downcasting.

**Prevention:**
- Add a `fn device_kind(&self) -> StorageDeviceKind` method to each trait (`StorageDevice`, `PcieDevice`, etc.)
- Match on the returned enum, not on the concrete type
- This preserves `Arc<dyn Trait>` polymorphism while giving the compiler exhaustiveness checking
- The enum `StorageDeviceKind { Hdd, Ssd, Cdrom }` is stable; the trait objects are polymorphic

**Detection:** Running `cargo clippy` will not catch this pattern — only adding a new device type and observing silent skip will reveal it.

**Phase assignment:** Runtime model refactor (early, before adding many new device types)

---

### Pitfall 19: Unit Error Types in `TryFrom` Chains Lose Diagnostic Information

**What goes wrong:** `proxmox.rs` and `qemu.rs` use `type Error = ()` in their `TryFrom` implementations. The ezkvm schema layer uses `type Error = String`. When a full conversion chain fails (Proxmox conf → Runtime → QEMU cmd), the error message says nothing about which device, which field, or which conversion step failed.

**Evidence:**
```rust
// qemu.rs:
impl TryFrom<Runtime> for QemuSchema {
    type Error = ();    ← no context possible
```
```rust
// proxmox.rs:
impl TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)> for Runtime {
    type Error = ();    ← caller gets nothing useful
```

**Prevention:**
- Define a proper error enum for each conversion boundary:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum ProxmoxImportError {
      #[error("snapshot section '{name}' missing required field '{field}'")]
      MissingSnapshotField { name: String, field: String },
      #[error("unsupported device type '{device}' in hostpci{index}")]
      UnsupportedHostPci { index: u8, device: String },
      ...
  }
  ```
- Use `thiserror` crate for ergonomic error derivation
- Propagate with `?` and `map_err(|e| ConversionError::Layer(e))`

**Detection:** Attempt to import a malformed `.conf` and observe what error you get; if it's `()` or `"conversion failed"`, this pitfall is present.

**Phase assignment:** Error handling refactor (early, before user-facing CLI is built)

---

### Pitfall 20: `TryFrom` Tuple Input Pattern Breaks Composability

**What goes wrong:** `TryFrom<(ProxmoxVmSchema, ProxmoxHostSchema)>` bundles unrelated schemas into a tuple to pass multiple inputs. Rust's `TryFrom` trait does not compose well with tuples — error propagation requires manual unwrapping, and you can't use `?` directly on `TryFrom` tuple conversions in chains.

**Prevention:**
- Use a dedicated `ProxmoxImporter` struct with fields for all inputs:
  ```rust
  pub struct ProxmoxImporter {
      vm_schema: ProxmoxVmSchema,
      host_schema: ProxmoxHostSchema,
      storage_config: StorageConfig,
  }
  impl ProxmoxImporter {
      pub fn into_runtime(self) -> Result<Runtime, ProxmoxImportError> { ... }
  }
  ```
- This is more discoverable, easier to extend (add new fields), and naturally composable
- Reserve `TryFrom` for simple single-input conversions where the conversion is unambiguous

**Phase assignment:** Proxmox import phase design (before implementing)

---

### Pitfall 21: `Mutex` in `RuntimeBuilder` Is Unnecessary and Panic-Prone

**What goes wrong:** `RuntimeBuilder` uses `Mutex<Vec<Arc<dyn RootDevice>>>` internally, but builders are fundamentally single-threaded. If any thread panics while holding the mutex lock, all subsequent `.lock().unwrap()` calls panic with "poisoned mutex". Additionally, `into_inner()` on a consumed `Mutex` is fine, but intermediate `lock().unwrap()` calls in builder methods are fragile.

**Evidence (existing codebase):**
```rust
pub struct RuntimeBuilder {
    root_devices: Mutex<Vec<Arc<dyn RootDevice>>>,  // Mutex for no reason
}
impl RuntimeBuilder {
    pub fn with_memory(self, memory: Memory) -> Self {
        self.root_devices.lock().unwrap().push(Arc::new(memory));  // can panic
        self
    }
}
```

**Prevention:**
- Remove `Mutex` from `RuntimeBuilder`; use `Vec<Arc<dyn RootDevice>>` directly
- Builders are single-threaded by design (they're consumed by value in the builder pattern)
- If thread safety is ever needed, wrap the `RuntimeBuilder` at the call site

**Phase assignment:** Runtime model cleanup (early)

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Proxmox `.conf` import | Snapshot section bleeding (P1) | State machine parser with explicit section tracking |
| Proxmox `.conf` import | `##key%3A` comment parsing (P2) | Strip `#` lines before any URL-decoding |
| Proxmox `.conf` import | Sub-option colons in values (P3) | Per-device-type tokenizer, not generic split |
| Proxmox `.conf` import | `storage.cfg` resolution (P10) | Require `storage.cfg` as companion input |
| Proxmox `.conf` import | USB passthrough address formats (P16) | Parse into `UsbHostRef` enum |
| Runtime model | Multi-function PCI expansion (P4) | Store function list per `hostpci` device |
| Runtime model | `virtio-scsi-single` vs `pvscsi` (P12, P13) | Controller type enum, one-controller-per-disk mode |
| Runtime model | `args` passthrough ordering (P7) | Opaque blob first; structured later |
| Runtime model | Trait downcast proliferation (P18) | Add `device_kind()` method to device traits early |
| QEMU export | Drive before device ordering (P5) | Emit drive+device pairs atomically |
| QEMU export | netdev before device ordering (P6) | Emit netdev+device pairs atomically |
| QEMU export | `bootindex` derivation (P11) | Derive from boot order list |
| Proxmox `.conf` export | Sub-option attribute ordering (P8) | Per-device canonical attribute order |
| Error handling | Unit error types (P19) | `thiserror` error enums from phase 1 |
| Error handling | Tuple `TryFrom` (P20) | Named importer structs instead |

---

## Sources

- Direct analysis of `input/felucia/108.conf`, `108.qemu.cmd.split`, `108.machine-layout`, `108.ezkvm.qemu.cmd`
- Direct analysis of `input/zbp-server-mh2/301.conf`, `103.conf`, `storage.cfg`
- Direct code analysis: `src/config/ezkvm/runtime/parser.rs`, `src/config/ezkvm/runtime/builder.rs`, `src/config/proxmox.rs`, `src/config/qemu.rs`, `src/runtime.rs`
- `src/config/ezkvm/runtime/parser.rs` test suite (lines 270–441) — reveals what round-trip paths are currently tested and what's missing
- `.planning/codebase/CONCERNS.md` — existing debt analysis (54 downcast sites, mutex poisoning, unit errors)
- **Confidence: HIGH** — all pitfalls are grounded in corpus evidence or existing code patterns, not speculation
