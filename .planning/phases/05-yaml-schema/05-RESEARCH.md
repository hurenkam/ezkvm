# Phase 5: YAML Schema — Research

**Researched:** 2025-07-22
**Domain:** Rust serde + bespoke saphyr YAML pipeline, ezkvm schema extension
**Confidence:** HIGH (all findings verified against codebase source)

---

## Summary

Phase 5 extends the ezkvm YAML schema to represent all seven v1 Runtime device types:
EfiDisk, TpmState, HostPci, Ivshmem, AudioDevice, SpiceDisplay, and RawArgs. The
project uses a bespoke saphyr-backed serde pipeline (NOT standard `serde_yaml` crate
— the internal module `src/serde_yaml/` provides drop-in `from_str` / `to_string` /
`to_value` using saphyr's AST). Schema structs are plain serde structs; serialization
is fully automatic via `#[derive(Serialize, Deserialize)]`. A separate rendering layer
(`ToStyledYaml` trait in `compact_yaml.rs`) controls block vs flow style in the
emitted YAML.

**Five of the seven target Runtime types have NO schema representation at all** (EfiDisk,
AudioDevice, RawArgs have no schema; HostPci and Ivshmem are partially represented via
`PcieDeviceTypeSchema` variants that don't match the Runtime field layout). TpmState and
SpiceDisplay have partial matches in the existing schema but with field-name and
field-type mismatches that must be reconciled.

**Primary recommendation:** Follow the existing `PcieDeviceTypeSchema` tagged-enum
pattern for bus devices (HostPci, Ivshmem) and the existing `TpmSchema` flat-field
pattern for root devices. Use `from_serde()` shortcut in `ToStyledYaml` impls for all
new simple schema types — only implement manual `StyledYaml` trees where custom
flow/block formatting is required.

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| YAML-03 | YAML schema covers all v1 Runtime device types | Seven schema types identified; placement decisions documented below; round-trip test pattern documented in §Test Pattern |
</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Schema struct definitions | `src/config/ezkvm/schema/` | — | Matches existing location for all schema types |
| YAML serialization / deserialization | `src/serde_yaml/` (bespoke) | — | Automatic via serde derive; no custom code needed for new types |
| Styled YAML rendering | `src/config/ezkvm/file/compact_yaml.rs` | — | `ToStyledYaml` impls; use `from_serde()` shortcut for new types |
| Schema tree wiring (VirtualMachineSchema, HostSchema) | `src/config/ezkvm/schema/virtual_machine.rs`, `host.rs` | — | New device fields added as `Option<T>` fields with serde skip |
| Module registration | `src/config/ezkvm/schema.rs` | — | Add `mod` + glob `pub use` entry per new schema file |
| Round-trip tests | `src/config/ezkvm/runtime/parser.rs` `#[cfg(test)]` block | Integration in `tests/` | Matches existing test pattern |

---

## Standard Stack

### Core (already in project — no new dependencies needed)

| Library | Version | Purpose | Notes |
|---------|---------|---------|-------|
| `serde` | 1.0.228 | `Serialize`/`Deserialize` derive for all schema structs | `features = ["derive"]` already enabled |
| `saphyr` | 0.0.11 | YAML AST; emitter used in `compact_yaml.rs` | Internal `src/serde_yaml/` wraps saphyr |
| `derive-getters` | 0.5.0 | `#[derive(Getters)]` for schema structs | Used by all existing schema types |
| `derive-new` | 0.7.0 | `#[derive(new)]` for constructor | Used by all existing schema types |

**No new Cargo.toml dependencies are required for this phase.** [VERIFIED: Cargo.toml]

---

## Architecture Patterns

### System Architecture Diagram

```
YAML string
    │
    ▼ serde_yaml::from_str (src/serde_yaml/de.rs, saphyr AST → serde Deserializer)
    │
    ▼ ConfigSchema (src/config/ezkvm/schema/config.rs)
       ├── Metadata
       ├── HostSchema ───► DisplaySchema, AudioSchema, Vec<ResourceSchema>
       └── VirtualMachineSchema ──► MachineSchema, MemorySchema, TpmSchema,
                                    EfiDiskSchema (NEW), AudioDeviceSchema (NEW),
                                    SpiceDisplaySchema (NEW), raw_args (NEW),
                                    Vec<DeviceSchema>
                                         └── PcieDeviceSchema ──► PcieDeviceTypeSchema
                                               (Passthrough, HostPci NEW, Ivshmem NEW, ...)
    │
    ▼ ConfigSchema::to_styled_compact_yaml()
       ├── ToStyledYaml impls (compact_yaml.rs)
       └── emit_styled_yaml → YAML string
```

### Existing Schema File Pattern (representative: `src/config/ezkvm/schema/tpm.rs`) [VERIFIED: codebase]

```rust
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TpmSchema {
    Emulated { swtpm: SwtpmSchema },
    Passthrough { hwtpm: HwtpmSchema },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct SwtpmSchema {
    version: f32,
    resource: String,  // ← NOTE: mismatch with Runtime's storage_volume: String
}
```

Key observations:
- Untagged enums for "one of these shapes" (TpmSchema, AudioSchema, DeviceSchema, DisplaySchema)
- Tagged enums with `#[serde(tag = "type", rename_all = "snake_case")]` for device-type discriminants (PcieDeviceTypeSchema, IdeDeviceTypeSchema)
- Optional fields: `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Flattened embedding: `#[serde(flatten)]` on struct fields within a parent struct
- `#[derive(Getters, new)]` on concrete structs (not enums)

### Module Registration Pattern (`src/config/ezkvm/schema.rs`) [VERIFIED: codebase]

```rust
mod audio;
// ... existing mods ...
mod tpm;

#[allow(unused_imports)]
pub use {
    audio::*, /* ... */ tpm::*, virtual_machine::*,
};
```

New files follow identical pattern: add `mod foo;` line and add `foo::*` to the glob.

### `ToStyledYaml` Shortcut Pattern (`compact_yaml.rs`) [VERIFIED: codebase]

```rust
fn from_serde<T: Serialize>(value: &T) -> YamlOwned {
    serde_yaml::to_value(value).expect("failed to serialize value for styled yaml rendering")
}

// Simple types: delegate entirely to serde
entries.push((
    "my_field".to_string(),
    raw_node(from_serde(self.my_field()), StyleHint::FlowPreferred),
));
```

For new simple schema types (EfiDiskSchema, TpmStateSchema, etc.), call `from_serde(value)`
and wrap with `StyleHint::FlowPreferred`. Manual `StyledYaml` trees are only needed when
the type mixes block and flow sections (as `VirtualMachineSchema` does).

### Recommended Project Structure for New Files

```
src/config/ezkvm/schema/
├── efidisk.rs       # NEW: EfiDiskSchema
├── tpmstate.rs      # NEW: TpmStateSchema (separate from existing tpm.rs TpmSchema)
├── hostpci.rs       # NEW: HostPciSchema (or added to pcie.rs as PcieDeviceTypeSchema variant)
├── ivshmem.rs       # NEW: IvshmemSchema (or updated in pcie.rs)
├── audio_device.rs  # NEW: AudioDeviceSchema (separate from existing audio.rs AudioSchema)
├── spice_display.rs # NEW: SpiceDisplaySchema (or updated SpiceSchema in display.rs)
├── tpm.rs           # EXISTING — keep as-is (TpmSchema = host TPM backend config)
├── audio.rs         # EXISTING — keep as-is (AudioSchema = host audio driver)
├── display.rs       # EXISTING — keep as-is or extend SpiceSchema
└── schema.rs        # EXISTING — add new mod + pub use entries
```

---

## Existing Schema Types Inventory

All files in `src/config/ezkvm/schema/` and their current state: [VERIFIED: codebase]

| File | Types Exported | Status re Phase 5 |
|------|----------------|-------------------|
| `audio.rs` | `AudioSchema` (Alsa/PulseAudio/PipeWire) | KEEP — host audio driver, different from AudioDevice |
| `boot.rs` | `BootSchema` | Unchanged |
| `chipset.rs` | `ChipsetSchema`, `Q35ChipsetSchema`, `I440FXChipsetSchema` | Unchanged |
| `config.rs` | `ConfigSchema` | Unchanged (contains metadata + host + virtual_machine) |
| `cpu.rs` | `CpuSchema` | Unchanged |
| `device.rs` | `DeviceSchema` (enum: Pcie/Pci/Usb/Sata/Ide/Scsi) | Unchanged |
| `display.rs` | `DisplaySchema` (enum), `SpiceSchema`, `VncSchema`, etc. | SpiceSchema may need field additions |
| `guest_agent.rs` | `GuestAgent` | Unchanged |
| `host.rs` | `HostSchema` | May need new fields for AudioDevice / SpiceDisplay |
| `ide.rs` | `IdeDeviceSchema`, `IdeDeviceTypeSchema` | Unchanged |
| `machine.rs` | `MachineSchema` | Unchanged |
| `memory.rs` | `MemorySchema` | Unchanged |
| `meta.rs` | `Metadata` | Unchanged |
| `pci.rs` | `PciDeviceSchema`, `PciDeviceType`, `PciAddressSchema` | Unchanged |
| `pcie.rs` | `PcieDeviceSchema`, `PcieDeviceTypeSchema`, `PcieAddressSchema` | **EXTEND** — add HostPci variant, fix Ivshmem |
| `resources.rs` | `ResourceSchema`, `PcieDeviceResourceSchema`, etc. | Unchanged |
| `sata.rs` | `SataDeviceSchema`, `SataDeviceTypeSchema` | Unchanged |
| `scsi.rs` | `ScsiDeviceSchema`, `ScsiDeviceTypeSchema` | Unchanged |
| `tpm.rs` | `TpmSchema`, `SwtpmSchema`, `HwtpmSchema` | KEEP — covers swtpm emulation backend |
| `usb.rs` | `UsbDeviceSchema`, `UsbDeviceTypeSchema` | Unchanged |
| `virtual_machine.rs` | `VirtualMachineSchema` | **EXTEND** — add efidisk, raw_args (and possibly audio_device) fields |

---

## Runtime Types to Map — Full Field Inventory

### EfiDisk (`src/runtime/efidisk.rs`) [VERIFIED: codebase]

```rust
pub struct EfiDisk {
    storage_volume: String,           // pool:volume ref, e.g. "vm1-pool:vm-108-efidisk"
    efitype: Option<String>,          // e.g. "4m"
    pre_enrolled_keys: bool,          // true when Secure Boot keys pre-installed
    ms_cert: Option<String>,          // MS cert identifier, e.g. "2023"
    logical_size: String,             // human-readable string e.g. "4M"
    block_device_size_bytes: Option<u64>, // raw block device size, e.g. 540672
}
// ⚠️ DUAL-SIZE: logical_size ("4M") ≠ block_device_size_bytes (540672). Never convert.
```

**Proposed `EfiDiskSchema`** (new file `src/config/ezkvm/schema/efidisk.rs`):

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct EfiDiskSchema {
    storage_volume: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    efitype: Option<String>,
    pre_enrolled_keys: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ms_cert: Option<String>,
    logical_size: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    block_device_size_bytes: Option<u64>,
}
```

**Placement in VirtualMachineSchema**: New field `efidisk: Option<EfiDiskSchema>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

---

### TpmState (`src/runtime/tpmstate.rs`) [VERIFIED: codebase]

```rust
pub struct TpmState {
    storage_volume: String,   // e.g. "vm1-pool:vm-108-tpmstate"
    version: String,          // e.g. "v2.0"
}
// Note: `size` field from Proxmox conf is not stored; Phase 7 emits it as a constant
```

**Existing `TpmSchema` / `SwtpmSchema`** maps conceptually but with mismatches:

| SwtpmSchema field | TpmState field | Mismatch |
|-------------------|----------------|----------|
| `resource: String` | `storage_volume: String` | Field name differs |
| `version: f32` | `version: String` | Type differs (f32 vs String "v2.0") |

**Recommended approach**: Add a new `TpmStateSchema` in a new file `src/config/ezkvm/schema/tpmstate.rs` with field names that exactly mirror the Runtime struct. Keep `TpmSchema` in `tpm.rs` unchanged (it describes the TPM backend type, which is a different configuration concern).

```rust
// src/config/ezkvm/schema/tpmstate.rs
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct TpmStateSchema {
    storage_volume: String,
    version: String,
}
```

**Placement**: New optional field `tpmstate: Option<TpmStateSchema>` in VirtualMachineSchema (alongside the existing `tpm: Option<TpmSchema>` which controls the TPM backend — they are orthogonal).

---

### HostPci (`src/runtime/devices/hostpci.rs`) [VERIFIED: codebase]

```rust
pub struct HostPci {
    base_bdf: String,         // e.g. "0000:03:00" (domain:bus:slot, NO function)
    functions: Vec<u8>,       // e.g. [0, 1] for multi-function GPU+audio
    pcie: bool,               // true → PCIe mode
    x_vga: bool,              // true → primary GPU passthrough
    rombar: Option<bool>,
    romfile: Option<String>,
}
// ⚠️ MULTI-FUNCTION: Never split into two structs; functions: Vec<u8> is authoritative
```

**Existing `PcieDeviceTypeSchema::Passthrough`** — does NOT cover HostPci correctly:

```rust
Passthrough {
    resource: Option<String>,   // ← not the same as base_bdf
    host: Option<String>,       // ← partially overlaps base_bdf, but Optional and unnamed
    id: Option<String>,
    multifunction: Option<bool>,// ← not the same as functions: Vec<u8>
    rombar: Option<bool>,
    romfile: Option<String>,
}
// Missing: pcie: bool, x_vga: bool, functions: Vec<u8>
```

**Recommended approach**: Add a new `HostPci` variant to `PcieDeviceTypeSchema`:

```rust
// In src/config/ezkvm/schema/pcie.rs — add to PcieDeviceTypeSchema enum:
HostPci {
    base_bdf: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    functions: Vec<u8>,
    #[serde(default)]
    pcie: bool,
    #[serde(default)]
    x_vga: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rombar: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    romfile: Option<String>,
},
```

This fits naturally as a new tagged enum variant: `type: host_pci` in YAML.

---

### Ivshmem (`src/runtime/devices/ivshmem.rs`) [VERIFIED: codebase]

```rust
pub struct Ivshmem {
    id: String,           // e.g. "ivshmem0"
    mem_path: String,     // e.g. "/dev/kvmfr0"
    size: String,         // e.g. "128M"
}
```

**Existing `PcieDeviceTypeSchema::IvshmemPlain { resource: String }`** — does NOT match:

| IvshmemPlain field | Ivshmem field | Mismatch |
|--------------------|---------------|----------|
| `resource: String` | ??? | No direct match |
| — | `id: String` | Missing |
| — | `mem_path: String` | Missing |
| — | `size: String` | Missing |

> ⚠️ The ROADMAP describes IvshmemSchema as `(size_mb, name)` but the actual Runtime struct
> fields are `{ id: String, mem_path: String, size: String }`. The planner must use the
> actual Runtime field names, NOT the ROADMAP description.

**Recommended approach**: Replace `IvshmemPlain` variant in `PcieDeviceTypeSchema` with a properly-fielded `Ivshmem` variant:

```rust
// In src/config/ezkvm/schema/pcie.rs:
Ivshmem {
    id: String,
    mem_path: String,
    size: String,
},
```

**Note**: Removing `IvshmemPlain` is a breaking change to the existing schema. Assess whether `IvshmemPlain` is used anywhere in tests before removing it.

```bash
# Check IvshmemPlain usage
grep -r "IvshmemPlain" /home/hurenkam/Workspace/ezkvm_v4/src/ /home/hurenkam/Workspace/ezkvm_v4/tests/
```

---

### AudioDevice (`src/runtime/audio.rs`) [VERIFIED: codebase]

```rust
pub struct AudioDevice {
    device_type: String,  // e.g. "ich9-intel-hda"
    driver: String,       // e.g. "spice" (host audio backend)
}
// Note: codec devices derived from device_type at emit time, not stored
```

**Existing `AudioSchema`** (`src/config/ezkvm/schema/audio.rs`) — DIFFERENT concept:

```rust
// AudioSchema = host audio backend selection (ALSA / PulseAudio / PipeWire)
pub enum AudioSchema {
    Alsa { alsa: AlsaSchema },
    PulseAudio { pulse_audio: PulseAudioSchema },
    PipeWire { pipe_wire: PipeWireSchema },
}
```

`AudioSchema` is in `HostSchema` and describes the host audio system, not the QEMU audio device.
`AudioDevice` is the QEMU audio device (ich9-intel-hda + spice driver). These are separate
concerns. A new `AudioDeviceSchema` is required.

**Recommended approach**: Add new struct in `src/config/ezkvm/schema/audio_device.rs`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct AudioDeviceSchema {
    device_type: String,
    driver: String,
}
```

**Placement**: New optional field `audio_device: Option<AudioDeviceSchema>` in `VirtualMachineSchema` (NOT in HostSchema — AudioDevice is a VM-level QEMU device, not a host system config).

---

### SpiceDisplay (`src/runtime/spice.rs`) [VERIFIED: codebase]

```rust
pub struct SpiceDisplay {
    port: Option<u16>,
    addr: Option<String>,
    disable_ticketing: bool,
    gl: bool,
    rendernode: Option<String>,
    clipboard: bool,
}
```

**Existing `SpiceSchema`** in `src/config/ezkvm/schema/display.rs`:

```rust
pub struct SpiceSchema {
    port: u16,                        // ← required (Runtime has Option<u16>)
    listen: String,                   // ← Runtime calls this "addr"
    disable_ticketing: bool,          // ✓ matches
    #[serde(default)]
    gl_enabled: bool,                 // ← Runtime calls this "gl"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls_port: Option<u16>,            // ← not in Runtime
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls_ciphers: Option<String>,      // ← not in Runtime
    #[serde(default)]
    seamless_migration: bool,         // ← not in Runtime
    // MISSING: rendernode: Option<String>, clipboard: bool
}
```

**Recommended approach**: Update `SpiceSchema` to match the Runtime. To avoid breaking
existing usages, keep the field name `port` as `Option<u16>` and add the missing fields:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct SpiceSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,          // matches Runtime Option<u16>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    addr: Option<String>,       // renamed from "listen" to match Runtime
    disable_ticketing: bool,
    #[serde(default)]
    gl: bool,                   // renamed from "gl_enabled" to match Runtime
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rendernode: Option<String>, // NEW
    #[serde(default)]
    clipboard: bool,            // NEW
    // Removed: tls_port, tls_ciphers, seamless_migration (not in Runtime)
}
```

> ⚠️ Renaming fields breaks any existing YAML files using the old field names (`listen`,
> `gl_enabled`). Check if sample YAML test fixtures exist that would break.

**Placement**: `SpiceDisplay` is a `RootDevice`, so a separate `SpiceDisplaySchema` struct
could also be added to VirtualMachineSchema directly. However, the existing schema places
display config in `HostSchema.display`. This is a design decision for the planner.

---

### RawArgs (`src/runtime/rawargs.rs`) [VERIFIED: codebase]

```rust
pub struct RawArgs(pub String);
// ⚠️ RAWARGS: The inner String is verbatim — never split, tokenize, or reorder it
```

**No existing schema** — needs a new field in `VirtualMachineSchema`:

```rust
// In VirtualMachineSchema:
#[serde(default, skip_serializing_if = "Option::is_none")]
raw_args: Option<String>,
```

The value is a raw string scalar in YAML. A separate `RawArgsSchema` wrapper struct is
unnecessary — `Option<String>` is sufficient and simpler.

> ⚠️ The YAML emitter must preserve internal whitespace and special chars verbatim.
> The bespoke serde_yaml backend handles this correctly for plain String values.

---

## VirtualMachineSchema Extension Plan

Current struct (`src/config/ezkvm/schema/virtual_machine.rs`): [VERIFIED: codebase]

```rust
pub struct VirtualMachineSchema {
    machine: MachineSchema,
    cpu: Option<CpuSchema>,
    memory: MemorySchema,
    boot: BootSchema,
    smbios_uuid: Option<String>,
    vmgenid: Option<String>,
    tpm: Option<TpmSchema>,       // ← existing TPM backend config
    guest_agent: Option<GuestAgent>,
    devices: Vec<DeviceSchema>,
}
```

Required additions for Phase 5:

```rust
pub struct VirtualMachineSchema {
    // ... existing fields ...
    tpm: Option<TpmSchema>,            // KEEP — TPM backend config
    tpmstate: Option<TpmStateSchema>,  // NEW — actual TPM storage volume
    efidisk: Option<EfiDiskSchema>,    // NEW
    audio_device: Option<AudioDeviceSchema>,  // NEW — QEMU audio device
    spice_display: Option<SpiceDisplaySchema>, // NEW — or update via HostSchema
    raw_args: Option<String>,          // NEW — verbatim QEMU args
    guest_agent: Option<GuestAgent>,
    devices: Vec<DeviceSchema>,        // existing — HostPci/Ivshmem go here
}
```

> **Design decision for planner**: SpiceDisplay is a RootDevice (VM-level QEMU option),
> not a host system config. However, the existing `DisplaySchema` lives in `HostSchema`.
> Placing `spice_display` in `VirtualMachineSchema` is a clean model; alternatively,
> update the existing `DisplaySchema::Spice` variant in `HostSchema`. Both are valid.
> **Recommendation**: Keep display in `HostSchema` (it controls how the host connects to
> the VM), but update `SpiceSchema` fields to match the Runtime.

---

## ToStyledYaml Implementation Guide

New types follow the "use `from_serde`" pattern for simple structs. [VERIFIED: codebase]

```rust
// In compact_yaml.rs — add to VirtualMachineSchema::to_styled_yaml():

if let Some(efidisk) = self.efidisk() {
    entries.push((
        "efidisk".to_string(),
        raw_node(from_serde(efidisk), StyleHint::FlowPreferred),
    ));
}

if let Some(tpmstate) = self.tpmstate() {
    entries.push((
        "tpmstate".to_string(),
        raw_node(from_serde(tpmstate), StyleHint::FlowPreferred),
    ));
}

if let Some(audio_device) = self.audio_device() {
    entries.push((
        "audio_device".to_string(),
        raw_node(from_serde(audio_device), StyleHint::FlowPreferred),
    ));
}

if let Some(raw_args) = self.raw_args() {
    entries.push((
        "raw_args".to_string(),
        raw_node(scalar_string(raw_args), StyleHint::FlowPreferred),
    ));
}
```

For new `PcieDeviceTypeSchema` variants (HostPci, Ivshmem), no changes to `compact_yaml.rs`
are needed — they are serialized via the existing `device_to_styled()` → `from_serde()`
path in `VirtualMachineSchema::to_styled_yaml()`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML serialization for new schema types | Custom YAML string builders | `#[derive(Serialize, Deserialize)]` + bespoke serde_yaml | serde derive handles all edge cases; manual builders create inconsistencies |
| YAML serde backend | Any new external crate | Existing `src/serde_yaml/` module | Project explicitly uses bespoke saphyr backend; do NOT add `serde_yaml` crate |
| Tagged enum discrimination | Hand-written match on string keys | `#[serde(tag = "type", rename_all = "snake_case")]` | Existing pattern for PcieDeviceTypeSchema, IdeDeviceTypeSchema |
| Vec serialization | Manual comma-joining | Standard serde Vec handling | `Vec<u8>` serializes as a YAML sequence automatically |

**Key insight:** The bespoke serde_yaml module (`src/serde_yaml/`) is a drop-in; new schema
types just need `#[derive(Serialize, Deserialize)]` and they work automatically.

---

## Common Pitfalls

### Pitfall 1: Adding `serde_yaml` crate as a dependency

**What goes wrong:** The project uses its OWN `serde_yaml` module at `src/serde_yaml.rs` —
adding the `serde_yaml` crate from crates.io creates a name conflict and contradicts the
project's saphyr-based pipeline.

**How to avoid:** Never add `serde_yaml` to Cargo.toml. Use `crate::serde_yaml` (the internal
module) which already provides `from_str`, `to_string`, and `to_value`.

### Pitfall 2: Mismatching `functions: Vec<u8>` serialization

**What goes wrong:** YAML round-trip of `functions: [0, 1]` requires the serde Vec handling
to use YAML sequence syntax `[0, 1]` not a comma-separated string.

**How to avoid:** Use plain `Vec<u8>` in the schema struct. The bespoke serde backend handles
sequences correctly (verified in existing SCSI/SATA device tests). Use `#[serde(default,
skip_serializing_if = "Vec::is_empty")]` to omit empty vecs.

### Pitfall 3: IvshmemPlain removal breaking existing code

**What goes wrong:** `PcieDeviceTypeSchema::IvshmemPlain` is referenced in
`src/config/ezkvm/runtime/builder.rs` (match arm). Removing it without updating the builder
causes a compile error.

**How to avoid:** Grep for `IvshmemPlain` before removing and update all match arms. Since the
builder already returns `Err` for unsupported Pcie device types, adding the new `Ivshmem`
variant requires adding a match arm there too.

### Pitfall 4: ROADMAP field names differ from actual Runtime fields

**What goes wrong:** ROADMAP Phase 5 plans describe `IvshmemSchema { size_mb, name }` but the
actual Runtime `Ivshmem` struct has `{ id: String, mem_path: String, size: String }`. Using
ROADMAP names creates a schema that doesn't match the Runtime.

**How to avoid:** Always use field names from the actual Runtime struct source files, not from
the ROADMAP description. The Runtime source is the authoritative contract.

### Pitfall 5: SpiceSchema field renaming breaking existing YAML round-trips

**What goes wrong:** Renaming `listen` → `addr` and `gl_enabled` → `gl` in SpiceSchema
breaks deserialization of any existing YAML files that use the old field names.

**How to avoid:** Check whether any integration test fixtures or sample YAML files use the old
SpiceSchema field names before renaming. If existing files exist, use `#[serde(rename = "...")]`
or add migration handling.

### Pitfall 6: `VirtualMachineSchema::new()` constructor arity changes

**What goes wrong:** Adding fields to `VirtualMachineSchema` with `#[derive(new)]` changes
the constructor's parameter count. All call sites (in `parser.rs`, `builder.rs`, and tests)
break with a compiler error.

**How to avoid:** When adding new fields to VirtualMachineSchema, update every call to
`VirtualMachineSchema::new(...)` simultaneously (the compiler will flag them all). The
`src/config/ezkvm/runtime/parser.rs` has one call; `runtime/builder.rs` tests have several.

---

## Test Pattern

### Existing round-trip test pattern in `src/config/ezkvm/runtime/parser.rs`: [VERIFIED: codebase]

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn round_trip_q35_pvscsi_scsi_ssd() {
        // 1. Build a Runtime
        let runtime = RuntimeBuilder::new()
            .with_memory(Memory::new(1024))
            .with_chipset(Chipset::Q35(...))
            .build()
            .expect("runtime build failed");

        // 2. Convert Runtime → ConfigSchema
        let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
            .expect("runtime -> schema conversion failed");

        // 3. Convert ConfigSchema → Runtime
        let round_tripped = Runtime::try_from(schema).expect("schema -> runtime conversion failed");

        // 4. Assert field values preserved
        assert_eq!(...);
    }
}
```

### Phase 5 test pattern (schema-level, NOT runtime round-trip)

Phase 5 scope is YAML schema, not runtime wiring. The appropriate test pattern is:

```rust
// In src/config/ezkvm/schema/efidisk.rs (or a test module)
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde_yaml;

    #[test]
    fn efidisk_schema_round_trips_yaml() {
        let schema = EfiDiskSchema::new(
            "vm1-pool:vm-108-efidisk".to_string(),
            Some("4m".to_string()),
            true,
            Some("2023".to_string()),
            "4M".to_string(),
            Some(540672u64),
        );

        // Serialize to YAML
        let yaml_str = serde_yaml::to_string(&schema).expect("serialize failed");

        // Deserialize back
        let recovered: EfiDiskSchema = serde_yaml::from_str(&yaml_str)
            .expect("deserialize failed");

        // Assert field-level equality
        assert_eq!(recovered.storage_volume(), schema.storage_volume());
        assert_eq!(recovered.pre_enrolled_keys(), schema.pre_enrolled_keys());
        assert_eq!(recovered.logical_size(), schema.logical_size());
        assert_eq!(recovered.block_device_size_bytes(), schema.block_device_size_bytes());
    }
}
```

A `ConfigSchema`-level round-trip test via `to_styled_compact_yaml()` + `ConfigSchema::from_str()`:

```rust
#[test]
fn config_with_efidisk_round_trips() {
    let config = ConfigSchema::new(
        Metadata::new("1.0.0".to_string(), "test-vm".to_string()),
        HostSchema::new(None, None, vec![]),
        VirtualMachineSchema::new(
            /* ... machine, cpu, memory, boot, smbios_uuid, vmgenid, tpm, */
            Some(EfiDiskSchema::new("pool:vol".to_string(), None, true, None, "4M".to_string(), None)),
            /* tpmstate, audio_device, spice_display, raw_args, guest_agent, devices */
        ),
    );

    let yaml = config.to_styled_compact_yaml().expect("emit failed");
    let recovered: ConfigSchema = yaml.parse().expect("parse failed");

    assert!(recovered.virtual_machine().efidisk().is_some());
}
```

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` |
| Config file | None (standard Cargo test harness) |
| Quick run command | `cargo test --lib 2>&1 \| head -30` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| YAML-03 | EfiDiskSchema serializes and deserializes losslessly | unit | `cargo test efidisk_schema_round_trips` | In schema/efidisk.rs |
| YAML-03 | TpmStateSchema round-trips YAML | unit | `cargo test tpmstate_schema_round_trips` | In schema/tpmstate.rs |
| YAML-03 | HostPciSchema round-trips YAML including `functions: Vec<u8>` | unit | `cargo test hostpci_schema_round_trips` | Critical: multi-function |
| YAML-03 | IvshmemSchema round-trips YAML | unit | `cargo test ivshmem_schema_round_trips` | Verify id/mem_path/size |
| YAML-03 | AudioDeviceSchema round-trips YAML | unit | `cargo test audio_device_schema_round_trips` | In schema/audio_device.rs |
| YAML-03 | SpiceDisplaySchema round-trips YAML with all optional fields | unit | `cargo test spice_display_schema_round_trips` | Check rendernode/clipboard |
| YAML-03 | raw_args Option<String> survives styled YAML round-trip verbatim | unit | `cargo test raw_args_round_trips` | Verify no normalization |
| YAML-03 | ConfigSchema with all seven new fields compiles and emits valid YAML | integration | `cargo test config_with_all_seven_types` | In parser.rs tests |

