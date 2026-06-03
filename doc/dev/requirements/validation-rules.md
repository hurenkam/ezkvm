# Canonical YAML Validation Rules

This document describes the validation rules enforced on canonical YAML VM configurations, organized by CT (Canonical Testing) requirement identifier and mapped to product requirements (FR/NFR).

## Overview

Validation is performed in two layers:

1. **Parsing** deserializes YAML into `RuntimeConfig` and enforces structural correctness via serde
2. **Conformance** validates semantic rules and business logic

Conformance validation issues are collected in one pass to support bulk remediation. Parse-layer failures are returned as serde/YAML decode errors and short-circuit at the first structural/type mismatch encountered.

Collected `ValidationIssue` diagnostics include severity, remediation guidance, and best-effort YAML source context for conformance-layer failures.

## Validation Rule Reference

| CT | Scope | Implemented checks | Primary evidence |
|---|---|---|---|
| `CT-001` | Required canonical shape | Required sections and fields, optional section omission accepted | `passing_canonical_example`, `optional_sections_omitted_is_valid` |
| `CT-002` | Parse-layer correctness | serde decode enforcement for missing fields and type mismatches, plus malformed YAML rejection | `missing_required_field`, `invalid_type_reports_field_path`, `collection_type_mismatch_reports_field_path`, `malformed_yaml_is_rejected_before_validation` |
| `CT-003` | Scope-local ID rules | Non-empty IDs, duplicate detection per storage/network/resources scope, cross-scope reuse allowed | `duplicate_ids`, `empty_ids_report_precise_field_paths`, `duplicate_storage_id_reports_offending_entry_index`, `same_id_across_scopes_is_allowed` |
| `CT-004` | Semantic policy | `vm_name` filename match, `pc` chipset restriction to `q35` or `i440fx` | `vm_name_mismatch`, `invalid_chipset_family_combination`, `unknown_machine_family` |
| `CT-005` | Diagnostics precision | Full field paths, indexed collection paths, formatter coverage for human and JSON output | `invalid_type_reports_field_path`, `duplicate_storage_id_reports_offending_entry_index`, `human_readable_report_formatting`, `json_report_formatting` |

## CT-001: Valid Canonical Document with All Required Fields

**Requirement:** A valid canonical YAML must include all required metadata and system fields.

**Related Specification:** [Canonical YAML Schema Contract](canonical-yaml-schema-contract.md)

**Required Fields:**

| Field Path | Type | Description |
|---|---|---|
| `metadata.schema_version` | string | Semantic schema version (e.g., "1.0.0") |
| `metadata.vm_name` | string | VM identifier; must match config filename stem |
| `virtual_machine.system.machine.family` | string | Machine family (e.g., "pc") |
| `virtual_machine.system.machine.chipset` | string | Chipset type (e.g., "q35", "i440fx") |
| `virtual_machine.system.cpu.model` | string | CPU model intent (e.g., "host", "qemu64") |
| `virtual_machine.system.memory.min` | integer | Minimum RAM in MiB (>= 0) |

**Valid Example:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-prod"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
  storage:
    - id: "disk0"
  network:
    - id: "net0"
```

**When Validation Passes:**
- All required fields are present
- String fields are non-empty
- Memory.min is an integer >= 0
- All subsequent validation rules pass

## CT-002: Required Field Validation and Type Correctness

**Requirement:** Parsing rejects configurations with missing required fields or type mismatches.

**Validation Rules:**

### V-001: Non-Empty String Validation

**Description:** All required string fields must be present and non-empty (after whitespace trimming).

**Affected Fields:**
- `metadata.schema_version`
- `metadata.vm_name`
- `virtual_machine.system.machine.family`
- `virtual_machine.system.machine.chipset`
- `virtual_machine.system.cpu.model`

**Parse diagnostic (serde):**

```
metadata: missing field `schema_version`
```

Empty/whitespace-only values still fail in conformance with `ValidationIssue` output (for example `is required and must be a non-empty string`).

**When Violated:**

Missing field:
```yaml
metadata:
  vm_name: "win11-dev"
  # schema_version omitted
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
```

Empty/whitespace-only field:
```yaml
metadata:
  schema_version: "   "
  vm_name: "win11-dev"
  ...
```

**How to Fix:**
- Ensure all required string fields are present in the YAML structure
- Remove leading/trailing whitespace or use explicit values
- Follow semantic versioning for `schema_version` (e.g., "1.0.0")

### V-002: Memory Type and Range Validation

**Description:** The `virtual_machine.system.memory.min` field must be an integer >= 0.

**Parse diagnostic (serde) for type mismatch:**

```
virtual_machine.system.memory.min: invalid type: string "8192", expected i64
```

Negative numeric values are parsed successfully but fail conformance with:

```
path: "virtual_machine.system.memory.min"
reason: "must be an integer >= 0"
remediation: "Change memory.min to a non-negative value"
```

**When Violated:**

Type mismatch:
```yaml
virtual_machine:
  system:
    memory:
      min: "8192"  # String instead of integer
```

Negative value:
```yaml
virtual_machine:
  system:
    memory:
      min: -1024  # Must be >= 0
