# Import Adapters Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define the contract for source-specific import adapters that transform external VM definitions into the canonical virtual_machine model consumed by runtime generation.

## Requirement Traceability

- FR-002 Multiple Import Sources
- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- NFR-004 Extensibility
- RDR-003 Layered Modular Architecture

## Contract

1. Each import source must be implemented as a dedicated adapter module.
   - Rust scaffold location: `src/import_stage/`
2. An adapter must accept:
   - Source payload (for example Proxmox config, raw QEMU CLI capture, or libvirt XML)
   - Import-host context (facts known at import time)
3. An adapter must produce:
   - Canonical virtual_machine model data only
   - Structured diagnostics (errors/warnings) with source locations when available
4. An adapter must not perform runtime-host resolution (paths, binaries, host device probing).
5. Adapter output must be normalized enough for deterministic downstream rendering.

## Validation Boundary

- Adapter stage must validate source syntax and semantic consistency that can be checked without runtime host data.
- Adapter stage should warn (not fail) on source fields that are ignored by policy, if conversion can continue safely.
- Runtime capability checks must be deferred to runtime preflight.

## Extension Rules

1. New adapters must map source-specific terms into canonical terms, not expand canonical model shape per source.
2. New adapter integration should only require:
   - Registering adapter ID
   - Implementing parse + map + diagnostics contract
   - Adding conformance tests for representative source inputs
3. Core runtime generation must remain unchanged when adding a source adapter.

## Current Scaffold

`CanonicalYamlImportStage` validates canonical YAML input against the current schema.
`ProxmoxConfImportStage` now performs a minimal, deterministic Proxmox `.conf` to canonical mapping behind the same `ImportStage` trait.

## Current Proxmox Mapping

- `name` -> `metadata.vm_name`
- `machine` -> `virtual_machine.system.machine.family` + `virtual_machine.system.machine.chipset`
- `cpu` -> `virtual_machine.system.cpu.model`
- `memory` -> `virtual_machine.system.memory.min`
- `scsi*` -> `virtual_machine.storage[].id`
- `net*` -> `virtual_machine.network[].id`
- `hostpci*`, `usb*` -> `virtual_machine.resources[].id`
- Unsupported source names remain typed failures; the adapter does not attempt general Proxmox syntax coverage yet

## Corpus-Backed Evidence

The current Proxmox adapter baseline is exercised against representative corpus samples from three hosts:

- `input/felucia/108.conf` -> `wakiza`, `q35`, storage `scsi0`/`scsi1`, network `net0`, resources `hostpci0`/`usb0`
- `input/coruscant/3101.conf` -> `gyndine`, `q35`, storage `scsi0`, no network entries, resources `hostpci0` through `hostpci11`
- `input/zbp-server-mh2/103.conf` -> `desktop-markh-3`, `q35`, storage `scsi0`/`scsi1`, network `net0`, no passthrough resources

Each imported document is validated with `validate_canonical_document` against the canonical output filename derived from `metadata.vm_name`.

## Known Validation Limit

Many Proxmox corpus files use numeric source filenames such as `108.conf` or `3101.conf` while the canonical document uses the guest name (`wakiza`, `gyndine`) as `metadata.vm_name`.

This means:

- validating the imported document against the original `.conf` source path is expected to fail the filename-stem rule
- validating the imported document against the emitted canonical YAML path (for example `wakiza.yaml`) is expected to pass when the mapped document is otherwise conformant

This is a boundary between source import fidelity and canonical file naming, not a parser defect.

## Minimal Example

```text
Adapter: proxmox
Input:  machine: q35, cpu: host, memory: 8192
Output: virtual_machine
   system:
      chipset: q35
      cpu:
         model: host
      memory:
         min: 8192
Diagnostics: []
```

## Non-Goals

- Defining runtime-host path precedence
- Encoding distribution-specific package assumptions
- Preserving source format fidelity when it conflicts with canonical model semantics