---
phase: 05-yaml-schema
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/config/ezkvm/schema/boot.rs
  - src/config/ezkvm/schema/audio_device.rs
  - src/config/ezkvm/schema/rawargs.rs
  - src/config/ezkvm/schema.rs
  - src/config/ezkvm/schema/pcie.rs
  - src/config/ezkvm/schema/pci.rs
  - src/config/ezkvm/schema/resources.rs
  - src/config/ezkvm/schema/display.rs
  - src/config/ezkvm/schema/tpm.rs
  - src/config/ezkvm/schema/virtual_machine.rs
  - src/config/ezkvm/file/compact_yaml.rs
  - src/config/ezkvm.rs
  - src/config/ezkvm/runtime/parser.rs
  - src/config/ezkvm/runtime/builder.rs
autonomous: true
requirements:
  - YAML-03

must_haves:
  truths:
    - UefiSchema gains efitype/pre_enrolled_keys/ms_cert/logical_size fields; round-trips all EFI disk properties alongside existing resource field (D-04)
    - PcieDeviceTypeSchema::HostPci serializes as type: host_pci and round-trips Vec<u8> functions through YAML sequence syntax (D-07)
    - ResourceSchema gains a Memory variant that round-trips id/path/size losslessly (D-03)
    - SpiceSchema gains gl/rendernode/clipboard fields; existing YAML using other fields continues to parse (D-01)
    - SwtpmSchema.version type is String not f32; "v2.0" round-trips verbatim (D-02)
    - AudioDeviceSchema and RawArgsSchema each round-trip their fields losslessly (D-05, D-06)
    - VirtualMachineSchema gains audio_device/raw_args optional fields; all five call sites pass the two new None arguments (D-05, D-06)
    - cargo test passes with zero regressions on all pre-existing tests
  artifacts:
    - src/config/ezkvm/schema/boot.rs (updated — UefiSchema extended with EFI disk fields)
    - src/config/ezkvm/schema/audio_device.rs (new)
    - src/config/ezkvm/schema/rawargs.rs (new)
    - src/config/ezkvm/schema.rs (updated — new mod + pub use entries)
    - src/config/ezkvm/schema/virtual_machine.rs (updated — two new optional fields)
    - src/config/ezkvm/file/compact_yaml.rs (updated — emit new optional fields)
  key_links:
    - VirtualMachineSchema::new() arity changes from 9 to 11 params — the compiler flags every missed call site; all five must be updated in Task 3
    - crate::serde_yaml (internal bespoke module at src/serde_yaml/) — use in all tests; do NOT add serde_yaml to Cargo.toml
    - PcieDeviceTypeSchema uses #[serde(tag = "type", rename_all = "snake_case")] — the new HostPci variant serializes as type: host_pci automatically
    - IvshmemPlain stays untouched per D-03; the Memory variant goes into ResourceSchema, not into PcieDeviceTypeSchema
---

<objective>
Extend the ezkvm YAML schema to cover all seven v1 Runtime device types so each can be
fully serialized and deserialized through the existing serde pipeline.

Purpose: Phase 7 (QEMU argument emission) needs schema representations with correct field
names and types before it can map ConfigSchema → QEMU CLI arguments. Without this phase,
five device types have no schema at all and two have field-level mismatches that would
produce wrong QEMU output.

Output: Two new schema files (audio_device.rs, rawargs.rs), five modified
schema files (boot.rs, pcie.rs, resources.rs, display.rs, tpm.rs), updated VirtualMachineSchema,
updated compact_yaml.rs, updated call sites, and per-type round-trip unit tests. No new
Cargo dependencies.
</objective>

<execution_context>
@.github/gsd-core/workflows/execute-plan.md
@.github/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md

# Schema module registration file
@src/config/ezkvm/schema.rs

# Schema types being extended
@src/config/ezkvm/schema/boot.rs
@src/config/ezkvm/schema/pcie.rs
@src/config/ezkvm/schema/resources.rs
@src/config/ezkvm/schema/display.rs
@src/config/ezkvm/schema/tpm.rs
@src/config/ezkvm/schema/virtual_machine.rs

