---
phase: 02-runtime-model
plan: 01
type: execute
wave: 1-3
depends_on: [01-04]
files_modified:
  - src/runtime/efidisk.rs
  - src/runtime/tpmstate.rs
  - src/runtime/audio.rs
  - src/runtime/spice.rs
  - src/runtime/rawargs.rs
  - src/runtime/devices/hostpci.rs
  - src/runtime/devices/ivshmem.rs
  - src/runtime/devices.rs
  - src/runtime/pcie.rs
  - src/runtime/q35.rs
  - src/runtime.rs
  - tests/runtime_phase2.rs
autonomous: true
requirements:
  - RUNT-01
  - RUNT-02
  - RUNT-03
  - RUNT-04
  - RUNT-05
  - RUNT-06
  - RUNT-07

must_haves:
  truths:
    - "`EfiDisk`, `TpmState`, `AudioDevice`, `SpiceDisplay`, `RawArgs` implement `RootDevice` and return correct `RootDeviceKind` variants from `device_kind()`"
    - "`HostPci` and `Ivshmem` implement `PcieDevice` and return correct `PcieBusDeviceKind` variants from `device_kind()`"
    - "`EfiDisk` stores `logical_size: String` and `block_device_size_bytes: Option<u64>` as two independent fields — never derived from each other (Pitfall: dual-size)"
    - "`HostPci` stores `base_bdf: String` and `functions: Vec<u8>` — never expanded to per-function structs (Pitfall: multi-function)"
    - "`RawArgs(String)` is a tuple newtype wrapping the opaque string verbatim — no tokenization (Pitfall: args ordering)"
    - "`RuntimeBuilder` gains `with_efidisk`, `with_tpmstate`, `with_audio_device`, `with_spice_display`, `with_raw_args` builder methods using the consuming-`mut self` pattern"
    - "`Q35ChipsetBuilder` gains `with_host_pci(idx: u8, device: Arc<dyn PcieDevice>)` and `with_ivshmem(idx: u8, device: Arc<dyn PcieDevice>)` methods"
    - "`format_pcie_device()` in `q35.rs` gains exactly two new `downcast_ref` arms (HostPci, Ivshmem); all other new device dispatch uses `device_kind()` match arms; no `downcast_ref` in any of the seven new device struct files"
  artifacts:
    - src/runtime/efidisk.rs
    - src/runtime/tpmstate.rs
    - src/runtime/audio.rs
    - src/runtime/spice.rs
    - src/runtime/rawargs.rs
    - src/runtime/devices/hostpci.rs
    - src/runtime/devices/ivshmem.rs
    - tests/runtime_phase2.rs
  key_links:
    - "`RootDeviceKind` in `src/runtime.rs` gains five new variants: EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs (alongside existing Memory, Chipset)"
    - "`PcieBusDeviceKind` in `src/runtime/pcie.rs` gains two new variants: HostPci, Ivshmem (alongside existing VirtioNet, PvScsi)"
    - "All seven new types are re-exported from `src/runtime.rs` via `pub use` so downstream phases import from one location"
    - "`format_pcie_device()` in `q35.rs` gains `downcast_ref::<HostPci>` and `downcast_ref::<Ivshmem>` arms — the ONLY two `downcast_ref` additions; all other dispatch uses `device_kind()`"
---

<objective>
Phase 2 completes the v1 Runtime device vocabulary by adding seven first-class structs:
five `RootDevice` types (EfiDisk, TpmState, AudioDevice, SpiceDisplay, RawArgs) and two
`PcieDevice` types (HostPci, Ivshmem). After this phase every downstream conversion phase
(Proxmox importer, YAML, QEMU emitter) has a complete, stable target API to write against.

Purpose: Phases 4–7 each need to construct or pattern-match one or more of these seven types.
Without them those phases cannot compile. Phase 2 ships the structs, wires them into the
builder API, and proves construction with a hand-written integration test.

Output: 4 execution plans across 3 waves. Wave 1 is a tracer that proves one RootDevice
(EfiDisk) works end-to-end. Wave 2 creates the remaining six struct files in parallel (no
shared file conflicts). Wave 3 wires the remaining six types into runtime.rs and q35.rs and
adds the felucia/108.conf construction test.
</objective>

<execution_context>
@.github/gsd-core/workflows/execute-plan.md
@.github/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/phases/02-runtime-model/02-RESEARCH.md
@.planning/phases/02-runtime-model/02-PATTERNS.md
</context>

<!-- ===================================================================
     WAVE MAP (read this first before executing any plan)
     ===================================================================
     Wave 1:  Plan 02-01  (tracer — EfiDisk end-to-end incl. runtime.rs wiring)
     Wave 2:  Plan 02-02  (TpmState + AudioDevice + SpiceDisplay + RawArgs structs)
              Plan 02-03  (HostPci + Ivshmem structs + devices.rs + pcie.rs) — PARALLEL
     Wave 3:  Plan 02-04  (wire remaining 6 types into runtime.rs + q35.rs + test)

     File overlap check (same-wave plans must not share files):
       Wave 1: 02-01 → src/runtime/efidisk.rs (new), src/runtime.rs
       Wave 2:
         02-02 → src/runtime/tpmstate.rs (new), src/runtime/audio.rs (new),
                 src/runtime/spice.rs (new), src/runtime/rawargs.rs (new)
         02-03 → src/runtime/devices/hostpci.rs (new), src/runtime/devices/ivshmem.rs (new),
                 src/runtime/devices.rs, src/runtime/pcie.rs
         → no overlap ✓
       Wave 3: 02-04 → src/runtime.rs, src/runtime/q35.rs, tests/runtime_phase2.rs
     =================================================================== -->

