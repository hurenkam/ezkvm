# Phase 7: QEMU Cmdline - Research

**Researched:** 2026-07-24
**Domain:** QEMU `-drive`/`-device`/`-netdev`/`-chardev`/`-object` commandline syntax; Rust segmented-builder emitter pattern over an existing typed device-tree (`Runtime`)
**Confidence:** MEDIUM — QEMU CLI syntax is HIGH confidence (cross-checked against a real Proxmox-generated command in-repo); the Rust integration plan is HIGH confidence (read directly from source); the bootindex/boot-order data model is a **confirmed gap** requiring a planning decision (see Critical Gap below).

## Summary

Phase 7 replaces the `src/config/qemu.rs` stub with a segmented `QemuCommandLine` struct and a `TryFrom<(Runtime, QemuContext)>` impl that walks the *entire* device tree — not just the seven Phase 2 root devices, but also the nested bus maps inside `Q35Chipset` (`pcie_bus`, `pci_bus`, `sata_bus`, `ide_bus`, `usb_bus`) where storage, network, PCI passthrough, ivshmem, and USB devices actually live. QEMU's `-drive`/`-device` and `-netdev`/`-device` pairs are two independent objects joined by an `id=`/`drive=`/`netdev=` string reference, and QEMU parses arguments strictly left-to-right — a `-device ...,drive=X` before its `-drive id=X` is a **silent runtime error** (device attaches to nothing, or QEMU refuses to start depending on version). This is why the codebase's own decision log already mandates a segmented builder instead of a flat `Vec<String>`.

Two source-verified facts materially change the shape of this phase versus the roadmap's plan-bullet sketch: (1) network (`net0`) and USB (`usb0`) devices are **not yet imported** by `ProxmoxImporter` (Phase 4) even though `ProxmoxVmConf` parses them — so the felucia golden-fixture Runtime used for the D-04 ordering test will *not* exercise netdev-before-device ordering unless synthetic tests cover it (already planned per D-04, but the planner must not assume the golden fixture alone proves QEMU-02/03 for net devices); and (2) **there is no boot-order field anywhere in the current data model** (Runtime, BootSchema, or ProxmoxVmConf's parsed struct) even though `boot: order=scsi0;ide2;net0` is captured as a raw string in `ProxmoxVmConf.boot: Option<String>` and then **silently dropped** by the importer. Pitfall 11 in the roadmap assumes a "Runtime boot order list" that does not exist yet — this must be resolved as a planning decision before Task 07-03 can be written (see Critical Gap).

**Primary recommendation:** Build the emitter as one dispatcher over `Runtime::root_devices()` (matching `RootDeviceKind`) plus one recursive walker over `Q35Chipset`'s five bus maps (matching `PcieBusDeviceKind`/`PciBusDeviceKind`/`SataDevice`/`IdeDevice`/`UsbBusDeviceKind`), both writing into the same shared `QemuCommandLineBuilder` (mutable, segment-keyed). Resolve the boot-order gap explicitly in Task 07-00 before starting emission logic.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Runtime→cmdline device dispatch | Backend/domain (`src/config/qemu.rs`) | — | Pure in-memory transformation, no I/O |
| Segment ordering (drives/netdevs/devices) | Backend/domain | — | Correctness invariant lives entirely in the builder struct |
| Path/socket resolution (`QemuContext`) | Backend/domain, caller-supplied | Phase 8 (process launch) | Phase 7 only *consumes* resolved paths; Phase 8 owns where sockets/files actually live on disk |
| bootindex derivation | Backend/domain | Runtime model (needs new field — see gap) | Requires ordered device-id list which isn't modeled yet |
| Actual `qemu-system-x86_64` process launch | Phase 8 | — | Out of scope here; Phase 7 only produces the string/argv |

## Package Legitimacy Audit

Not applicable — this phase adds zero new external dependencies. `thiserror`, `derive-getters`, `derive-new` are already in `Cargo.toml` and are reused, not newly installed.

## User Constraints (from CONTEXT.md)

<user_constraints>
### Locked Decisions

- **D-01:** `QemuCommandLine::try_from((Runtime, QemuContext))` dispatches over ALL root devices via `device_kind()`, not just the seven v1 types. Pre-existing types (`Memory`, `Q35Chipset`, storage, network, USB) get their own emit handlers in this phase since `src/config/qemu.rs` is currently only a stub (`NoHandler` error, no working handlers). — **Reversibility:** costly.
- **D-02:** Args needed to make the emitted cmdline runnable for testing are in scope now: `-machine` (from `Q35Chipset`), `-m` (from `Memory`), `-smp`, `-cpu`.
- **D-03:** Host/process-management args are explicitly deferred to Phase 8: `-id`, `-name`, `-smbios type=1,uuid=...`, `-pidfile`, `-daemonize`, `-readconfig`. Phase 7 does not emit these.
- **D-04:** Use `input/felucia/108.conf` (via Phase 3/4 Proxmox parser + importer) as a golden-fixture Runtime source for an ordering/coverage test, combined with synthetic per-device unit tests.
- **D-05:** The felucia-based test asserts **structural correctness only** — token presence + relative ordering — NOT byte-for-byte match against `108.qemu.cmd`. Exact-match/boot verification is Phase 9's job (QEMU-04).
- **D-06:** `QemuContext` is built internally (not parsed from untrusted input). Missing context data is a programmer error — panic/`.expect()`, not a typed `QemuConversionError` variant. Typed errors remain reserved for Runtime-data conversion problems.

### the agent's Discretion

