# Model Separation Pipeline Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define the normative boundary between import-host data, canonical virtual_machine model data, and runtime-host data, including deterministic handoff expectations.

## Requirement Traceability

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-005 Deterministic Command Generation
- FR-006 Validation Before Execution
- RDR-003 Layered Modular Architecture
- RDR-005 Deterministic Runtime Rendering

## Pipeline Stages

1. Config Importer Stage
   - Inputs: source VM definition + import-host context
    - Output: canonical virtual_machine model
2. Runtime Resolution Stage
    - Inputs: canonical virtual_machine model + runtime-host context
   - Output: effective runtime model
3. Render Stage
   - Input: effective runtime model
   - Output: ordered QEMU argument vector

## Implementation Naming (Current Scaffold)

- `src/config_importer/`: config importer stage orchestration boundary
- `src/vm_spec/`: canonical VM specification schema and validation boundary
- `src/runtime_resolution/`: runtime resolution stage boundary
- `src/render_stage/`: deterministic render stage boundary

## Stage Abstractions (Current)

- Config importer stage now exposes a trait boundary (`ConfigImporter`) with a generic config contract (`ConfigArgs`) and current implementations for ezkvm YAML (`EzkvmConfigImporter`) plus a Proxmox `.conf` scaffold (`ProxmoxConfigImporter`).
- Config importer stage now exposes a trait boundary (`ConfigImporter`) with a generic config contract (`ConfigArgs`) and current implementations for ezkvm YAML (`EzkvmConfigImporter`) plus a minimal Proxmox `.conf` adapter (`ProxmoxConfigImporter`) that maps name, machine, cpu, memory, and stable slot IDs into the canonical model.
- Render stage now exposes a trait boundary (`RenderStage`) with a minimal request contract (`RenderRequest`) and default implementation (`DeterministicRenderStage`).
- Runtime resolution currently publishes a minimal `EffectiveRuntimeModel` placeholder consumed by render.

Trait boundaries are synchronous and object-safe by default (no generic method parameters, no `async fn`, no `Self` in return positions).

## Stage Contracts

1. Import stage must not access runtime-host probes or runtime-only path resolution.
2. Runtime resolution must not reinterpret source-specific syntax.
3. Render stage must be side-effect free and deterministic.
4. Validation must run at each boundary and must fail fast when contract violations are detected.

## Trait Constraints

- Sync vs async: keep sync now; no current stage performs I/O that requires async in the contract.
- Object safety: required so stage implementations can be selected behind trait objects later.
- Error typing: each trait uses an associated `Error` type so implementations can remain specific without forcing a global error enum too early.

## Data Ownership Rules

- Import-host data should capture source assumptions that are needed to normalize machine intent.
- Canonical virtual_machine model must represent source-agnostic guest intent.
- Runtime-host data must capture executable environment facts only.

## Allowed Transformations

- Source-specific keys -> canonical keys: allowed in import stage.
- Host capability fallback selection: allowed in runtime resolution stage.
- Argument ordering and quoting canonicalization: allowed in render stage.

## Disallowed Transformations

- Runtime-specific path mutation during import stage.
- Source syntax fallback logic during render stage.
- Silent field drops without diagnostics at stage boundaries.

## Minimal Example

```text
import(source=proxmox.conf, import_host=H1)
  -> canonical_vm=C

resolve_runtime(canonical_vm=C, runtime_host=H2)
  -> effective_runtime=R

render_qemu_args(runtime=R)
  -> ["-machine", "q35", "-m", "8192", ...]
```

If H1 and H2 are unchanged and C is unchanged, rendering must produce the same argument vector order.