<tasks>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 02-01 · Wave 1 · TRACER
     EfiDisk end-to-end: struct + RootDeviceKind variant + runtime.rs wiring
     Proves the RootDevice pattern works with device_kind() before building the rest.
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 02-01 — Tracer: EfiDisk end-to-end (struct + runtime.rs wiring)

```
wave: 1
depends_on: [01-04]
files_modified: [src/runtime/efidisk.rs, src/runtime.rs]
autonomous: true
requirements: [RUNT-01]
```

**Rationale:** EfiDisk is the most structurally interesting RootDevice (dual-size pitfall, optional
`efitype`, optional `ms_cert`). Proving it compiles end-to-end — struct definition, trait impl
with `device_kind()`, mod wiring, `pub use`, and builder method — validates the template every
subsequent RootDevice will follow. One `cargo check` at the end confirms the whole stack from
field types through RuntimeBuilder compiles cleanly.

<task type="tracer">
  <name>02-01-T1: EfiDisk struct + RootDeviceKind::EfiDisk variant + runtime.rs wiring</name>
  <files>src/runtime/efidisk.rs, src/runtime.rs</files>
  <action>
**Create `src/runtime/efidisk.rs`** — new file, following the `src/runtime/memory.rs` pattern:

Imports (lines 1–4 of new file):
- `use derive_getters::Getters;`
- `use derive_new::new;`
- `use crate::runtime::{RootDevice, RootDeviceKind};`

Struct definition — `#[allow(dead_code)]` + `#[derive(Debug, Clone, Getters, new)]`:
- `storage_volume: String` — the `<pool>:<name>` reference verbatim from `.conf` (e.g. `"vm1-pool:vm-108-efidisk"`)
- `efitype: Option<String>` — `Some("4m")` for 4 MiB OVMF, `None` for legacy 2M format (absent in corpus)
- `pre_enrolled_keys: bool` — `true` when `pre-enrolled-keys=1` is present
- `ms_cert: Option<String>` — `Some("2023")` when `ms-cert=` present; `None` otherwise
- `logical_size: String` — the `size=` value from `.conf`, kept verbatim as a string (e.g. `"4M"`)
- `block_device_size_bytes: Option<u64>` — physical NVRAM image size in bytes (e.g. `540672`);
  `None` until Phase 7 resolves from the host's OVMF image; **do NOT derive this from `logical_size`**

⚠️ DUAL-SIZE PITFALL: `logical_size` ("4M") and `block_device_size_bytes` (540672) are
   structurally unrelated. 4 MiB ≠ 540672 bytes. Never convert between them.

RootDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn get_name(&self) -> &str { "efidisk" }`
- `fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::EfiDisk }` — requires the variant added below

**Edit `src/runtime.rs`** — four targeted changes:

1. **Add `mod efidisk;`** — insert after the last existing `mod` declaration line
   (currently `mod usb;`). Add: `mod efidisk;`

2. **Extend `pub use` for EfiDisk** — on the `pub use devices::{...}` line (currently line 15),
   add `EfiDisk` to the list. Alternatively, add a separate line:
   `pub use efidisk::EfiDisk;`
   Place it alongside the other top-level `pub use` statements (lines 14–24).

3. **Add `RootDeviceKind::EfiDisk` variant** — `RootDeviceKind` was defined by Phase 1 in this
   file with variants `Memory` and `Chipset`. Add `EfiDisk,` after `Chipset,`.
   (If Phase 1 is not yet complete, check current source; if `RootDeviceKind` does not exist yet,
   create it here with `Memory`, `Chipset`, `EfiDisk` as the initial three variants — subsequent
   plans will add more variants to this same enum. Also check the `RootDevice` trait definition
   in `src/runtime.rs`: if `fn device_kind(&self) -> RootDeviceKind;` is absent from the trait
   body, add it now AND implement it on `Memory` → `RootDeviceKind::Memory` and `Chipset` →
   `RootDeviceKind::Chipset` before adding the EfiDisk impl.)

4. **Add `with_efidisk` to RuntimeBuilder** — following the consuming-`mut self` pattern that
   Phase 1 establishes for `with_memory` and `with_chipset`. Add directly after `with_chipset`:
   - Signature: `pub fn with_efidisk(mut self, efidisk: EfiDisk) -> Self`
   - Body: `self.root_devices.push(Arc::new(efidisk)); self`
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -10</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `src/runtime/efidisk.rs` exists and contains `pub struct EfiDisk` with all six fields.
    `grep "EfiDisk" src/runtime.rs` returns at least three matches: the `RootDeviceKind` variant,
    the `mod efidisk;` line, and the `pub use efidisk::EfiDisk` line (or devices line).
    `grep "with_efidisk" src/runtime.rs` returns the builder method.
    No `downcast_ref` added in this plan.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 02-02 · Wave 2 · depends_on: [02-01] — parallel with 02-03
     RootDevice struct files: TpmState, AudioDevice, SpiceDisplay, RawArgs
     (struct files only — wiring into runtime.rs deferred to Plan 02-04)
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 02-02 — RootDevice structs: TpmState, AudioDevice, SpiceDisplay, RawArgs

```
wave: 2
depends_on: [02-01]
files_modified:
  - src/runtime/tpmstate.rs
  - src/runtime/audio.rs
  - src/runtime/spice.rs
  - src/runtime/rawargs.rs