Not explicitly separated in CONTEXT.md beyond the above decisions — implementation details of segment struct field naming, handler function decomposition, and bootindex-gap remediation approach are left to the planner/implementer, constrained by the roadmap's fixed segment list (`machine`, `firmware`, `drives`, `netdevs`, `chardevs`, `tpm`, `objects`, `devices`, `misc`).

### Deferred Ideas (OUT OF SCOPE)

- Host/process-management args (`-id`, `-name`, `-smbios`, `-pidfile`, `-daemonize`, `-readconfig`) — Phase 8.
- Byte-for-byte / boot-verified cmdline correctness against real Proxmox output — Phase 9 (QEMU-04).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| QEMU-01 | Runtime generates a valid QEMU commandline covering all RUNT-01–07 device types | Standard Stack + Code Examples sections give exact flag syntax for each of the 7 v1 types; Architecture section gives the dispatch structure to guarantee coverage |
| QEMU-02 | Emitter guarantees drive/netdev argument precedes its corresponding `-device` argument | Segment-struct pattern (Don't Hand-Roll + Architecture Patterns) enforces this by construction, not by careful call-ordering |
| QEMU-03 | Raw `args` passthrough is appended verbatim at end of generated commandline | `RawArgs.0` is a single opaque `String`; Common Pitfalls documents the verbatim-append rule and why it must not be tokenized |
</phase_requirements>

## Critical Gap (must be resolved before Task 07-03 can be written)

**Finding [VERIFIED: repo inspection]:** The roadmap's Pitfall 11 says "Derive `bootindex` values from the Runtime boot order list starting at 100" — but no such list exists in the codebase today:

- `ProxmoxVmConf.boot: Option<String>` (`src/config/proxmox/conf.rs`) stores the raw Proxmox string (e.g. `"order=scsi0;ide2;net0"`) but is **never read** by `ProxmoxImporter::into_runtime()` (`src/config/proxmox/importer.rs`) — confirmed by grep, zero references to `.boot` in that file.
- `Runtime` (`src/runtime.rs`) has no boot-order field of any kind.
- `BootSchema` (`src/config/ezkvm/schema/boot.rs`) has only `secure: Option<bool>` and `bios: BiosSchema` — no order list.
- The real felucia sample confirms the expected mapping (`108.qemu.cmd.split` lines 41/44/48): boot order `scsi0;ide2;net0` → `scsi0` gets `bootindex=100`, `ide2` gets `bootindex=101`, `net0` gets `bootindex=102` (i.e., **increment by 1 per position, starting at 100** — not gapped, not weighted).

**Recommendation for the planner:** Add a minimal, additive Runtime concept before Task 07-03:
1. Add `boot_order: Vec<String>` (device-id strings like `"scsi0"`, `"ide2"`, `"net0"`) as a new field/getter — either on `Runtime` directly, or threaded through `QemuContext` if the team wants to avoid touching the Phase 2 `Runtime` struct. Given `QemuContext` per D-06 is "built internally," and boot order is genuinely Runtime-derived data (not caller-environment data like socket paths), the cleaner home is `Runtime`, but this is a call for the planner/user, not this research doc.
2. Update `ProxmoxImporter::into_runtime()` to parse `order=a;b;c` (split on `;`, strip `order=` prefix) into that list — this is a small, isolated change to a completed phase's code, so flag it explicitly as a cross-phase touch in the plan (it modifies Phase 4 output, not just Phase 7 code).
3. The emitter then needs a **device-id reconstruction function** per bus (see below) to match Runtime bus addresses back to the label strings in that boot-order list, since Runtime stores devices by `(target, lun)` / `(channel, device)` / `(device, function)` tuples, not by the Proxmox-style label string.

**Device-id reconstruction mapping (derived from the felucia sample, MEDIUM confidence — only one example VM to derive the pattern from):**

| Device family | Runtime address | Proxmox/QEMU label | Formula |
|---|---|---|---|
| SCSI (`PvScsi.scsi_bus`) | `ScsiAddress { target, lun }` | `scsi{target}` | label = `format!("scsi{}", target)` (assumes `lun == 0`, true in all Proxmox single-disk-per-target usage) |
| IDE (`Q35Chipset.ide_bus`) | `IdeAddress { channel, device }` | `ide{N}` | `N = channel * 2 + device` (confirmed: felucia `ide2` → `bus=ide.1,unit=0` → channel=1, device=0 → 1*2+0=2 ✓) |
| Network (`Q35Chipset.pcie_bus`, kind `VirtioNet`) | `PcieAddress` | `net{N}` | `N` = ordinal position among `VirtioNetPcie` entries sorted by `PcieAddress` (only one nic in the sample; ordinal assumption needs a second corpus file to fully confirm — flag as `[ASSUMED]`) |
| SATA (`Q35Chipset.sata_bus`) | `SataAddress { port, device }` | `sata{port}` | Not present in the felucia sample; `[ASSUMED]` by analogy with SCSI/IDE conventions |

If the planner decides bootindex is not achievable cleanly in this phase's timebox, the fallback (consistent with D-06's spirit of "typed errors reserved for genuine problems") is to make `bootindex` assignment a **best-effort pass**: emit no `bootindex` for any device when `boot_order` is empty (which is exactly what "devices absent from the boot order omit bootindex entirely" already implies for the *partial* case) — this makes the gap non-blocking for QEMU-01/02/03 (bootindex isn't in the phase's 4 success criteria) while still being addressed for QEMU-04 (Phase 9, byte-match) via the boot_order plumbing above. **This means Task 07-03's bootindex work could be descoped to "no-op until boot_order exists," with the plumbing done as a fast-follow inside Phase 7 or deferred to Phase 9 — the planner must pick one explicitly, it should not be silently assumed.**

