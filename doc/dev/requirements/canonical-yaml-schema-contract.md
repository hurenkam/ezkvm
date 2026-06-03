# Canonical YAML Schema Contract

Status: Draft  
Date: 2026-05-29

## Purpose

Define the canonical YAML structure that represents source-agnostic virtual machine intent in a human-readable, deterministic form.

## Requirement Traceability

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- NFR-001 Usability
- RDR-002 Typed Internal Contracts
- RDR-006 Profile/Composition Friendly Config

## Schema Contract

1. The top-level document must be YAML mapping data.
2. The canonical document must include these top-level sections:
   - `metadata`
   - `virtual_machine`
3. `virtual_machine` must be source-agnostic and sufficient to describe guest intent.
4. Unknown top-level keys must be rejected unless explicitly marked as extension namespace.

## Required Core Fields

- `metadata.schema_version`: semantic schema version string.
- `metadata.vm_name`: stable VM identifier (must be the same as the basename of the config file).
- `virtual_machine.system.machine.family`: canonical machine family.
- `virtual_machine.system.machine.chipset`: canonical machine chipset identifier.
- `virtual_machine.system.cpu.model`: canonical CPU model intent.
- `virtual_machine.system.memory.min`: integer MiB memory value.

## Normative Capability Areas

### 1) Machine identity

Canonical machine identity must be represented under `virtual_machine.system.machine`.

- `family`: required string identifying the machine family (`pc` for x86 PC machine families).
- `chipset`: required string identifying the machine chipset (`q35`, `i440fx`, or future canonical chipsets).
- `version`: optional string for explicit machine version pinning.
- `options`: optional mapping of machine options that are still guest-intent level.

`virtual_machine.system.machine.chipset` remains required and must be consistent with `virtual_machine.system.machine.family`.

### 2) Deterministic topology placement identity

Chipset-sensitive devices must carry a deterministic placement identity so rendering is stable.

- Placement identity must include deterministic coordinates (for example bus/slot/function, port, or controller/unit depending on bus type).
- Placement identity must be modeled as guest intent and must not depend on host runtime probing.
- Two devices in the same topology scope must not resolve to the same coordinates.

### 3) Boot, storage, and network intent shape

The canonical model must encode intent for boot ordering, storage role, and network role.

- Boot intent must be explicit using an ordered list of boot targets.
- Storage entries must include stable `id` values and role/type intent (for example system disk, data disk, cdrom).
- Network entries must include stable `id` values and guest-visible interface intent (driver/model/mac/role).
- Host attachment details for storage and network backends are not canonical guest intent and must be resolved from host/runtime config.

### 4) Firmware and TPM identity

Firmware and TPM identity must be explicit and stable.

- Firmware identity must include implementation type (for example `ovmf`) and security mode (`secure_boot`).
- TPM identity must include implementation type and interface model (for example `tpm-tis` or `tpm-crb`).
- Host-local state paths (for firmware vars or swtpm state) must not be literal canonical guest intent fields.

### 5) Passthrough intent boundary

Passthrough intent must remain generic in canonical VM config, while host-specific binding lives in host config.

- Canonical VM config uses resource references by ID only.
- Host config owns the resource catalog and host literal binding material.
- Runtime resolution binds VM resource IDs to host catalog entries.
- Literal host coordinates (for example PCI BDF, USB bus/port, IOMMU group paths) are not canonical guest intent.

## Host resource catalog and VM reference contract

This contract formalizes the generic pass-through concept across host and VM configs.

### Host config side

Host config defines a resource catalog:

- `resources[].id`: required globally unique identifier per host config.
- `resources[].type`: required canonical resource class (for example `pcie`, `usb`).
- `resources[].host`: required host-literal binding payload, interpreted only by host/runtime resolution.

### VM config side

VM config references host catalog resources:

- `virtual_machine.resources[]`: list of resource reference objects.
- `virtual_machine.resources[].id`: required and must match a host catalog `resources[].id` value at runtime.
- VM resource references must not include host-literal keys such as `host`, BDF values, or bus/port literals.

## Validation Rules

1. Required fields must be present and type-correct.
2. Scalar normalization must be deterministic (for example booleans and integers).
3. List fields that influence rendering order must define stable ordering semantics.
4. Validation errors must include field path and reason.
5. `metadata.vm_name` must match the config filename stem.
6. `virtual_machine.system.machine.chipset` and `virtual_machine.system.machine.family` must be consistent.
7. Device and resource IDs must be unique within their scope.
8. Topology coordinates must be unique within the same topology namespace.
9. VM resource references must fail validation if no matching host catalog ID exists.
10. Canonical VM config must reject host-literal binding keys in guest-intent sections.