autonomous: true
requirements: [RUNT-02, RUNT-05, RUNT-06, RUNT-07]
```

**Rationale:** All four follow the identical `src/runtime/memory.rs` RootDevice template
established by the tracer. They are created as unwired struct files in this plan; Plan 02-04
adds their `mod`, `pub use`, enum variants, and builder methods to `src/runtime.rs`. No file
in this plan overlaps with Plan 02-03's files.

<task type="auto">
  <name>02-02-T1: Create TpmState and AudioDevice struct files</name>
  <files>src/runtime/tpmstate.rs, src/runtime/audio.rs</files>
  <action>
**Create `src/runtime/tpmstate.rs`** — new file, RootDevice template:

Imports: `use derive_getters::Getters; use derive_new::new; use crate::runtime::{RootDevice, RootDeviceKind};`

Struct `#[allow(dead_code)] #[derive(Debug, Clone, Getters, new)] pub struct TpmState`:
- `storage_volume: String` — pool:volume reference (e.g. `"vm1-pool:vm-108-tpmstate"`)
- `version: String` — TPM version string verbatim (e.g. `"v2.0"`); drives `--tpm2` flag in Phase 8

RootDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn get_name(&self) -> &str { "tpmstate" }`
- `fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::TpmState }`

Note: The `size` field seen in Proxmox conf (`size=4M`) is invariant across the entire corpus
and carries no information not already implicit in `version = "v2.0"`. Do NOT add a `size`
field to the struct; it will be emitted as a constant by the Phase 7 QEMU emitter.
swtpm launch is a Phase 8 concern — this struct only models the configuration.

---

**Create `src/runtime/audio.rs`** — new file, RootDevice template:

Imports: same three lines as TpmState.

Struct `#[allow(dead_code)] #[derive(Debug, Clone, Getters, new)] pub struct AudioDevice`:
- `device_type: String` — QEMU device model name (e.g. `"ich9-intel-hda"`)
- `driver: String` — audio backend driver (e.g. `"spice"`, `"pa"`, `"pipewire"`, `"none"`)

RootDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn get_name(&self) -> &str { "audio_device" }`
- `fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::AudioDevice }`

Note: Do NOT add bus address fields or codec fields. Bus placement is fixed by Q35 conventions
(`pci.2,addr=0xc`); the QEMU emitter determines placement. Codec devices (`hda-micro`,
`hda-duplex`) are derived from `device_type` at emit time, not stored in the Runtime.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0 (files compile but are not yet referenced — that is expected).
    NOTE: This cargo check only verifies no regressions in existing code; the new struct files are NOT compiled until Wave 3 wires them into the module tree. Type errors in these files will surface at 02-04.
    Add per-file spot-check: `rustc --edition 2024 --crate-type lib src/runtime/tpmstate.rs 2>&1 | tail -5` and `rustc --edition 2024 --crate-type lib src/runtime/audio.rs 2>&1 | tail -5` — each must exit 0.
    `src/runtime/tpmstate.rs` exists with `TpmState`, two fields, RootDevice impl.
    `src/runtime/audio.rs` exists with `AudioDevice`, two fields, RootDevice impl.
  </done>
</task>

<task type="auto">
  <name>02-02-T2: Create SpiceDisplay and RawArgs struct files</name>
  <files>src/runtime/spice.rs, src/runtime/rawargs.rs</files>
  <action>
**Create `src/runtime/spice.rs`** — new file, RootDevice template:

Imports: `use derive_getters::Getters; use derive_new::new; use crate::runtime::{RootDevice, RootDeviceKind};`

Struct `#[allow(dead_code)] #[derive(Debug, Clone, Getters, new)] pub struct SpiceDisplay`:
- `port: Option<u16>` — TCP port for SPICE connections (e.g. `Some(5903)`); `None` when using Unix socket mode
- `addr: Option<String>` — listen address (e.g. `Some("0.0.0.0".to_string())`); `None` means localhost
- `disable_ticketing: bool` — `true` in all corpus examples (no password required)
- `gl: bool` — `false` in corpus; `true` when `gl=on` (GPU-accelerated rendering)
- `rendernode: Option<String>` — DRM render node path (e.g. `Some("/dev/dri/renderD128")`); `None` when `gl=false`
- `clipboard: bool` — `false` by default; `true` when clipboard sharing enabled

RootDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn get_name(&self) -> &str { "spice_display" }`
- `fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::SpiceDisplay }`

Note: Phase 4 imports `SpiceDisplay` by extracting `-spice ...` from the `args:` string. Phase 2
only provides the struct; no extraction logic belongs here.

---

**Create `src/runtime/rawargs.rs`** — new file, tuple newtype:

Imports: `use crate::runtime::{RootDevice, RootDeviceKind};`

Struct definition — do NOT add `Getters` or `new` derives, and do NOT add `#[derive(Clone, ...)]` beyond what is needed:
`#[allow(dead_code)] #[derive(Debug, Clone)] pub struct RawArgs(pub String);`