---

## Security Domain

This phase adds no authentication, session management, network access, or cryptography. All
changes are in-process data structure definitions and YAML serialization. ASVS categories
V2–V6 do not apply. The only relevant concern is:

- **V5 Input Validation**: `raw_args` field is stored verbatim; no validation is applied.
  This is intentional by design (the RAWARGS invariant). Phase 8 (VM Lifecycle) bears
  responsibility for safe process-level handling of the raw args string.

---

## Code Examples

### Minimal new schema type (from `src/config/ezkvm/schema/memory.rs`): [VERIFIED: codebase]

```rust
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, new)]
pub struct MemorySchema {
    size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hugepages_kb: Option<usize>,
    #[serde(default)]
    numa_enabled: bool,
}
```

### Enum with tag discriminant (from `src/config/ezkvm/schema/ide.rs`): [VERIFIED: codebase]

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdeDeviceTypeSchema {
    Hdd { resource: String },
    Ssd { resource: String },
    Cdrom { resource: String },
}
// Serializes as: {type: hdd, resource: "..."}
```

### Untagged enum (from `src/config/ezkvm/schema/tpm.rs`): [VERIFIED: codebase]

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TpmSchema {
    Emulated { swtpm: SwtpmSchema },
    Passthrough { hwtpm: HwtpmSchema },
}
// Serializes as: {swtpm: {version: 2.0, resource: "..."}}
```