## Composition Rules

- Profile layering should resolve to one effective canonical model before rendering.
- Layering precedence must be explicit and documented by the runtime command.
- Conflicting immutable fields in composed profiles must fail validation.
- Layering must not alter resource-ID binding semantics (only values, never host-literal ownership rules).

## Validation Conformance Terms (CT-001 through CT-005)

The validation framework enforces five core contract terms, each with dedicated test coverage in `src/runtime_config/validation.rs`:

### CT-001: Valid Canonical Document Structure

**Definition:** A canonical YAML document is valid when all required fields are present, correctly typed, and consistent with validation rules.

**Test:** `passing_canonical_example` — validates happy path with metadata, system config, and resource entries

**Edge Cases Covered:**
- `optional_sections_omitted_is_valid` — Confirms storage, network, and resources sections are optional and document is valid without them
- `same_id_across_scopes_is_allowed` — Validates that identical IDs across different scopes (storage, network, resources) do not cause errors

### CT-002: Required Field Presence and Type Validation

**Definition:** All required fields (metadata, system, machine, cpu, memory) must be present and correctly typed; violations report field path and constraint.

**Tests:**
- `missing_required_field` — rejects empty cpu.model, reports precise field path
- `invalid_type_reports_field_path` — rejects memory.min as string, reports type constraint
- `collection_type_mismatch_reports_field_path` — rejects `virtual_machine.storage` when provided as a mapping instead of a list

**Edge Cases Covered:**
- `malformed_yaml_is_rejected_before_validation` — Syntax-invalid YAML is rejected at the parse boundary before conformance validation runs
- `multiple_missing_required_fields` — Multiple missing required fields (cpu.model and machine.chipset) reported simultaneously with precise paths
- `empty_vm_name` — Boundary test: empty string vm_name is properly rejected as required field
- `empty_schema_version` — Boundary test: empty string schema_version is properly rejected as required field
- `memory_minimum_zero_is_valid` — Edge boundary: memory.min = 0 is valid (constraint is >= 0, not > 0)

### CT-003: Resource ID Uniqueness Within Scope

**Definition:** IDs must be unique within storage, network, and resources scopes respectively; same ID allowed across different scopes (cross-scope reuse permitted).

**Tests:**
- `duplicate_ids` — detects duplicates in all three scopes simultaneously
- `empty_ids_report_precise_field_paths` — rejects empty IDs with indexed paths
- `duplicate_storage_id_reports_offending_entry_index` — targets duplicate (not first) occurrence
- `same_id_across_scopes_is_allowed` — confirms cross-scope ID reuse is valid

**Edge Cases Covered:**
- `multiple_duplicates_in_storage_scope` — Multiple instances (3+) of the same ID within a single scope; all duplicates after first are reported with correct indices

### CT-004: Machine Consistency and vm_name Enforcement

**Definition:** Machine family and chipset must be consistent (family "pc" requires chipset in [q35, i440fx]); vm_name must match filename stem.

**Tests:**
- `invalid_chipset_family_combination` — rejects arm-virt for family pc
- `vm_name_mismatch` — rejects vm_name that differs from filename stem

**Edge Cases Covered:**
- `unknown_machine_family` — Machine families outside the validated set (not "pc") are accepted as-is; chipset validation only applies when family is "pc"

### CT-005: Precise Field Path Error Reporting

**Definition:** All validation errors must include full field path (with array indices) and specific constraint violated, enabling deterministic error routing.

**Tests:**
- `invalid_type_reports_field_path` — includes full nesting: `virtual_machine.system.memory.min`
- `collection_type_mismatch_reports_field_path` — reports the collection path itself: `virtual_machine.storage`
- `empty_ids_report_precise_field_paths` — includes array indices: `storage[0].id`, `network[0].id`, `resources[0].id`
- `duplicate_storage_id_reports_offending_entry_index` — reports duplicate at [2], not [0]

**Edge Cases Covered:**
- `empty_schema_version` — Precise path reporting for empty required field: `metadata.schema_version`
- `empty_vm_name` — Precise path reporting for empty required field: `metadata.vm_name`
- `multiple_missing_required_fields` — Multiple field paths reported (e.g., `virtual_machine.system.cpu.model` and `virtual_machine.system.machine.chipset`)