⚠️ RAWARGS PITFALL: The inner `String` is the verbatim value of the Proxmox `args:` field
   (after Phase 4 extracts SpiceDisplay and Ivshmem). Do NOT attempt to split, tokenize, or
   restructure the string — cross-references like `-chardev spicevmc,id=vdagent` and
   `-device virtserialport,chardev=vdagent` depend on their relative order; reordering breaks them.
   Access is via `.0` (direct field access), not a getter.

RootDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn get_name(&self) -> &str { "raw_args" }`
- `fn device_kind(&self) -> RootDeviceKind { RootDeviceKind::RawArgs }`
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `src/runtime/spice.rs` exists with `SpiceDisplay`, six fields, RootDevice impl.
    `src/runtime/rawargs.rs` exists with `RawArgs(pub String)` tuple newtype, RootDevice impl.
    No `Getters` or `new` derive on `RawArgs` — inner string accessed via `.0`.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 02-03 · Wave 2 · depends_on: [02-01] — parallel with 02-02
     PcieDevice struct files: HostPci, Ivshmem + wire devices.rs + pcie.rs variants
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 02-03 — PcieDevice structs: HostPci + Ivshmem + devices.rs + pcie.rs variants

```
wave: 2
depends_on: [02-01]
files_modified:
  - src/runtime/devices/hostpci.rs
  - src/runtime/devices/ivshmem.rs
  - src/runtime/devices.rs
  - src/runtime/pcie.rs
autonomous: true
requirements: [RUNT-03, RUNT-04]
```

**Rationale:** `HostPci` and `Ivshmem` live in `src/runtime/devices/` (the PCIe bus sub-module),
not in `src/runtime/` directly. Their wiring destination is `src/runtime/devices.rs` (mod + pub use)
and `src/runtime/pcie.rs` (PcieBusDeviceKind variants) — neither file overlaps with Plan 02-02.
The `src/runtime.rs` pub-use extension for PCIe types is deferred to Plan 02-04 which also
handles all remaining `src/runtime.rs` edits in one place.

<task type="auto">
  <name>02-03-T1: Create HostPci and Ivshmem struct files</name>
  <files>src/runtime/devices/hostpci.rs, src/runtime/devices/ivshmem.rs</files>
  <action>
**Create `src/runtime/devices/hostpci.rs`** — new file, following `pcie_net.rs` PcieDevice template:

Imports:
- `use derive_getters::Getters;`
- `use derive_new::new;`
- `use crate::runtime::{PcieDevice, pcie::PcieBusDeviceKind};`

Struct `#[allow(dead_code)] #[derive(Debug, Clone, Getters, new)] pub struct HostPci`:
- `base_bdf: String` — host PCI address without function suffix (e.g. `"0000:03:00"`) OR with
  explicit function (e.g. `"0000:05:00.0"`); keep verbatim; Phase 4 populates `functions`
- `functions: Vec<u8>` — function numbers for this device; `vec![0, 1]` for multi-function GPU
  (when BDF has no function suffix); `vec![0]` for single-function (when BDF ends in `.0`);
  Phase 4 determines the actual value — Phase 2 just needs the field to exist
- `pcie: bool` — `true` when `pcie=1` in Proxmox conf (PCIe mode vs PCI compatibility mode)
- `x_vga: bool` — `true` when `x-vga=1` (VGA compatibility / primary GPU flag)
- `rombar: Option<bool>` — `None` = default (show ROM BAR); `Some(false)` = `rombar=0` (hide);
  `Some(true)` = `rombar=1` (explicit show)
- `romfile: Option<String>` — custom VBIOS ROM filename (e.g. `Some("vgabios-radeon-rx570.bin")`);
  `None` when not specified

⚠️ MULTI-FUNCTION PITFALL: Store ONE `HostPci` per Proxmox `hostpciN:` line. Never create two
   separate HostPci structs for `.0` and `.1` functions of the same device — that breaks the
   YAML round-trip back to a single Proxmox line. The `functions: Vec<u8>` field holds both.

PcieDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn device_kind(&self) -> PcieBusDeviceKind { PcieBusDeviceKind::HostPci }`

---

**Create `src/runtime/devices/ivshmem.rs`** — new file, PcieDevice template:

Imports: same three lines as hostpci.rs.

Struct `#[allow(dead_code)] #[derive(Debug, Clone, Getters, new)] pub struct Ivshmem`:
- `id: String` — correlating name between the QEMU `-device` and `-object` (e.g. `"ivshmem0"`);
  used as `memdev=<id>` in the device and `id=<id>` in the memory backend object
- `mem_path: String` — path to the shared memory backing (e.g. `"/dev/kvmfr0"` or `"/dev/shm/looking-glass"`)
- `size: String` — size string with unit suffix, kept verbatim (e.g. `"128M"`); drives
  `-object memory-backend-file,...,size=128M` at emit time

PcieDevice impl:
- `fn as_any(&self) -> &dyn std::any::Any { self }`
- `fn device_kind(&self) -> PcieBusDeviceKind { PcieBusDeviceKind::Ivshmem }`
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0 (files compile but are not yet in the module tree — expected).
    NOTE: This cargo check only verifies no regressions in existing code; the new struct files are NOT compiled until Wave 3 wires them in. Spot-check new files: `rustc --edition 2024 --crate-type lib src/runtime/devices/hostpci.rs 2>&1 | tail -5` and `rustc --edition 2024 --crate-type lib src/runtime/devices/ivshmem.rs 2>&1 | tail -5` — each must exit 0.
    `src/runtime/devices/hostpci.rs` exists with `HostPci`, six fields, PcieDevice impl.
    `src/runtime/devices/ivshmem.rs` exists with `Ivshmem`, three fields, PcieDevice impl.
    No `functions` expansion into separate structs.
  </done>