```

**How to Fix:**
- Use YAML integer syntax without quotes: `min: 8192`
- Ensure value is >= 0
- Typical VM memory ranges: 512–65536 MiB

## CT-003: Resource ID Uniqueness Within Scopes

**Requirement:** All resource IDs must be unique within their own scope (storage, network, resources). IDs are allowed to repeat across scopes.

**Validation Rules:**

### V-003: Storage ID Uniqueness

**Description:** Each `virtual_machine.storage[*].id` must be unique within the storage list.

**Error message:**

```
path: "virtual_machine.storage[2].id"
reason: "duplicate id 'disk0'"
remediation: "Change the id to a unique value; 'disk0' is already used in storage"
```

**When Violated:**

```yaml
virtual_machine:
  storage:
    - id: "disk0"
    - id: "disk1"
    - id: "disk0"  # Duplicate; error reported at index [2]
```

**How to Fix:**
- Ensure each storage entry has a unique identifier
- Common naming: `disk0`, `disk1`, `cdrom0`, etc.

### V-004: Network ID Uniqueness

**Description:** Each `virtual_machine.network[*].id` must be unique within the network list.

**Error message:**

```
path: "virtual_machine.network[1].id"
reason: "duplicate id 'eth0'"
remediation: "Change the id to a unique value; 'eth0' is already used in network"
```

**When Violated:**

```yaml
virtual_machine:
  network:
    - id: "eth0"
    - id: "eth0"  # Duplicate; error reported at index [1]
```

**How to Fix:**
- Each network interface must have a distinct identifier
- Common naming: `eth0`, `eth1`, `net0`, etc.

### V-005: Resource ID Uniqueness

**Description:** Each `virtual_machine.resources[*].id` must be unique within the resources list.

**Error message:**

```
path: "virtual_machine.resources[2].id"
reason: "duplicate id 'gpu0'"
remediation: "Change the id to a unique value; 'gpu0' is already used in resources"
```

**When Violated:**

```yaml
virtual_machine:
  resources:
    - id: "gpu0"
    - id: "vfio1"
    - id: "gpu0"  # Duplicate; error reported at index [2]
```

**How to Fix:**
- Ensure each passthrough resource has a unique identifier
- Common naming: `gpu0`, `usb0`, `vfio1`, etc.

### Cross-Scope Duplication is Allowed

**Valid Example:**

```yaml
virtual_machine:
  storage:
    - id: "shared0"
  network:
    - id: "shared0"  # Same ID allowed in different scope
  resources:
    - id: "shared0"  # Same ID allowed in different scope
```

## CT-004: Filename and Chipset Policy Validation

**Requirement:** VM name must match the configuration filename stem, and chipset must be consistent with machine family.

**Validation Rules:**

### V-006: VM Name Matches Filename Stem

**Description:** The `metadata.vm_name` value must match the filename stem (filename without extension).

**Error message:**

```
path: "metadata.vm_name"
reason: "must match filename stem 'production-vm'"
```

**When Violated:**

Config file: `production-vm.yaml`
```yaml
metadata:
  vm_name: "dev-vm"  # Does not match filename
  schema_version: "1.0.0"
```

**How to Fix:**
- Rename the YAML file to match the vm_name value: `dev-vm.yaml`
- OR update vm_name to match the filename: `vm_name: "production-vm"`

### V-007: Chipset-Family Consistency

**Description:** When machine family is `"pc"`, the chipset must be one of the supported x86 options: `"q35"` or `"i440fx"`.

**Error message:**

```
path: "virtual_machine.system.machine.chipset"
reason: "must be one of [q35, i440fx] when machine family is 'pc'"
```

**Decision Basis:** See [Squad Decisions](../../planning/project-plan.md) — canonical machine model is authoritative for guest-visible topology.

**When Violated:**

```yaml
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "arm-virt"  # Invalid for pc family
```

**How to Fix:**
- For x86 VMs, use `q35` (modern, UEFI/OVMF) or `i440fx` (legacy, BIOS)
- Choose based on guest OS and feature needs:
  - **q35:** Windows 11, modern Linux, UEFI SecureBoot
  - **i440fx:** Windows 7, legacy BIOS-only systems

## CT-005: Precise Field Path Error Reporting

**Requirement:** All validation errors include precise field paths with array indices, enabling tools to pinpoint exact configuration problems.

**Field Path Format:**

| Scenario | Example Path |
|---|---|
| Simple field | `metadata.vm_name` |
| Nested field | `virtual_machine.system.machine.chipset` |
| Array item | `virtual_machine.storage[0].id` |
| array with index | `virtual_machine.network[3].id` |

**Key Principles:**

1. Array indices are 0-based
2. For duplicate detection, the error is reported at the index of the duplicate entry (not the first occurrence)
3. Error contexts include the full canonical field path for programmatic parsing

**Example Error Output:**

```json
{
  "issues": [
    {
      "path": "virtual_machine.storage[2].id",
      "reason": "duplicate id 'disk0'"
    },
    {
      "path": "virtual_machine.system.memory.min",
      "reason": "must be an integer >= 0"
    }
  ]
}
```

**Integration Notes:**

When implementing tools that process these errors:
- Parse field paths with regex: `^([a-z_][a-z0-9_.]*?)(\[\d+\])?$`
- Use array indices to locate exact YAML array elements
- Batch multiple issues together for bulk remediation

## Implementation Reference

See [Validation Examples](../architecture/validation-examples.md) for runnable code examples and [Coding Guidelines](../architecture/coding-guidelines.md#validation-layer-implementation) for implementation patterns.

**Test Coverage:** All rules are validated by the test suite in [src/runtime_config/validation.rs](../../../src/runtime_config/validation.rs).