# Styled YAML emission layer
@src/config/ezkvm/file/compact_yaml.rs

# All VirtualMachineSchema::new() call sites
@src/config/ezkvm.rs
@src/config/ezkvm/runtime/parser.rs
@src/config/ezkvm/runtime/builder.rs

# Reference: existing schema type that follows the exact pattern to use
@src/config/ezkvm/schema/memory.rs
</context>

<tasks>

<!-- ═══════════════════════════════════════════════════════════════
     TASK 1 (TRACER) — Leaf schema modifications
     These touch only existing files; they have no inbound dependencies
     and unblock Tasks 2 and 3.
     ═══════════════════════════════════════════════════════════════ -->

<task type="auto">
  <name>Task 1: Leaf schema modifications — HostPci variant, Memory resource, UefiSchema EFI fields, SpiceSchema fields, SwtpmSchema version type</name>
  <files>
    src/config/ezkvm/schema/boot.rs,
    src/config/ezkvm/schema/pcie.rs,
    src/config/ezkvm/schema/pci.rs,
    src/config/ezkvm/schema/resources.rs,
    src/config/ezkvm/schema/display.rs,
    src/config/ezkvm/schema/tpm.rs,
    src/main.rs,
    wakiza.yaml
  </files>
  <action>
Make four targeted modifications to existing schema files. No new files are created here.

**1a. resources.rs — extend PcieDeviceResourceSchema::HostAddress with functions (D-07)**

`HostAddress` already has `address: String`, `rombar: Option<bool>`, `romfile: Option<String>`.
These are host-specific properties. Add `functions: Vec<u8>` to represent multi-function
PCIe passthrough (runtime `HostPci.functions`). The `multifunction: Option<bool>` field
is kept for backward compat; `functions` replaces it as the authoritative source.

```rust
HostAddress {
    address: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    functions: Vec<u8>,    // len() > 1 implies multifunction; no separate field needed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rombar: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    romfile: Option<String>,
},
```

Add test `hostpci_resource_round_trips_yaml` in resources.rs:
- Construct `ResourceSchema::PcieDevice { id: "hostpci0".to_string(), pcie: PcieDeviceResourceSchema::HostAddress { address: "0000:03:00".to_string(), functions: vec![0u8, 1u8], rombar: None, romfile: None } }`
- Serialize and deserialize; assert `functions` round-trips as `vec![0u8, 1u8]`.

**1b. pcie.rs — add HostPci variant to PcieDeviceTypeSchema (D-07)**

The enum already has `#[serde(tag = "type", rename_all = "snake_case")]`. Append a new
`HostPci` variant with VM-specific properties only. Host-specific data (address, functions,
rombar, romfile) lives in `ResourceSchema::PcieDevice` (see 1a above). `pcie=true` is
implied by placement in `PcieDeviceTypeSchema` — no explicit field needed:

```rust
HostPci {
    resource: String,   // ID ref into HostSchema.resources (PcieDevice/HostAddress entry)
    #[serde(default)]
    x_vga: bool,
},
```

**1c. pci.rs — add HostPci variant to PciDeviceType**

Same device, non-PCIe bus placement (pcie=false implied). Add to the existing `PciDeviceType` enum:

```rust
HostPci {
    resource: String,   // ID ref into HostSchema.resources (PcieDevice/HostAddress entry)
    #[serde(default)]
    x_vga: bool,
},
```

`functions: Vec<u8>` serializes as a YAML sequence (e.g. `[0, 1]`) via the bespoke
serde_yaml backend — do NOT attempt manual comma-joining.

