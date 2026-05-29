# Canonical YAML Schema Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define the canonical YAML structure that represents fixed machine layout and runtime-host overlays in a human-readable, deterministic form.

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
   - `fixed_machine`
   - `runtime_overrides`
3. `fixed_machine` must be source-agnostic and sufficient to describe guest intent.
4. `runtime_overrides` must be optional and host-resolution-oriented.
5. Unknown top-level keys must be rejected unless explicitly marked as extension namespace.

## Required Core Fields

- `metadata.schema_version`: semantic schema version string.
- `metadata.vm_name`: stable VM identifier.
- `fixed_machine.chipset`: canonical chipset identifier.
- `fixed_machine.cpu.model`: canonical CPU model intent.
- `fixed_machine.memory.mib`: integer MiB memory value.

## Validation Rules

1. Required fields must be present and type-correct.
2. Scalar normalization must be deterministic (for example booleans and integers).
3. List fields that influence rendering order must define stable ordering semantics.
4. Validation errors must include field path and reason.

## Composition Rules

- Profile layering should resolve to one effective canonical model before rendering.
- Layering precedence must be explicit and documented by the runtime command.
- Conflicting immutable fields in composed profiles must fail validation.

## Minimal Example

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"

fixed_machine:
  chipset: "q35"
  cpu:
    model: "host"
  memory:
    mib: 8192

runtime_overrides:
  devices:
    tpm: "auto"
```

## Notes

- This contract defines shape and semantics, not every optional field.
- Additional field catalogs should remain compatible with this contract.