</task>

<task type="auto">
  <name>02-03-T2: Wire HostPci + Ivshmem into devices.rs; add PcieBusDeviceKind variants</name>
  <files>src/runtime/devices.rs, src/runtime/pcie.rs</files>
  <action>
**Edit `src/runtime/devices.rs`** — add two new device modules following the existing pattern
(currently: `mod pvscsi; mod pcie_net; mod pci_generic; mod usb_generic;` + matching pub uses):

Add after the existing `mod usb_generic;` line:
- `mod hostpci;`
- `mod ivshmem;`

Add after the existing `pub use usb_generic::*;` line:
- `pub use hostpci::*;`
- `pub use ivshmem::*;`

The `*` glob re-export is consistent with all existing entries in this file. This makes
`HostPci` and `Ivshmem` accessible as `crate::runtime::devices::HostPci` and `::Ivshmem`.

---

**Edit `src/runtime/pcie.rs`** — add two new variants to `PcieBusDeviceKind`:

`PcieBusDeviceKind` was defined by Phase 1 with variants `VirtioNet` and `PvScsi`.
Add two new variants to the enum body:
- `HostPci,` — for `HostPci` devices
- `Ivshmem,` — for `Ivshmem` shared memory devices

The existing variants (`VirtioNet`, `PvScsi`) must be preserved unchanged. New variants go
after `PvScsi,`.

If Phase 1 has not yet added `PcieBusDeviceKind` (check current `src/runtime/pcie.rs`):
create the enum now with all four variants: `VirtioNet`, `PvScsi`, `HostPci`, `Ivshmem`.
Then update `PcieDevice` trait to add: `fn device_kind(&self) -> PcieBusDeviceKind;`
And implement on existing types (`VirtioNetPcie` → `PcieBusDeviceKind::VirtioNet`,
`PvScsi` → `PcieBusDeviceKind::PvScsi`) — this is normally Phase 1 work but must not be
left incomplete if Phase 1 shipped without it.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep "hostpci\|ivshmem" src/runtime/devices.rs` returns four lines (mod + pub use for each).
    `grep "HostPci\|Ivshmem" src/runtime/pcie.rs` returns the two new enum variants.
    `src/runtime/pcie.rs` has no `downcast_ref` additions.
  </done>
</task>

<!-- ═══════════════════════════════════════════════════════════════════
     PLAN 02-04 · Wave 3 · depends_on: [02-02, 02-03]
     Wire remaining 6 types into runtime.rs + q35.rs + felucia-108 test
     ═══════════════════════════════════════════════════════════════════ -->

## Plan 02-04 — Wire remaining 6 types into runtime.rs + q35.rs + integration test

```
wave: 3
depends_on: [02-02, 02-03]
files_modified:
  - src/runtime.rs
  - src/runtime/q35.rs
  - tests/runtime_phase2.rs
autonomous: true
requirements: [RUNT-01, RUNT-02, RUNT-03, RUNT-04, RUNT-05, RUNT-06, RUNT-07]
```

**Rationale:** All struct files now exist but five of the seven types are not yet accessible
from `crate::runtime::*`. This plan makes all seven types available through the public surface,
adds the remaining builder methods, extends Q35ChipsetBuilder for PCIe types, and then validates
the complete picture with a hand-written construction test that mirrors the felucia/108.conf
device set.

<task type="auto">
  <name>02-04-T1: Wire TpmState, AudioDevice, SpiceDisplay, RawArgs into runtime.rs; extend HostPci/Ivshmem pub use</name>
  <files>src/runtime.rs</files>
  <action>
Make the following targeted additions to `src/runtime.rs`. Read the current file first and
apply each change in sequence. Do not rewrite the whole file — use scoped edits only.

**1. Add four new `mod` declarations** — after `mod efidisk;` (added by Plan 02-01), add:
- `mod tpmstate;`
- `mod audio;`
- `mod spice;`
- `mod rawargs;`

**2. Add four new `pub use` statements** — after `pub use efidisk::EfiDisk;`, add:
- `pub use tpmstate::TpmState;`
- `pub use audio::AudioDevice;`
- `pub use spice::SpiceDisplay;`
- `pub use rawargs::RawArgs;`

**3. Extend the `pub use devices::{...}` line** (currently lists GenericPciDevice, VirtioNetPcie,
PvScsi, etc.) to also include `HostPci` and `Ivshmem`. The PCIe device types live in
`src/runtime/devices/` and are re-exported by `src/runtime/devices.rs`, so adding them to this
existing line makes them accessible as `crate::runtime::HostPci`.

**4. Add five new variants to `RootDeviceKind`** — the enum was defined by Phase 1 (or Plan 02-01
for `EfiDisk`). Add to the enum body after `EfiDisk,`:
- `TpmState,`
- `AudioDevice,`
- `SpiceDisplay,`
- `RawArgs,`

**5. Add five new builder methods to `RuntimeBuilder`** — following the consuming-`mut self`
pattern established by Phase 1 for `with_memory` and `with_chipset`, and mirrored by Plan 02-01's
`with_efidisk`. Add after `with_efidisk`:
- `pub fn with_tpmstate(mut self, tpmstate: TpmState) -> Self { self.root_devices.push(Arc::new(tpmstate)); self }`
- `pub fn with_audio_device(mut self, audio: AudioDevice) -> Self { self.root_devices.push(Arc::new(audio)); self }`
- `pub fn with_spice_display(mut self, spice: SpiceDisplay) -> Self { self.root_devices.push(Arc::new(spice)); self }`
- `pub fn with_raw_args(mut self, raw_args: RawArgs) -> Self { self.root_devices.push(Arc::new(raw_args)); self }`

All five methods follow the identical `Arc::new(device)` push pattern. No Mutex involved
(Phase 1 removed it). Builder remains infallible — no `Result` return type needed.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep "with_tpmstate\|with_audio_device\|with_spice_display\|with_raw_args" src/runtime.rs`
    returns four builder method lines.
    `grep "TpmState\|AudioDevice\|SpiceDisplay\|RawArgs" src/runtime.rs` returns at least
    eight matches (mod, pub use, RootDeviceKind variant, builder method for each).
  </done>