Add a `#[cfg(test)]` block at the bottom of pcie.rs with test `hostpci_round_trips_yaml`:
- Construct `PcieDeviceTypeSchema::HostPci { resource: "hostpci0".to_string(), x_vga: false }`
- Wrap it in `PcieDeviceSchema::new(None, None, device_type)` to reach the outermost
  serializable type (ensures flatten attributes don't break anything)
- Serialize with `crate::serde_yaml::to_string(&schema)` and deserialize with
  `crate::serde_yaml::from_str::<PcieDeviceSchema>(&yaml_str)`
- Assert `type: host_pci` appears in the YAML string
- Assert `functions` round-trips as `vec![0u8, 1u8]`

**2. resources.rs — add Memory variant to ResourceSchema (D-03)**

The enum uses `#[serde(untagged)]`. Append a new variant that identifies itself by the
presence of a `memory` field:

```
Memory {
    id: String,
    memory: MemoryResourceSchema,
},
```

Add a new companion struct in the same file:

```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryResourceSchema {
    path: String,
    size: String,
}
```

Untagged discrimination works because no other variant has a `memory` field.

Add test `memory_resource_round_trips_yaml` in a `#[cfg(test)]` block:
- Construct `ResourceSchema::Memory { id: "shm0".to_string(), memory: MemoryResourceSchema { path: "/dev/kvmfr0".to_string(), size: "128M".to_string() } }`
- Serialize and deserialize; assert id, path, and size equal the originals.

**3. display.rs — add three fields to SpiceSchema (D-01)**

SpiceSchema currently has: port, listen, disable_ticketing, gl_enabled, tls_port,
tls_ciphers, seamless_migration.

Append three new fields at the end of the struct definition:

```
#[serde(default)]
gl: bool,
#[serde(default, skip_serializing_if = "Option::is_none")]
rendernode: Option<String>,
#[serde(default)]
clipboard: bool,
```

The derive macros `#[derive(Getters, new)]` are already on SpiceSchema — adding three
fields increases the `new()` constructor arity. Check whether any existing call sites
use `SpiceSchema::new(...)` directly; if found, add the three new arguments (`false`,
`None`, `false`). If no call sites exist, the `new()` arity change is safe without
further action.

Add test `spice_schema_round_trips_yaml` in the existing or a new `#[cfg(test)]` block:
- Construct a SpiceSchema with `gl: true`, `rendernode: Some("/dev/dri/renderD128".to_string())`, `clipboard: false`
- Serialize and deserialize; assert the three new fields round-trip correctly.

**4. tpm.rs — change SwtpmSchema.version from f32 to String (D-02)**

Change the field declaration from `version: f32` to `version: String`.

`SwtpmSchema::new()` constructor parameter order is preserved; callers that passed a
float literal must now pass a `String`. Run `grep -rn "SwtpmSchema::new" src/` — there
are NO call sites currently; the only construction is `SwtpmSchema::default()` in tpm.rs
itself (the handwritten `impl Default for TpmSchema`). `cargo build` will surface any
missed call sites via type mismatch errors automatically.

Also update `src/main.rs` and any YAML fixtures (e.g. `wakiza.yaml`) that contain
`version: 2.0` (YAML float) to `version: "v2.0"` (quoted string), because the bespoke
`serde_yaml` deserializer dispatches float scalars to `visit_f64`, not `visit_str`, and
would panic at runtime after this type change.

**Field name note for Phase 7**: `SwtpmSchema.resource` intentionally aliases the Runtime
field `TpmState.storage_volume`. The schema uses the generic name `resource` (an ID ref
into `HostSchema.resources`) while the Runtime uses `storage_volume`. Phase 7 authors
must map `TpmState.storage_volume → SwtpmSchema.resource` during conversion.

Add test `swtpm_schema_round_trips_yaml`:
- Construct `SwtpmSchema::new("v2.0".to_string(), "vm1-pool:vm-108-tpmstate".to_string())`
- Serialize and deserialize; assert version is the string `"v2.0"` not a float.

**5. boot.rs — extend UefiSchema with EFI disk fields (D-04)**

`UefiSchema` already has `resource: String` (the storage volume ID ref). Add the remaining
EFI disk properties alongside it:

```rust
#[derive(Debug, Clone, Default, Deserialize, Serialize, Getters, new)]
pub struct UefiSchema {
    resource: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    efitype: Option<String>,
    #[serde(default)]
    pre_enrolled_keys: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ms_cert: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    logical_size: Option<String>,
}
```

`logical_size` is `Option<String>` (not required) so that existing YAML with only `resource`
continues to deserialize without error.

No `block_device_size_bytes` field (runtime-only; not part of the schema design).

Check whether any existing `UefiSchema::new(...)` call sites exist (`grep -rn "UefiSchema::new" src/`).
If found, add the four new arguments (`None, false, None, None`). If the `Default` impl is
handwritten, update it to set `efitype: None, pre_enrolled_keys: false, ms_cert: None, logical_size: None`.

Add test `uefi_schema_round_trips_yaml` in a `#[cfg(test)]` block:
- Full case: `UefiSchema::new("pool:vol".to_string(), Some("4m".to_string()), true, Some("2023".to_string()), Some("4M".to_string()))`
- Minimal case: `UefiSchema::new("pool:vol".to_string(), None, false, None, None)`
- For minimal case, assert the YAML does NOT contain `efitype`, `ms_cert`, or `logical_size`
- Deserialize both and assert all fields equal the originals.
  </action>
  <verify>
    <automated>cargo test hostpci_round_trips_yaml memory_resource_round_trips_yaml spice_schema_round_trips_yaml swtpm_schema_round_trips_yaml uefi_schema_round_trips_yaml 2>&1 | tail -20</automated>
  </verify>
  <done>
    All five tests pass. `cargo test --lib` shows no pre-existing test regressions.
    `PcieDeviceTypeSchema::HostPci` exists in pcie.rs; `ResourceSchema::Memory` exists in
    resources.rs; SpiceSchema has gl/rendernode/clipboard; SwtpmSchema.version is `String`;
    UefiSchema has efitype/pre_enrolled_keys/ms_cert/logical_size alongside resource.
  </done>
</task>


<!-- ═══════════════════════════════════════════════════════════════
     TASK 2 — New schema files: AudioDevice, RawArgs
     Independent of Task 1; can run in parallel but must complete
     before Task 3.
     ═══════════════════════════════════════════════════════════════ -->

<task type="auto" tdd="true">
  <name>Task 2: New schema files — AudioDeviceSchema, RawArgsSchema + module registration</name>
  <files>
    src/config/ezkvm/schema/audio_device.rs,
    src/config/ezkvm/schema/rawargs.rs,
    src/config/ezkvm/schema.rs
  </files>
  <behavior>
    - AudioDeviceSchema { device_type: "ich9-intel-hda", driver: "spice" } round-trips both fields
    - RawArgsSchema wrapping a string with internal spaces and hyphens round-trips verbatim
  </behavior>
  <action>
**1. src/config/ezkvm/schema/audio_device.rs (new file, D-05)**

```rust
use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct AudioDeviceSchema {
    device_type: String,
    driver: String,
}
```

Add `#[cfg(test)]` block with `audio_device_schema_round_trips_yaml`:
- Construct `AudioDeviceSchema::new("ich9-intel-hda".to_string(), "spice".to_string())`
- Serialize; assert the YAML contains `device_type` and `driver` keys
- Deserialize and assert both fields equal the originals.

**2. src/config/ezkvm/schema/rawargs.rs (new file, D-06)**

```rust
use derive_getters::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Getters)]
pub struct RawArgsSchema(pub String);
```

Note: `RawArgsSchema` is a newtype tuple struct. `Getters` on a tuple struct generates
`fn 0(&self) -> &String`. If `derive-getters` 0.5.0 does not support tuple struct
field 0, use a named wrapper instead: `pub struct RawArgsSchema { pub value: String }`.
Verify at compile time and choose the form that compiles. `#[derive(new)]` is not needed;
construction is `RawArgsSchema("...".to_string())` or `RawArgsSchema { value: "...".to_string() }`.

Add `#[cfg(test)]` block with `raw_args_schema_round_trips_verbatim`:
- Construct with a string containing embedded spaces, hyphens, and equals signs:
  `"-device virtio-serial-pci -chardev spicevmc,id=vdagent"`
- Serialize with `crate::serde_yaml::to_string` and deserialize back
- Assert the deserialized inner string equals the original byte-for-byte (no whitespace normalization).

**3. src/config/ezkvm/schema.rs (module registration)**

In the `mod` block, add (maintain alphabetical order):
```
mod audio_device;
mod rawargs;
```

In the `pub use` glob block, add:
```
audio_device::*, rawargs::*,
```
  </action>
  <verify>
    <automated>cargo test audio_device_schema_round_trips_yaml raw_args_schema_round_trips_verbatim 2>&1 | tail -20</automated>
  </verify>
  <done>
    Two new schema files compile and their unit tests pass. All exported types are visible
    via `use ezkvm::config::ezkvm::schema::*`. `cargo build` succeeds without errors.
  </done>
</task>


<!-- ═══════════════════════════════════════════════════════════════
     TASK 3 (Wave 2) — Wire into VirtualMachineSchema + call sites + compact_yaml
     Depends on Tasks 1 and 2 being complete.
     ═══════════════════════════════════════════════════════════════ -->

<task type="auto">
  <name>Task 3: Wire new fields into VirtualMachineSchema, update all call sites, extend compact_yaml emission</name>
  <files>
    src/config/ezkvm/schema/virtual_machine.rs,
    src/config/ezkvm/file/compact_yaml.rs,
    src/config/ezkvm.rs,
    src/config/ezkvm/runtime/parser.rs,
    src/config/ezkvm/runtime/builder.rs
  </files>
  <action>
**1. virtual_machine.rs — add two new optional fields (D-05, D-06)**

Add imports at the top of the file (alongside existing imports):
```rust
use crate::config::ezkvm::schema::audio_device::AudioDeviceSchema;
use crate::config::ezkvm::schema::rawargs::RawArgsSchema;
```

Insert two new fields after the existing `tpm` field and before `guest_agent`:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
audio_device: Option<AudioDeviceSchema>,
#[serde(default, skip_serializing_if = "Option::is_none")]
raw_args: Option<RawArgsSchema>,
```

After this edit, `VirtualMachineSchema::new()` takes 11 parameters (was 9). The field
declaration order determines constructor parameter order:
1. machine  2. cpu  3. memory  4. boot  5. smbios_uuid  6. vmgenid  7. tpm
8. audio_device ← NEW  9. raw_args ← NEW
10. guest_agent  11. devices

**2. Verify VirtualMachineSchema construction**

Do NOT proceed to edit call sites until `cargo build` confirms the compiler has flagged
each of the five call sites. Fix them one by one:

a) `src/config/ezkvm.rs` (~line 72, inside `SchemaBuilder::build`):
   After the `let tpm = None;` line, add:
   ```rust
   let audio_device = None;
   let raw_args = None;
   ```
   Pass both in the `VirtualMachineSchema::new(...)` call between `tpm` and
   `guest_agent`.

b) `src/config/ezkvm/runtime/parser.rs` (~line 55, `RuntimeToSchemaConverter::convert`):
   Add `None, None,` between the existing `None` (tpm) and `None` (guest_agent)
   in the `VirtualMachineSchema::new(...)` call.

c-e) `src/config/ezkvm/runtime/builder.rs` (three test functions at ~lines 371, 400, 435):
   Each call currently passes `None, None, None, None` for params 5-8 (smbios_uuid,
   vmgenid, tpm, guest_agent). The guest_agent `None` is now at position 10. Insert
   two additional `None,` values between tpm (param 7) and guest_agent (param 10):
   the seven trailing arguments become:
   `None, None, None, None, None, None, devices` — that is:
   smbios_uuid=None, vmgenid=None, tpm=None, audio_device=None,
   raw_args=None, guest_agent=None, devices.

**3. compact_yaml.rs — emit new optional fields in VirtualMachineSchema::to_styled_yaml()**

In the `impl ToStyledYaml for VirtualMachineSchema` block, after the existing `tpm`
emission block (`if let Some(tpm) = self.tpm() { ... }`), insert two new emission
blocks before the `guest_agent` block:

```rust
if let Some(audio_device) = self.audio_device() {
    entries.push((
        "audio_device".to_string(),
        raw_node(from_serde(audio_device), StyleHint::FlowPreferred),
    ));
}

if let Some(raw_args) = self.raw_args() {
    entries.push((
        "raw_args".to_string(),
        raw_node(from_serde(raw_args), StyleHint::FlowPreferred),
    ));
}
```

Also update the import list at the top of compact_yaml.rs to include the two new types:
`AudioDeviceSchema, RawArgsSchema` alongside the existing imports.

**4. Integration test**

Add a test `virtual_machine_schema_with_new_fields_round_trips_yaml` in
`src/config/ezkvm/runtime/builder.rs` (or parser.rs) `#[cfg(test)]` block:
- Construct a `VirtualMachineSchema` with both new fields set to `Some(...)` values:
  audio_device with device_type="ich9-intel-hda" driver="spice",
  raw_args with inner string="-vga none".
- Wrap in a full `ConfigSchema::new(...)`.
- Emit via `config.to_styled_compact_yaml()`.
- Parse back via `ConfigSchema::from_str(...)` (or however the existing tests parse back).
- Assert `virtual_machine.audio_device().is_some()`,
  `virtual_machine.raw_args().is_some()`.
  </action>
  <verify>
    <automated>cargo test 2>&1 | tail -30</automated>
  </verify>
  <done>
    `cargo test` exits 0 with no failures. VirtualMachineSchema has audio_device and
    raw_args fields. compact_yaml.rs emits both when present. All five
    VirtualMachineSchema::new() call sites compile with the new 11-parameter signature.
    Integration test confirms ConfigSchema round-trips the two new optional fields.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| YAML file → schema structs | Untrusted YAML input deserialized via serde; field types validated by serde |
| raw_args String → process invocation | raw_args value is stored verbatim in schema; Phase 8 bears responsibility for safe shell handling at process invocation time |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-05-01 | Tampering | RawArgsSchema inner String | low | accept | raw_args is stored verbatim by design (RAWARGS invariant); schema phase has no CLI; safe handling is Phase 8's responsibility |
| T-05-02 | Information Disclosure | SwtpmSchema.resource field | low | accept | Resource IDs are non-secret config references; schema file is user-controlled YAML |
| T-05-03 | Denial of Service | Deeply-nested or very large YAML input | low | accept | saphyr parser has no recursion depth limit but this is an operator-controlled config file, not externally-supplied input |
</threat_model>

<verification>
Run in order after all three tasks complete:

```bash
# Unit tests for new types
cargo test hostpci_round_trips_yaml
cargo test memory_resource_round_trips_yaml
cargo test spice_schema_round_trips_yaml
cargo test swtpm_schema_round_trips_yaml
cargo test uefi_schema_round_trips_yaml
cargo test audio_device_schema_round_trips_yaml
cargo test raw_args_schema_round_trips_verbatim
cargo test virtual_machine_schema_with_new_fields_round_trips_yaml

# Full suite — must exit 0
cargo test 2>&1 | tail -20

# Confirm new types are exported
grep -c "AudioDeviceSchema\|RawArgsSchema" src/config/ezkvm/schema.rs

# Confirm HostPci variant exists
grep -c "HostPci" src/config/ezkvm/schema/pcie.rs

# Confirm Memory variant exists
grep -c "MemoryResourceSchema" src/config/ezkvm/schema/resources.rs

# Confirm UefiSchema has EFI disk fields
grep -c "efitype\|pre_enrolled_keys\|ms_cert\|logical_size" src/config/ezkvm/schema/boot.rs

# Confirm VirtualMachineSchema has two new fields
grep -c "audio_device\|raw_args" src/config/ezkvm/schema/virtual_machine.rs
```
</verification>

<success_criteria>
1. `cargo test` exits 0 — all eight new round-trip tests pass, zero pre-existing test regressions
2. `PcieDeviceTypeSchema::HostPci` variant exists with resource, x_vga
3. `ResourceSchema::Memory` variant exists with id, path, size
4. `SpiceSchema` has gl, rendernode, clipboard fields
5. `SwtpmSchema.version` is `String`
6. `UefiSchema` has efitype, pre_enrolled_keys, ms_cert, logical_size fields alongside resource
7. `AudioDeviceSchema`, `RawArgsSchema` each exist in their own files and are pub-exported from `schema.rs`
8. `VirtualMachineSchema` has `audio_device: Option<AudioDeviceSchema>`, `raw_args: Option<RawArgsSchema>`
9. All five `VirtualMachineSchema::new(...)` call sites compile with the 11-parameter signature
10. `compact_yaml.rs` emits the two new optional fields when present
11. No `serde_yaml` entry added to Cargo.toml (internal crate::serde_yaml module used throughout)
</success_criteria>

## Source Audit

| Source | Item | Status | Plan Task |
|--------|------|--------|-----------|
| GOAL | EfiDisk schema coverage | COVERED | Task 1 (D-04, UefiSchema extended) |
| GOAL | TpmState schema coverage | COVERED | Task 1 (D-02, SwtpmSchema.version fix) |
| GOAL | HostPci schema coverage | COVERED | Task 1 (D-07, new PcieDeviceTypeSchema::HostPci) |
| GOAL | Ivshmem schema coverage | COVERED | Task 1 (D-03, Memory ResourceSchema + keep IvshmemPlain) |
| GOAL | AudioDevice schema coverage | COVERED | Task 2+3 (D-05) |
| GOAL | SpiceDisplay schema coverage | COVERED | Task 1 (D-01, SpiceSchema field additions) |
| GOAL | RawArgs schema coverage | COVERED | Task 2+3 (D-06) |
| REQ | YAML-03 — YAML schema covers all v1 Runtime device types | COVERED | All tasks |
| RESEARCH | HostPci functions: Vec\<u8\> sequence serialization | COVERED | Task 1 — Vec\<u8\> via serde; test verifies |
| RESEARCH | IvshmemPlain removal pitfall | N/A — deferred | D-03 explicitly keeps IvshmemPlain |
| RESEARCH | VirtualMachineSchema::new() arity breakage pitfall | COVERED | Task 3 — all 5 call sites updated |
| RESEARCH | crate::serde_yaml (not external crate) pitfall | COVERED | All task actions explicitly reference crate::serde_yaml |
| RESEARCH | SpiceSchema field rename breaking existing YAML | COVERED | Task 1 adds fields; existing fields kept |
| CONTEXT | D-01 SpiceSchema: add gl/rendernode/clipboard | COVERED | Task 1 |
| CONTEXT | D-02 SwtpmSchema.version: f32 → String | COVERED | Task 1 |
| CONTEXT | D-03 IvshmemPlain kept; Memory added to ResourceSchema | COVERED | Task 1 |
| CONTEXT | D-04 UefiSchema extended with efitype/pre_enrolled_keys/ms_cert/logical_size | COVERED | Task 1 |
| CONTEXT | D-05 AudioDeviceSchema device_type/driver in VirtualMachineSchema | COVERED | Task 2+3 |
| CONTEXT | D-06 RawArgsSchema(pub String) in rawargs.rs, raw_args in VirtualMachineSchema | COVERED | Task 2+3 |
| CONTEXT | D-07 PcieDeviceTypeSchema::HostPci with full field set | COVERED | Task 1 |

<output>
Create `.planning/phases/05-yaml-schema/05-01-SUMMARY.md` when all three tasks complete.
</output>