## Standard Stack

### Core

No new libraries. This phase is 100% string formatting over existing typed structs using the standard library (`std::fmt::Display`, `format!`, `Vec<String>`).

| Component | Version | Purpose | Why Standard |
|---|---|---|---|
| `std::fmt::Display` | stdlib | `QemuCommandLine` → final argv string | Matches existing project convention (`Memory`, `Q35Chipset`, `Chipset` already implement `Display`) [VERIFIED: repo inspection] |
| `thiserror` | 2.x (already a dep) | `QemuConversionError` variants for genuine Runtime-data problems only (per D-06) | Established Phase 1 convention [VERIFIED: repo inspection, `src/config/proxmox/error.rs`, `src/config/ezkvm/runtime/error.rs`] |

### Supporting

None needed — no new crates for tokenizing/shlex (that's QIMP-01/02, a different, v2/import-direction requirement, not this phase).

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| Segmented `Vec<String>` fields + fixed-order `Display` | Single flat `Vec<String>` built via careful call-ordering | Rejected — this is exactly Pitfall 5/6; a flat vec depends on handler call order being correct forever, segments make correctness structural |
| `Vec<String>` per segment (roadmap's chosen shape) | `Vec<Cow<'static, str>>` or a typed `Arg` enum | Not worth the complexity for this phase; plain owned `String`s match existing codebase idioms (no `Cow` used anywhere else in the project) |

**Installation:** None — no `Cargo.toml` changes required for this phase.

**Version verification:** N/A (no new packages).

## Architecture Patterns

### System Architecture Diagram

```
Runtime                                   QemuContext (caller-built)
  root_devices: Vec<Arc<dyn RootDevice>>    vm_name, socket_paths, storage_paths
        │                                            │
        └───────────────┬────────────────────────────┘
                         ▼
        TryFrom<(Runtime, QemuContext)> for QemuCommandLine
                         │
      ┌──────────────────┴───────────────────────┐
      ▼                                           ▼
 dispatch over root_devices()             recurse into Chipset::Q35(q35)
 match RootDeviceKind { ... }             walk q35.pcie_bus / pci_bus /
   Memory      → machine.push("-m ..")           sata_bus / ide_bus / usb_bus
   Chipset     → machine.push("-machine ..")     match {Pcie,Pci,Sata,Ide,Usb}DeviceKind
   EfiDisk     → drives.push(pflash pair)          HostPci    → devices (+ multifunction)
   TpmState    → chardevs+tpm.push(chardev+tpmdev)  Ivshmem    → objects + devices
   AudioDevice → devices+misc.push(audiodev pair)   VirtioNet  → netdevs + devices
   SpiceDisplay→ misc.push(-spice ..)               PvScsi     → devices, then recurse scsi_bus
   RawArgs     → misc.push(verbatim string)         Sata/IdeDevice → drives + devices
      │                                           │
      └──────────────────┬────────────────────────┘
                          ▼
              QemuCommandLineBuilder
       (mutable accumulator: 9 Vec<String> segments)
                          │
                          ▼
                  QemuCommandLine { .. }
             impl Display: emit segments in
     fixed order: machine, firmware, drives, netdevs,
       chardevs, tpm, objects, devices, misc
                          │
                          ▼
        "qemu-system-x86_64 -machine q35 -m 16384 ..."
     (consumed by Phase 8 process launch; validated
      end-to-end by Phase 9 against a running VM)
```

### Recommended Project Structure

```
src/config/qemu.rs                 # QemuCommandLine, QemuContext, QemuConversionError, TryFrom impl (entry point)
src/config/qemu/
├── builder.rs                     # QemuCommandLineBuilder: 9 Vec<String> fields + push helpers per segment
├── handlers/
│   ├── root.rs                    # handlers for RootDeviceKind::{Memory,Chipset,EfiDisk,TpmState,AudioDevice,SpiceDisplay,RawArgs}
│   ├── pcie.rs                    # handlers for PcieBusDeviceKind::{HostPci,Ivshmem,VirtioNet,PvScsi}
│   ├── pci.rs                     # handlers for PciBusDeviceKind::{GenericPci,PvScsi}
│   ├── storage.rs                 # shared drive/device emission for Ssd/Hdd/Cdrom across scsi/sata/ide buses
│   └── usb.rs                     # handlers for UsbBusDeviceKind::Generic
└── bootindex.rs                   # boot_order lookup + device-id reconstruction (see Critical Gap)
```
(This is a *suggested* decomposition consistent with the codebase's existing pattern of one file per device family under `src/runtime/devices/` — the planner may keep it flatter in `qemu.rs` if the total line count stays manageable; the codebase currently favors small, focused files.)

### Pattern 1: Segmented Builder with `Vec<String>` Fields

**What:** Instead of emitting tokens in traversal order, every handler pushes into one of nine typed `Vec<String>` buckets on a builder struct. `Display` (or a final `build()`) concatenates buckets in a fixed order.
**When to use:** Any time output order must differ from input traversal order — exactly QEMU-02/03's requirement.
**Example (pattern, not copy-paste — no official QEMU Rust binding exists to cite; this mirrors the project's own existing `Q35Chipset::fmt` sorted-then-rendered pattern):**
```rust
// Source: pattern derived from src/runtime/q35.rs's existing sort-then-render Display impl
pub struct QemuCommandLineBuilder {
    machine: Vec<String>,
    firmware: Vec<String>,
    drives: Vec<String>,
    netdevs: Vec<String>,
    chardevs: Vec<String>,
    tpm: Vec<String>,
    objects: Vec<String>,
    devices: Vec<String>,
    misc: Vec<String>,
}

impl QemuCommandLineBuilder {
    fn push_drive(&mut self, arg: String) { self.drives.push(arg); }
    fn push_device(&mut self, arg: String) { self.devices.push(arg); }
    // ... one push_* per segment
}

impl std::fmt::Display for QemuCommandLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for seg in [&self.machine, &self.firmware, &self.drives, &self.netdevs,
                    &self.chardevs, &self.tpm, &self.objects, &self.devices, &self.misc] {
            for arg in seg { write!(f, " {}", arg)?; }
        }
        Ok(())
    }
}
```

### Pattern 2: Two-Level Dispatch (Root + Nested Bus)

**What:** `Runtime::root_devices()` only yields 7 kinds (`RootDeviceKind`); storage/network/PCI-passthrough/USB devices live *inside* `Chipset::Q35(Q35Chipset)`'s five bus `HashMap`s. The emitter must dispatch twice: once over root devices, once (per `Q35Chipset` found) over each bus map.
**When to use:** Always, for this phase — this is not optional, it's how the existing data model is shaped [VERIFIED: `src/runtime.rs`, `src/runtime/q35.rs`].
**Example — mirrors the *existing* `format_pcie_device`/`format_pci_device`/`format_storage_device` functions in `q35.rs`, which already do exactly this kind of `device_kind()`-then-`downcast_ref()` dispatch for `Display`:**
```rust
// Source: pattern lifted directly from src/runtime/q35.rs::format_pcie_device
fn emit_pcie_device(b: &mut QemuCommandLineBuilder, ctx: &QemuContext, address: &PcieAddress, device: &dyn PcieDevice) {
    match device.device_kind() {
        PcieBusDeviceKind::HostPci => {
            let host_pci = device.as_any().downcast_ref::<HostPci>().unwrap();
            emit_hostpci(b, address, host_pci);
        }
        PcieBusDeviceKind::Ivshmem => {
            let ivshmem = device.as_any().downcast_ref::<Ivshmem>().unwrap();
            emit_ivshmem(b, ivshmem);
        }
        PcieBusDeviceKind::VirtioNet => {
            let nic = device.as_any().downcast_ref::<VirtioNetPcie>().unwrap();
            emit_virtio_net(b, ctx, address, nic);
        }
        PcieBusDeviceKind::PvScsi => {
            let pvscsi = device.as_any().downcast_ref::<PvScsi>().unwrap();
            emit_pvscsi(b, ctx, address, pvscsi); // recurses into pvscsi.scsi_bus()
        }
    }
}
```

### Anti-Patterns to Avoid

- **Single-pass inline emission (Pitfall 5/6):** Writing `-drive ...` and `-device ...` to the same output string as you traverse devices in `HashMap` iteration order. `HashMap` iteration order is not even stable across runs — this would produce nondeterministic *and* invalid cmdlines. The existing `q35.rs` already demonstrates the fix (sort entries by address before rendering) for `Display`; the emitter must do the same but with segment separation on top.
- **Trusting `HashMap` iteration order for anything user-visible:** `Q35Chipset`'s bus maps are `HashMap`, not `BTreeMap`. Any place order matters (drive/device pairing per bus, deterministic test assertions) must explicitly `.sort_by_key()` on the address first — exactly as `Q35Chipset::fmt` already does (`pcie_entries.sort_by_key(...)`).
- **Tokenizing or re-joining `RawArgs`:** The type's own doc comment says "never split, tokenize, or reorder it" — treat `.0` as an opaque string, `push_str`/`push` it once into `misc` (or append after all segments as the roadmap allows), never `split(' ')` and re-`join`.
- **Using `QemuConversionError` for missing `QemuContext` fields:** Per D-06, that's a programmer error → `.expect()`/`panic!`, not a typed error variant. Reserve `QemuConversionError` for genuinely malformed Runtime data (e.g., a `HostPci` with an empty `functions` vec, which should never happen post-Phase-2 but is theoretically representable).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Shell-safe argument quoting | A custom quoting/escaping function for spaces in paths | Nothing needed *in this phase* — Phase 7 produces a `Display`-able string or `Vec<String>` of tokens; actual process spawning (Phase 8, via `std::process::Command`) should pass args as a `Vec<&str>`/`Vec<String>` directly to `.arg()`/`.args()`, which bypasses shell parsing entirely and needs no escaping. Do not have Phase 7 pre-escape for shell — this only causes double-escaping bugs when Phase 8 uses `Command::arg()`. | `Command::arg()` is exec()-based, not shell-based, in Rust's stdlib — no shell involved, no escaping needed |
| Deterministic ordering over `HashMap` | A custom "insertion order tracking" wrapper around `HashMap` | Sort by the existing address key (`PcieAddress`, `SataAddress`, etc.) at emission time, mirroring `Q35Chipset::fmt`'s existing pattern | Already-implemented, already-tested convention in this exact codebase |
| ID/label string generation | Ad hoc `format!` calls scattered through every handler | One small `bootindex.rs`/`ids.rs` module with one function per bus family (see Critical Gap table) | Centralizes the one truly `[ASSUMED]`/fragile piece of this phase so it's easy to find and fix when a second real-world corpus file surfaces an edge case |

**Key insight:** Nothing in this phase needs an external crate. The two subtle risks are (1) ordering, which is solved by sorting on the address key exactly like the existing `Display` impls already do, and (2) ID-label reconstruction for bootindex, which is a genuine open modeling question (see Critical Gap) rather than a "don't hand-roll" library problem.

## Common Pitfalls

### Pitfall 1: Drive/device ordering (roadmap Pitfall 5 — FATAL)
**What goes wrong:** `-device scsi-hd,...,drive=drive-scsi0` appears before `-drive if=none,id=drive-scsi0,...` in the emitted string.
**Why it happens:** Emitting during a single traversal pass instead of into separate segments.
**How to avoid:** The `QemuCommandLineBuilder`'s 9-field segment design — `drives` is a field distinct from `devices`, and `Display`/`build()` always concatenates `drives` before `devices` regardless of traversal order.
**Warning signs:** Any code path that does `write!(output, "-drive ...")` immediately followed inline by `write!(output, "-device ...")` in the same function during a single top-to-bottom pass over devices.

### Pitfall 2: netdev ordering (roadmap Pitfall 6)
**What goes wrong:** Same class of bug for `-netdev type=tap,id=net0,...` vs `-device virtio-net-pci,...,netdev=net0,...`.
**Why it happens:** Same root cause as Pitfall 1.
**How to avoid:** `netdevs` segment always precedes `devices` segment in the fixed `Display` order.
**Warning signs:** Same as Pitfall 1, for network devices specifically.

### Pitfall 3: bootindex gap (roadmap Pitfall 11) — see Critical Gap section above
**What goes wrong:** Planner writes Task 07-03 assuming `Runtime` already exposes a boot order, discovers mid-implementation that it doesn't, and either stalls or invents an ad hoc mechanism that conflicts with Phase 9's exact-match goal.
**Why it happens:** The roadmap's plan-bullet text describes the desired *behavior* without having verified the *data model* supports it.
**How to avoid:** Resolve explicitly in planning (see Critical Gap recommendation) before writing Task 07-03.
**Warning signs:** Any attempt to call a `.boot_order()` getter on `Runtime` that doesn't currently exist — this will simply fail to compile, which is actually the safest failure mode here.

### Pitfall 4: HostPci multifunction flag placement
**What goes wrong:** Emitting `multifunction=on` on the wrong function, or on both functions, or omitting it when a GPU has both `.0` audio and `.1` video functions (`x_vga=true` → `functions: vec![0, 1]`).
**Why it happens:** Copying the flag onto every function device instead of only the base (`.0`) function when a companion function exists.
**How to avoid:** Per the real sample (line 25-26 of `108.qemu.cmd.split`): `multifunction=on` appears **only** on `hostpci0.0` (the first function), never on `hostpci0.1`. Emit it conditionally: `if functions.len() > 1 && this_function == 0 { ",multifunction=on" } else { "" }`.
**Warning signs:** A test that checks token *presence* without checking *which* function token it's attached to would miss this bug — the ordering test (07-04) should assert on the specific device id, not just substring presence.

### Pitfall 5: `RawArgs` re-tokenization
**What goes wrong:** Splitting `RawArgs.0` on whitespace to "normalize" it, which corrupts values containing spaces (e.g. `-name "my vm"`-style, or any future value with embedded spaces) and definitely reorders cross-references inside the blob (e.g., felucia's raw args contain `-chardev spicevmc,id=vdagent,...` followed later by `-device virtserialport,chardev=vdagent,...` — an internal ordering dependency the phase must NOT disturb).
**Why it happens:** Treating `RawArgs` like every other device instead of as opaque pass-through.
**How to avoid:** `misc.push(raw_args.0.clone())` (or equivalent single push) — never iterate/split/rejoin.
**Warning signs:** Any `.split(' ')` or `.split_whitespace()` call touching `RawArgs.0`.

### Pitfall 6: EfiDisk dual-size confusion in the pflash drive line
**What goes wrong:** Using `logical_size` (e.g. `"4M"`) where `block_device_size_bytes` (e.g. `540672`) is needed, or vice versa.
**Why it happens:** Both fields describe "the EFI disk's size" but are structurally unrelated (already flagged in `src/runtime/efidisk.rs`'s doc comment).
**How to avoid:** The real sample's second pflash line is `-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=...,size=540672` — that `size=` is the **byte** size (`block_device_size_bytes`), not `logical_size`. `logical_size` (`"4M"`/`"4m"`) is Proxmox/ezkvm-schema metadata about the *pflash template variant* to select (e.g. picking `OVMF_VARS_4M.fd` vs a 2M variant) for the **first** pflash line (`unit=0`, the read-only code volume), not a value that appears literally in the second line's `size=`.
**Warning signs:** A test asserting a specific numeric size on the wrong pflash line.

### Pitfall 7: PvScsi/network devices imported from ezkvm-YAML path but not from Proxmox path (testing gotcha, not an emission bug)
**What goes wrong:** Assuming the felucia golden-fixture Runtime (built via `ProxmoxImporter`) contains network/USB devices, when in fact `ProxmoxImporter::into_runtime()` currently only imports scsi/sata/ide/virtio-disk/hostpci/efidisk/tpmstate/audio/rawargs — **not** `net` or `usb`, even though `ProxmoxVmConf` parses both.
**Why it happens:** Phase 4 (`ProxmoxImporter`) was scoped before Phase 2's full device set was finalized, or simply hasn't caught up yet — confirmed by `grep -n "usb\|net" src/config/proxmox/importer.rs` returning zero handler code for either.
**How to avoid:** Do not rely on the felucia fixture alone to prove QEMU-02's netdev-ordering clause; D-04 already mandates synthetic per-device unit tests for exactly this reason — make sure the plan's Task 07-04 (or equivalent) includes an explicit synthetic test constructing a `Runtime` with a `VirtioNetPcie` device directly via `Q35ChipsetBuilder::with_pcie_device`, not only via `ProxmoxImporter`.
**Warning signs:** A "coverage" test that only exercises the felucia conf and silently never touches the netdev-ordering assertion because there's no net device in the fixture Runtime to find.

## Code Examples

Real QEMU CLI syntax verified against `input/felucia/108.qemu.cmd.split` (a genuine Proxmox `qm`-generated commandline for a Windows 11 VM with EFI, TPM 2.0, GPU passthrough, ivshmem/Looking-Glass, SPICE, and virtio networking) [VERIFIED: repo file, real-world source]. ezkvm's own generated output is **not required to match this byte-for-byte** (per D-05) — these are shown to establish correct QEMU syntax shape and cross-reference semantics, not exact strings to reproduce.

### EfiDisk → pflash drive pair
```
-drive if=pflash,unit=0,format=raw,readonly=on,file=<OVMF_CODE_path>
-drive if=pflash,unit=1,id=drive-efidisk0,format=raw,file=<resolved_efidisk_path>,size=<block_device_size_bytes>
```
Two `-drive` lines, no `-device` — pflash is consumed directly by `-machine`'s firmware slots, not attached via a `-device`. `unit=0` is the read-only firmware code (from a fixed OVMF path resolved via `QemuContext`), `unit=1` is the writable EFI vars volume (the actual `EfiDisk.storage_volume`).

### TpmState → chardev + tpmdev + device triple
```
-chardev socket,id=tpmchar,path=<swtpm_socket_path>
-tpmdev emulator,id=tpmdev,chardev=tpmchar
-device tpm-tis,tpmdev=tpmdev
```
Three-way reference chain: `chardev` (id=tpmchar) → `tpmdev` (chardev=tpmchar, id=tpmdev) → `device` (tpmdev=tpmdev). All three must be ordered `chardev` < `tpmdev` < `device` — matches the roadmap's `chardevs` then `tpm` then `devices` segment order. `<swtpm_socket_path>` comes from `QemuContext`'s socket_paths (the actual `swtpm` process is Phase 8's responsibility to launch and bind that socket).

### HostPci → one vfio-pci device per function, multifunction on `.0` only
```
-device vfio-pci,host=0000:03:00.0,id=hostpci0.0,bus=ich9-pcie-port-1,addr=0x0.0,multifunction=on
-device vfio-pci,host=0000:03:00.1,id=hostpci0.1,bus=ich9-pcie-port-1,addr=0x0.1
```
One `-device vfio-pci` per entry in `HostPci.functions: Vec<u8>`. `host=<base_bdf>.<function>`. `id=hostpciN.<function>`. `multifunction=on` appears **only** when `functions.len() > 1` and only on the function-0 entry (see Pitfall 4). No `-drive` counterpart — passthrough devices have no drive/netdev reference to order against.

### AudioDevice → device + codec devices + audiodev backend
```
-device ich9-intel-hda,id=audiodev0,bus=pci.2,addr=0xc
-device hda-micro,id=audiodev0-codec0,bus=audiodev0.0,cad=0,audiodev=spice-backend0
-device hda-duplex,id=audiodev0-codec1,bus=audiodev0.0,cad=1,audiodev=spice-backend0
-audiodev spice,id=spice-backend0
```
`device_type` (`AudioDevice.device_type`, e.g. `"ich9-intel-hda"`) drives the first `-device` line; `driver` (`AudioDevice.driver`, e.g. `"spice"`) drives the `-audiodev <driver>,id=...` line and which codec devices are attached (per the runtime doc comment: "codec devices are derived from `device_type` at emit time, not stored in the Runtime" — for `ich9-intel-hda` + `spice` driver specifically, that's a fixed micro+duplex codec pair). `-audiodev` is a distinct top-level flag (not `-device`); it can live in `misc` or get its own handling — it's not itself drive/netdev-referenced so ordering relative to the device lines only matters insofar as `audiodev=spice-backend0` must resolve, meaning the `-audiodev` line must appear *somewhere* in the full argv (QEMU parses `-audiodev` independently of position relative to `-device`, but keeping it adjacent in `misc`/its own segment is cleanest).

### Ivshmem → object + device pair
```
-device ivshmem-plain,memdev=ivshmem0,bus=pcie.0
-object memory-backend-file,id=ivshmem0,share=on,mem-path=/dev/kvmfr0,size=128M
```
Reference direction is **inverted** relative to drive/netdev: the `-device` references `memdev=<id>`, and the *referenced* `-object` can legally appear **after** the device in real Proxmox output (see the felucia sample — object comes last). This is `[CITED: felucia/108.qemu.cmd.split, real Proxmox output]` evidence that QEMU's `-object`/`memdev=` reference does *not* have the same strict left-to-right ordering requirement as `-drive`/`drive=` and `-netdev`/`netdev=`. **However**, do not rely on this leniency — the roadmap's `objects` segment is listed *before* `devices` in the fixed emission order, and putting objects first is always safe (never actually forbidden by QEMU), so the planner should still emit `objects` before `devices` for consistency and defensiveness, even though the real-world reference shows QEMU tolerates the reverse for this specific pairing. `Ivshmem.id` (Runtime field) is the memdev id.

### Network → netdev + device pair
```
-netdev type=tap,id=net0,ifname=<iface>,script=...,downscript=...,vhost=on
-device virtio-net-pci,mac=BC:24:11:3A:21:B7,netdev=net0,bus=pci.0,addr=0x12,id=net0,rx_queue_size=1024,tx_queue_size=256,bootindex=102
```
`VirtioNetPcie.mac_address`, `.rx_queue_size`, `.tx_queue_size`, `.vhost` map directly to the corresponding flags. `resource` (an `Option<String>`, likely the tap/bridge interface name or similar) maps to the `-netdev` line's `ifname=`/bridge parameters — exact mapping depends on what values `resource` actually holds at runtime (not fully resolvable from the struct definition alone; **`[ASSUMED]`** that it's a bridge/tap identifier — verify against Phase 2/4/6 SUMMARY docs or existing round-trip tests during implementation). Note QEMU-server's `script=`/`downscript=` values are Proxmox-specific helper paths not modeled in `VirtioNetPcie` at all — ezkvm's own netdev line is expected to differ here per D-05 (not byte-identical).

### Storage (Ssd/Hdd/Cdrom on scsi/sata/ide bus) → drive + device pair
```
-drive file=/dev/vm1/vm-108-boot,if=none,id=drive-scsi0,discard=on,format=raw,cache=none,aio=io_uring,detect-zeroes=unmap
-device scsi-hd,bus=scsihw0.0,scsi-id=0,drive=drive-scsi0,id=scsi0,rotation_rate=1,bootindex=100
```
```
-drive if=none,id=drive-ide2,media=cdrom,aio=io_uring
-device ide-cd,bus=ide.1,unit=0,drive=drive-ide2,id=ide2,bootindex=101
```
`StorageDeviceType::{Ssd,Hdd,Odd}` selects the `-device` model string (`scsi-hd`/`ide-hd`/`ide-cd` etc. depending on bus+type) and whether `media=cdrom` appears on the `-drive` line. `rotation_rate=1` on the sample's SSD line signals "this is actually flash, not spinning disk" to the guest — a nice-to-have detail, not required for QEMU-01/02/03. `discard=on`/`ssd=1` styling is Proxmox-specific tuning; ezkvm's own drive options can be simpler per D-05.

### PvScsi controller device (no drive/netdev reference itself)
```
-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5
```
This is the SCSI *controller*, emitted once per `PvScsi` instance found in a bus map, before any of its attached `scsi-hd`/`scsi-cd` child devices reference `bus=scsihw0.0`.

## State of the Art

Not applicable in the traditional sense — QEMU's `-drive`/`-device` split-object CLI model has been stable since QEMU 1.x and is not undergoing active change; there's no "old way vs new way" to document here beyond what's already captured in Pitfalls. `-audiodev` (as opposed to inline `audiodev=` machine option) is the modern QEMU 4.2+ syntax and is what the felucia sample already uses — no further verification needed since ezkvm targets modern QEMU per `machine: pc-q35-8.1`/`meta: creation-qemu=8.1.5` in the sample conf.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `net{N}` label = ordinal position among `VirtioNetPcie` entries sorted by `PcieAddress` | Critical Gap table | Wrong bootindex assignment for multi-NIC VMs; low risk since only one NIC exists in the only available corpus file, and bootindex isn't a Phase 7 success criterion |
| A2 | `sata{port}` label follows the same `{bus}{index}` convention as scsi/ide | Critical Gap table | Same as A1 — SATA isn't present in the felucia sample at all, so this is unverified by any real data |
| A3 | `VirtioNetPcie.resource` maps to the `-netdev` line's interface/bridge identifier | Code Examples: Network section | If wrong, netdev emission produces a plausible-looking but semantically incorrect `-netdev` line; caught by Phase 9's boot-verification, but would be better caught by a Phase 7 unit test asserting the exact field→flag mapping once `resource`'s actual populated values are confirmed against Phase 2/6 summaries during implementation |
| A4 | Boot-order gap should be fixed by adding `boot_order: Vec<String>` to `Runtime` (vs. `QemuContext` or elsewhere) | Critical Gap | If the team prefers a different home for this data (e.g., keeping it entirely inside `QemuContext` as caller-supplied), the plan's task list changes; this is presented as a recommendation, not a locked decision — needs explicit planner/user sign-off |
| A5 | `-audiodev` line placement relative to `-device` lines is not order-sensitive | Code Examples: AudioDevice section | Low risk — QEMU's `-audiodev`/`audiodev=` reference resolution is a named global registry, not a positional stream like `-drive`, so this is a safe assumption, but not independently verified via QEMU source/docs in this session |

**If this table is empty:** N/A — assumptions exist and are listed above; several are LOW risk (don't gate Phase 7's 4 success criteria) and one (A4) is a real planning decision point already elevated in the Critical Gap section.

## Open Questions

1. **(RESOLVED — see D-07, Plan 07-01) Where does `boot_order` live, and who populates it?**
   - What we know: The data (`order=scsi0;ide2;net0`) is parsed by the Proxmox `.conf` parser into `ProxmoxVmConf.boot: Option<String>` today, but goes nowhere from there.
   - Resolution: CONTEXT.md's D-07 decided to implement full plumbing in this phase — `Runtime` gains a `boot_order: Vec<String>` field, `ProxmoxImporter` parses `order=a;b;c` into it (Plan 07-01), and Plan 07-04 implements device-id reconstruction + `bootindex` assignment consuming it. No longer an open question.

2. **(PARTIALLY RESOLVED — deliberately deferred, see Plan 07-02 prohibitions) What exactly does `VirtioNetPcie.resource` hold?**
   - What we know: It's `Option<String>`, used identically in both the Proxmox-import path (Plan 07-03 parses it from `net{N}`'s `bridge=` sub-option) and the ezkvm-YAML path (`builder.rs` line ~434, `parser.rs` line ~516).
   - What's unclear: Full semantic meaning (bridge name? tap device name? raw Proxmox `net0:` value fragment?) without reading Phase 2/6 SUMMARY.md in full detail (this research read the struct definition and construction call sites, but not every consuming test).
   - Resolution: Plan 07-02's `emit_virtio_net` does NOT read `nic.resource()` when building the `-netdev`/`-device` lines — this is a deliberate scope decision (documented in Plan 07-02's `must_haves.prohibitions`), matching D-05's "structural-only fidelity" testing strategy rather than full netdev flag parity. `resource` remains parsed-but-unconsumed at the emitter layer this phase; wiring it into the netdev line (if warranted) is left to a future phase once its exact semantics are confirmed against Phase 2/6 SUMMARY docs.

## Environment Availability

Not applicable — this phase has no external tool/service dependencies. It doesn't invoke `qemu-system-x86_64` itself (that's Phase 8/9); it only produces a string. `cargo test` is the only tool needed, already verified present via the existing project test suite (`.planning/codebase/TESTING.md`).

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` via `cargo test` (edition 2024) |
| Config file | none — no external test config; tests co-located with source per project convention |
| Quick run command | `cargo test qemu` (filters to this phase's test module names) |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| QEMU-01 | felucia-derived + synthetic Runtimes each produce a cmdline containing tokens for all 7 v1 device types plus pre-existing storage/network/usb | unit + fixture | `cargo test --test qemu_cmdline` (or inline `#[cfg(test)]` in `src/config/qemu.rs`) | ❌ Wave 0 — new test file/module |
| QEMU-02 | Every `drive=<id>`/`netdev=<id>` reference position in the emitted string follows the corresponding `-drive id=<id>`/`-netdev id=<id>` position | unit, string-position assertion | same as above | ❌ Wave 0 |
| QEMU-03 | `RawArgs` blob appears verbatim, in full, at/near the end, with internal token order unchanged | unit, substring + position assertion | same as above | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test qemu` (fast, scoped)
- **Per wave merge:** `cargo test` (full suite — must stay green; this phase touches a stub file (`qemu.rs`) with zero prior tests, so no regression risk to unrelated modules, but full-suite run is still cheap given current project size)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] New test module (either `src/config/qemu.rs`'s own `#[cfg(test)]` block, matching the codebase's co-location convention, or a new `tests/qemu_cmdline.rs` if the felucia-fixture test needs the full `ProxmoxImporter` pipeline as an integration-style test) — covers QEMU-01/02/03
- [ ] Ordering-assertion helper: a small `fn assert_precedes(haystack: &str, needle_before: &str, needle_after: &str)` tokenizing/`.find()`-based helper, reusable across all drive/netdev ordering assertions — not present anywhere in the codebase today
- [ ] Framework install: none — `cargo test` already works project-wide

## Security Domain

Not applicable in the ASVS sense — this phase has no authentication, session, network-input, or cryptography surface. The one input-validation-adjacent concern is that `QemuContext`'s paths (socket paths, storage paths) are, per D-06, caller-constructed/internal (not attacker-controlled), so no injection-style validation is required for this phase. (If `RawArgs` originates from an untrusted Proxmox `.conf` file in a future multi-tenant context, that's already handled upstream by the Phase 3/4 parser boundary — Phase 7 only ever sees an already-typed `Runtime`, never raw text.)