---

## Planned Corpus-Based Conformance Matrix

This matrix documents the intended corpus-backed coverage target for Item 4. The current validation suite is still synthetic and does not yet execute these corpus cases directly.

| Test ID | Corpus Input | Focus | Expected Result |
|---------|--------------|-------|-----------------|
| CT-001 | [input/felucia/108.conf](../../../input/felucia/108.conf) | Canonical `metadata` + `virtual_machine` shape, boot order, storage/network intent, resource refs | Pass with stable mapping of system, boot, storage, network, and resource IDs |
| CT-002 | [input/coruscant/3101.conf](../../../input/coruscant/3101.conf) | Machine identity, chipset consistency, passthrough intent, topology-sensitive placement | Pass only if machine identity and placement constraints are preserved deterministically |
| CT-003 | [input/coruscant/3101.qemu.cmd.split](../../../input/coruscant/3101.qemu.cmd.split) | Topology identity and deterministic rendering inputs | Pass with stable topology coordinates and byte-stable argument ordering for equivalent inputs |
| CT-004 | [input/coruscant/902.qemu.cmd.split](../../../input/coruscant/902.qemu.cmd.split) | Legacy/bridge topology representation and render boundary | Pass if bridge/path placement is represented through canonical placement identity |
| CT-005 | [input/felucia/108.qemu.cmd.split](../../../input/felucia/108.qemu.cmd.split) | Firmware, TPM, and host-resource boundary | Pass if firmware/TPM identity is captured and host-literal details remain outside canonical guest intent |

## Global config file
`/etc/ezkvm/config.yaml`

```yaml
host: "proxmox9"
```

## Host config file
`/etc/ezkvm/host/proxmox9.yaml`

```yaml
state_path: "/var/run/qemu-server"
qemu:
  network:
    tap:
      script: "/usr/libexec/qemu-server/pve-bridge"
      downscript: "/usr/libexec/qemu-server/pve-bridgedown"
  ovmf:
    path: "/usr/share/pve-edk2-firmware"
    firmware:
      normal: "OVMF_CODE_4M.fd"
      secure: "OVMF_CODE_4M.secboot.fd"
resources:
  - id: "gpu0"
    type: "pcie"
    host:
      bdf: ["0000:03:00.0", "0000:03:00.1"]
  - id: "usb0"
    type: "usb"
    host:
      bus: 1
      port: "2.2"
```

## Typical VM canonical config file
`/etc/ezkvm/vm/win11-dev.yaml`

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"

virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
      version: "9.0"
      options:
        acpi: true
    firmware:
      type: "ovmf"
      secure_boot: true
    tpm:
      type: "swtpm"
      interface: "tpm-crb"
    cpu:
      model: "host"
    memory:
      min: 8192
      max: 16384
  boot:
    order: ["disk0", "net0"]
  storage:
    - id: "disk0"
      kind: "disk"
      controller: "virtio-scsi"
      role: "system"
  network:
    - id: "net0"
      driver: "virtio-net-pci"
      mac: "BC:24:11:9F:A0:E4"
      role: "primary"
  resources:
    - id: "gpu0"
    - id: "usb0"
```

## Notes

- This contract defines shape and semantics, not every optional field.
- Additional field catalogs should remain compatible with this contract.

## Edge Cases: Deferred / Out of Scope

The following edge cases identified during Item 2 are documented as out-of-scope for contract v1.0:

### Oversized Value Validation

**Rationale:** Numeric limits (e.g., maximum CPU count, memory ceiling) are host-dependent and require performance/resource profile validation beyond YAML schema scope. These belong in host-config validation or runtime resolution stages.

**Related fields:** `virtual_machine.system.cpu.cores`, `virtual_machine.system.memory.max`

### Resource Catalog Cross-Reference Validation

**Rationale:** Validating that VM `resources[].id` references match host catalog entries requires access to host configuration context, which is not available during canonical VM document validation. This validation is deferred to runtime resolution stage.

**Related contract term:** "VM resource references must fail validation if no matching host catalog ID exists" — validation deferred to runtime resolution.

### Full Machine Family Enumeration

**Rationale:** Only "pc" family is validation-enforced; other families (e.g., "arm-virt", "virt") are accepted as custom/future extensions. Full enumeration deferred until additional architectures are formally normalized into the schema.

**Current behavior:** Family "pc" → chipset [q35, i440fx] enforced; other families → no chipset constraint.