</task>

<task type="auto">
  <name>02-04-T2: Add Q35ChipsetBuilder methods for HostPci + Ivshmem; extend format_pcie_device</name>
  <files>src/runtime/q35.rs</files>
  <action>
Make three targeted additions to `src/runtime/q35.rs`.

**1. Extend the `use crate::runtime::{...}` import block** — add `HostPci, Ivshmem` to the
existing import list (currently imports VirtioNetPcie, PvScsi, and others from crate::runtime).

**2. Add two builder methods to `Q35ChipsetBuilder`** — insert after the existing `with_usb_device`
method and before `build(self)`:

`with_host_pci(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self`:
- Uses `PcieAddress::new(idx, 0)` as the slot key — one root-port slot per `hostpciN` line.
  `idx` is the numeric suffix from `hostpci<idx>:` in the Proxmox conf (0-based).
- Body: `self.pcie_bus.insert(PcieAddress::new(idx, 0), device); self`

`with_ivshmem(mut self, idx: u8, device: Arc<dyn PcieDevice>) -> Self`:
- Uses `PcieAddress::new(32 + idx, 0)` as the slot key — offset by 32 to avoid collision with
  `hostpci` slots (which occupy 0–15 in practice). This is an internal slot-numbering decision;
  Phase 7 maps to actual QEMU bus addresses independently.
- Body: `self.pcie_bus.insert(PcieAddress::new(32 + idx, 0), device); self`

**3. Extend `format_pcie_device()`** — add two new `downcast_ref` arms after the existing
`VirtioNetPcie` arm (currently lines 163–172 of q35.rs). These are the ONLY two `downcast_ref`
additions in Phase 2; all other device dispatch uses `device_kind()`:

After the `virtio_net` arm, add:

```
if let Some(host_pci) = device.as_any().downcast_ref::<HostPci>() {
    return format!(
        "HostPci(base_bdf={:?}, functions={:?}, pcie={}, x_vga={}, rombar={:?}, romfile={:?})",
        host_pci.base_bdf(),
        host_pci.functions(),
        host_pci.pcie(),
        host_pci.x_vga(),
        host_pci.rombar(),
        host_pci.romfile(),
    );
}

if let Some(ivshmem) = device.as_any().downcast_ref::<Ivshmem>() {
    return format!(
        "Ivshmem(id={:?}, mem_path={:?}, size={:?})",
        ivshmem.id(),
        ivshmem.mem_path(),
        ivshmem.size(),
    );
}
```

Note: The `format!` string uses the getter method names from `derive_getters`. These return
references so the `:?` formatter works directly.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo check 2>&amp;1 | tail -5</automated>
  </verify>
  <done>
    `cargo check` exits 0.
    `grep "with_host_pci\|with_ivshmem" src/runtime/q35.rs` returns two builder method lines.
    `grep "HostPci\|Ivshmem" src/runtime/q35.rs` returns at least four matches
    (import, builder method ×2, downcast arm ×2).
    Total new `downcast_ref` additions across all of Phase 2 is exactly 2 (HostPci, Ivshmem).
  </done>
</task>

<task type="auto" tdd="true">
  <name>02-04-T3: felucia/108.conf device set construction test</name>
  <files>tests/runtime_phase2.rs</files>
  <behavior>
    - test_felucia_108_runtime_constructs: Build a Runtime using RuntimeBuilder with all seven
      new device types populated with field values matching input/felucia/108.conf; assert the
      Runtime contains exactly the expected number of root devices without panicking.
    - test_efidisk_dual_size_fields_are_independent: Construct EfiDisk with logical_size="4M" and
      block_device_size_bytes=Some(540672); assert both fields read back correctly and are not
      equal (540672 ≠ 4194304).
    - test_hostpci_multi_function_stored_as_vec: Construct HostPci with base_bdf="0000:03:00"
      and functions=vec![0, 1]; assert functions().len() == 2.
    - test_rawargs_string_is_verbatim: Construct RawArgs("hello -device foo,bar".to_string());
      assert the inner .0 equals "hello -device foo,bar" unchanged.
    - test_ivshmem_is_pcie_device_kind: Construct Ivshmem and call device_kind(); assert it
      returns PcieBusDeviceKind::Ivshmem.
  </behavior>
  <action>
