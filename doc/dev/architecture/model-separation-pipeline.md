# Model Separation Pipeline Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define the normative boundary between import-host data, fixed machine layout, and runtime-host data, including deterministic handoff expectations.

## Requirement Traceability

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-005 Deterministic Command Generation
- FR-006 Validation Before Execution
- RDR-003 Layered Modular Architecture
- RDR-005 Deterministic Runtime Rendering

## Pipeline Stages

1. Import Stage
   - Inputs: source VM definition + import-host context
   - Output: fixed machine layout
2. Runtime Resolution Stage
   - Inputs: fixed machine layout + runtime-host context
   - Output: effective runtime model
3. Render Stage
   - Input: effective runtime model
   - Output: ordered QEMU argument vector

## Stage Contracts

1. Import stage must not access runtime-host probes or runtime-only path resolution.
2. Runtime resolution must not reinterpret source-specific syntax.
3. Render stage must be side-effect free and deterministic.
4. Validation must run at each boundary and must fail fast when contract violations are detected.

## Data Ownership Rules

- Import-host data should capture source assumptions that are needed to normalize machine intent.
- Fixed machine layout must represent source-agnostic guest intent.
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
  -> fixed_machine_layout=F

resolve_runtime(fixed=F, runtime_host=H2)
  -> effective_runtime=R

render_qemu_args(runtime=R)
  -> ["-machine", "q35", "-m", "8192", ...]
```

If H1 and H2 are unchanged and F is unchanged, rendering must produce the same argument vector order.