## Sources

### Primary (HIGH confidence)
- `src/runtime.rs`, `src/runtime/*.rs`, `src/runtime/devices/*.rs` — full Runtime/device-trait surface this phase builds against [VERIFIED: repo inspection]
- `src/config/qemu.rs` — current stub to be replaced [VERIFIED: repo inspection]
- `src/config/proxmox/importer.rs`, `src/config/proxmox/conf.rs` — confirms the boot-order and net/usb import gaps [VERIFIED: repo inspection]
- `input/felucia/108.qemu.cmd.split` — real Proxmox-generated `qemu-system-x86_64` commandline, used as ground truth for CLI flag syntax [VERIFIED: repo file, real-world tool output]
- `.planning/ROADMAP.md` §Phase 7, `.planning/REQUIREMENTS.md`, `.planning/phases/07-qemu-cmdline/07-CONTEXT.md`, `.planning/STATE.md` — locked decisions and requirement text [VERIFIED: repo inspection]
- `.planning/codebase/TESTING.md`, `.planning/codebase/CONVENTIONS.md` — established project test/code conventions [VERIFIED: repo inspection]

### Secondary (MEDIUM confidence)
- Device-id reconstruction formulas (scsi/ide/net labeling) — derived by cross-referencing `input/felucia/108.qemu.cmd.split` against `input/felucia/108.conf`'s `boot:` line; only one corpus file available so patterns for SATA and multi-NIC cases are extrapolated, not directly observed.

### Tertiary (LOW confidence)
- None — no WebSearch/external-only claims were needed for this phase; all findings were verifiable directly against the repo.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies, pure stdlib
- Architecture: HIGH — read directly from source (`Runtime`, `Q35Chipset`, bus traits, existing `Display` impls)
- Pitfalls: HIGH for ordering (Pitfalls 1/2/4/5/6), MEDIUM for the boot-order gap (Pitfall 3 — the gap itself is verified, but the *fix* is a recommendation, not yet a locked decision)

**Research date:** 2026-07-24
**Valid until:** No expiry pressure — this is an internal, stable codebase; re-research only needed if Phase 2/4/6 (Runtime/importer) are revisited before Phase 7 executes.