Create `tests/runtime_phase2.rs` — an integration test file (not a unit test mod inside
src/), which allows importing from `ezkvm` as an external consumer.

The test file imports:
- `use ezkvm::runtime::{RuntimeBuilder, EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, SpiceDisplay, RawArgs};`
- `use ezkvm::runtime::{RootDeviceKind, pcie::PcieBusDeviceKind};`
- `use ezkvm::runtime::{Q35Chipset, Q35ChipsetBuilder, PcieAddress};`
- `use std::sync::Arc;`

Note: the crate name is determined from `Cargo.toml`'s `[package] name` field. Read it first
if uncertain; the current name is `ezkvm`.

**Test 1 — `test_felucia_108_runtime_constructs`**: Build a Runtime with values mirroring
`input/felucia/108.conf` active section:
- EfiDisk: storage_volume="vm1-pool:vm-108-efidisk", efitype=Some("4m"), pre_enrolled_keys=true,
  ms_cert=Some("2023"), logical_size="4M", block_device_size_bytes=Some(540672)
- TpmState: storage_volume="vm1-pool:vm-108-tpmstate", version="v2.0"
- AudioDevice: device_type="ich9-intel-hda", driver="spice"
- SpiceDisplay: port=Some(5903), addr=Some("0.0.0.0"), disable_ticketing=true, gl=false,
  rendernode=None, clipboard=false
- RawArgs: inner string = "-device virtio-serial-pci -chardev spicevmc,id=vdagent,name=vdagent"

Build the Runtime via RuntimeBuilder. Assert it does not panic. Assert `runtime.root_devices().len() == 5`
(five RootDevices above — HostPci and Ivshmem go into Q35ChipsetBuilder, not RuntimeBuilder).

Build a Q35ChipsetBuilder with:
- HostPci at idx=0: base_bdf="0000:03:00", functions=vec![0, 1], pcie=true, x_vga=true, rombar=None, romfile=None
- Ivshmem at idx=0: id="ivshmem0", mem_path="/dev/kvmfr0", size="128M"

Build the chipset. Assert it does not panic.

**Tests 2–5**: Write the four behavioral tests listed in `<behavior>` above. Each test is a
single `#[test]` fn constructing the minimum required struct and asserting one property.
Use `assert_eq!` with `.0` for RawArgs, getter methods for all other fields.

All five test functions must pass with `cargo test --test runtime_phase2`.
  </action>
  <verify>
    <automated>cd /home/hurenkam/Workspace/ezkvm_v4 &amp;&amp; cargo test --test runtime_phase2 -- --nocapture 2>&amp;1 | tail -20</automated>
  </verify>
  <done>
    `cargo test --test runtime_phase2` exits 0 with all five tests passing.
    `cargo test` (full suite) exits 0 — no regressions from Phase 1 or existing tests.
    `grep -r "downcast_ref" src/` returns only the two HostPci/Ivshmem arms in q35.rs
    plus any pre-existing sites from before Phase 2; no new sites in the seven device files.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| caller → struct constructors | All struct fields are caller-supplied; no validation at construction time in Phase 2 |
| Phase 4 → HostPci.functions | The `functions: Vec<u8>` field is populated by the Phase 4 importer; incorrect expansion (e.g. adding .2 when only .0 and .1 exist) would produce invalid QEMU arguments |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-02-01 | Tampering | EfiDisk.logical_size / block_device_size_bytes | medium | accept | Both are String/Option<u64> opaque stores; conversion logic lives in Phase 7 emitter, not here. The dual-size pitfall is architectural (not a security issue) — documented in struct comments and plan action. |
| T-02-02 | Information Disclosure | SpiceDisplay.disable_ticketing = false | low | accept | A Runtime with `disable_ticketing: false` could allow unauthenticated SPICE connections. Phase 2 only models the field; enforcement is a VM operator concern. Document the field semantics clearly. |
| T-02-03 | Tampering | RawArgs inner string | low | accept | RawArgs stores opaque QEMU arguments verbatim. A caller could inject arbitrary QEMU flags. This is by design (RUNT-07); trust boundary is at Phase 4 import, not at the struct level. |
| T-02-04 | Tampering | Q35ChipsetBuilder slot-index arithmetic | low | accept | PcieAddress::new(32 + idx, 0) for ivshmem could overflow if idx > 223. In practice `idx` comes from a bounded Proxmox config (max ~10 devices). No bounds check needed in Phase 2; Phase 4 importer should validate the source index. |
| T-02-SC | Tampering | No new crates | low | accept | Phase 2 adds zero new dependencies (all required crates — derive_getters, derive_new — are already in Cargo.lock). No slopcheck required. |
</threat_model>

<verification>
After all four plans execute in wave order:

1. `cargo check` — exits 0 (validates from Wave 1 tracer onward)
2. `cargo test` — exits 0; full suite passes including new `tests/runtime_phase2.rs`
3. All seven new type files exist under `src/runtime/` or `src/runtime/devices/`
4. `grep -r "struct EfiDisk\|struct TpmState\|struct HostPci\|struct Ivshmem\|struct AudioDevice\|struct SpiceDisplay\|struct RawArgs" src/` returns exactly seven matches
5. `grep -r "downcast_ref" src/` — new additions are exactly two: HostPci and Ivshmem arms in q35.rs
6. `grep "with_efidisk\|with_tpmstate\|with_audio_device\|with_spice_display\|with_raw_args" src/runtime.rs` — returns five builder methods
7. `grep "with_host_pci\|with_ivshmem" src/runtime/q35.rs` — returns two builder methods
</verification>