### `ToStyledYaml` for a simple struct (from `compact_yaml.rs`): [VERIFIED: codebase]

```rust
// Simple types: use from_serde() which delegates to serde serialization
entries.push((
    "memory".to_string(),
    raw_node(from_serde(self.memory()), StyleHint::FlowPreferred),
));
```

### Optional field in `to_styled_yaml` (from `compact_yaml.rs`): [VERIFIED: codebase]

```rust
if let Some(cpu) = self.cpu() {
    entries.push((
        "cpu".to_string(),
        raw_node(from_serde(cpu), StyleHint::FlowPreferred),
    ));
}
```

---

## Open Questions (RESOLVED)

1. **SpiceDisplay placement**: RESOLVED: Update `SpiceSchema` in-place within `HostSchema.display` (D-01). SpiceDisplay is a display protocol, not a GPU. The missing fields `gl`, `rendernode`, `clipboard` are appended to the existing `SpiceSchema`.

2. **`IvshmemPlain` removal**: RESOLVED: Keep `IvshmemPlain { resource: String }` untouched (D-03). A new `ResourceSchema::Memory` variant is added for size+path. No YAML fixtures use `IvshmemPlain` (confirmed via grep).

3. **`TpmSchema` vs `TpmStateSchema` coexistence**: RESOLVED: Update `SwtpmSchema.version: f32 → String` in-place (D-02). No new TpmStateSchema is introduced. `SwtpmSchema.resource` is an ID ref into ResourceSchema (intentional alias for Runtime's `storage_volume` — Phase 7 maps accordingly).

4. **`VirtualMachineSchema::new()` parameter explosion**: RESOLVED: Accept the 12-parameter constructor (consistent with existing style). All 5 call sites are identified in PLAN.md and updated with `None` for the three new optional fields.

---

## Environment Availability

All dependencies are in Cargo.toml. No external tools, services, or CLIs are required.
`cargo test` and `cargo build` are the only commands needed.

| Dependency | Required By | Available | Version |
|------------|------------|-----------|---------|
| Rust toolchain | Build & test | ✓ | Verified by existing code |
| `serde` | Schema derive | ✓ | 1.0.228 (Cargo.toml) |
| `saphyr` | YAML backend | ✓ | 0.0.11 (Cargo.toml) |
| `derive-getters` | Getters derive | ✓ | 0.5.0 (Cargo.toml) |
| `derive-new` | new() derive | ✓ | 0.7.0 (Cargo.toml) |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SpiceSchema should be updated in-place (not duplicated as SpiceDisplaySchema) | SpiceDisplay section | Design inconsistency; both approaches compile |
| A2 | `IvshmemPlain` has no external YAML fixture usages and can be removed | Ivshmem section | Existing YAML files break on deserialization |
| A3 | `TpmStateSchema` should be a new separate type alongside `TpmSchema` | TpmState section | Planner may prefer updating SwtpmSchema instead |
| A4 | `AudioDeviceSchema` belongs in `VirtualMachineSchema`, not `HostSchema` | AudioDevice section | Model may expect audio device in host config |

---

## Sources

### Primary (HIGH confidence)
- `src/config/ezkvm/schema/` — all schema type definitions read directly [VERIFIED: codebase]
- `src/config/ezkvm/file/compact_yaml.rs` — `ToStyledYaml` pattern, `from_serde`, `StyleHint` [VERIFIED: codebase]
- `src/runtime/efidisk.rs`, `tpmstate.rs`, `audio.rs`, `spice.rs`, `rawargs.rs` — Runtime field inventory [VERIFIED: codebase]
- `src/runtime/devices/hostpci.rs`, `ivshmem.rs` — PCIe bus device field inventory [VERIFIED: codebase]
- `src/config/ezkvm/runtime/parser.rs`, `builder.rs` — TryFrom impls and test patterns [VERIFIED: codebase]
- `Cargo.toml` — dependency versions [VERIFIED: codebase]
- `.planning/REQUIREMENTS.md` — YAML-03 requirement definition [VERIFIED: codebase]
- `.planning/ROADMAP.md` Phase 5 section — planned task descriptions [VERIFIED: codebase]

---

## Metadata

**Confidence breakdown:**
- Schema type design: HIGH — all existing types and Runtime types read directly
- Placement decisions: MEDIUM — some design choices (SpiceDisplay, TpmState coexistence) involve architectural judgment
- ToStyledYaml pattern: HIGH — verified against existing compact_yaml.rs implementation
- Test pattern: HIGH — existing tests in parser.rs and runtime_phase2.rs confirmed

**Research date:** 2025-07-22
**Valid until:** Stable (Rust/serde/saphyr APIs; no external services)