<success_criteria>
1. **All 7 structs compile** with correct field types, derives (`Debug, Clone, Getters, new`),
   and trait implementations (`RootDevice` or `PcieDevice` with `device_kind()` returning the
   correct enum variant)
2. **`RuntimeBuilder`** has `with_efidisk`, `with_tpmstate`, `with_audio_device`,
   `with_spice_display`, `with_raw_args` builder methods using the consuming-`mut self` pattern
3. **`Q35ChipsetBuilder`** has `with_host_pci(idx: u8, device: Arc<dyn PcieDevice>)` and
   `with_ivshmem(idx: u8, device: Arc<dyn PcieDevice>)` builder methods
4. **`cargo test` passes** — no regressions from Phase 1, all five `tests/runtime_phase2.rs`
   tests pass
5. **Dual-size integrity**: `EfiDisk.logical_size` and `EfiDisk.block_device_size_bytes` are
   never computationally related; test asserts they hold independent values
6. **Multi-function integrity**: `HostPci.functions` is a `Vec<u8>` holding both function
   numbers; test asserts `functions().len() == 2` for a multi-function device
7. **RawArgs verbatim**: `RawArgs.0` stores the inner string unchanged; test asserts round-trip
   identity
8. **No new `downcast_ref` sprawl**: exactly two new `downcast_ref` sites in q35.rs
   (`HostPci`, `Ivshmem`) — all other dispatch uses `device_kind()` match arms
</success_criteria>

<output>
Create `.planning/phases/02-runtime-model/02-01-SUMMARY.md` (and 02-02, 02-03, 02-04 counterparts)
when each plan finishes execution, following `.github/gsd-core/templates/summary.md`.
</output>

---

## Multi-Source Coverage Audit

| Source | ID | Item | Status | Covered By |
|--------|----|------|--------|------------|
| ROADMAP | Phase 2 goal | All 7 v1 device types as first-class Runtime structs | COVERED | Plans 02-01 – 02-04 |
| REQ | RUNT-01 | EfiDisk — efidisk0 with pre-enrolled keys and dual size fields | COVERED | Plan 02-01 (struct), Plan 02-04 (wiring + test) |
| REQ | RUNT-02 | TpmState — tpmstate0 at version 2.0 with storage volume | COVERED | Plan 02-02-T1, Plan 02-04-T1 |
| REQ | RUNT-03 | HostPci — hostpci0–N with pcie/x-vga/rombar/romfile; multi-function | COVERED | Plan 02-03-T1 (struct), Plan 02-04-T2 (q35 wiring), Plan 02-04-T3 (test) |
| REQ | RUNT-04 | Ivshmem — ivshmem-plain with size and name | COVERED | Plan 02-03-T1 (struct), Plan 02-04-T2 (q35 wiring), Plan 02-04-T3 (test) |
| REQ | RUNT-05 | AudioDevice — ich9-intel-hda with spice/pa/none driver | COVERED | Plan 02-02-T1, Plan 02-04-T1 |
| REQ | RUNT-06 | SpiceDisplay — port, gl, rendernode, clipboard | COVERED | Plan 02-02-T2, Plan 02-04-T1 |
| REQ | RUNT-07 | RawArgs — opaque verbatim string passthrough | COVERED | Plan 02-02-T2, Plan 02-04-T1 |
| RESEARCH | §EfiDisk | Dual-size pitfall: logical_size vs block_device_size_bytes non-derivable | COVERED | Plan 02-01-T1 action ⚠️ note + test 02-04-T3 |
| RESEARCH | §HostPci | Multi-function BDF: store base_bdf + Vec<u8>, not expanded structs | COVERED | Plan 02-03-T1 action ⚠️ note + test 02-04-T3 |
| RESEARCH | §RawArgs | Do NOT tokenize; wrap verbatim String | COVERED | Plan 02-02-T2 action ⚠️ note + test 02-04-T3 |
| RESEARCH | §AudioDevice | RootDevice (not PcieDevice) — bus placement implicit in Q35 | COVERED | Plan 02-02-T1 (RootDevice impl) |
| RESEARCH | device_kind() | Add RootDeviceKind/PcieBusDeviceKind variants per Phase 1 contract | COVERED | Plans 02-01 (EfiDisk), 02-02 (4 types), 02-03-T2 (2 PCIe variants), 02-04-T1 (4 more variants) |
| RESEARCH | §Ivshmem | Promoted from args: blob; Phase 4 handles extraction | COVERED | Plan 02-03-T1 note (no extraction in Phase 2) |
| CONTEXT | D-n/a | YOLO mode (no checkpoints required) | COVERED | All plans: `autonomous: true`, no checkpoint tasks |
| CONTEXT | D-n/a | Standard granularity (2-3 tasks per plan) | COVERED | 4 plans × 1–3 tasks each |

**No gaps found. All RUNT-01 through RUNT-07 requirements are covered.**
Deferred: swtpm launch (Phase 8), SpiceDisplay extraction from args: (Phase 4),
Ivshmem extraction from args: (Phase 4), EFI firmware path resolution (Phase 7).
These are explicitly out of scope for Phase 2 per RESEARCH.md "Don't Hand-Roll" table.
