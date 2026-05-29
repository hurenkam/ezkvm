---
name: "design-patterns"
description: "Reusable stage and adapter patterns for ezkvm_v3's import, model, runtime, and render pipeline"
---

## Context

ezkvm_v3 is organized as a staged pipeline:

- import adapters normalize source-specific VM definitions
- `vm_spec` holds the canonical VM model and validation
- `runtime_resolution` applies host-bound runtime facts
- `render_stage` emits deterministic QEMU arguments

Use this skill when designing a new module, trait boundary, or source adapter inside that pipeline.

## Patterns

### Trait-Based Stage Boundary

Use a small object-safe trait when a stage needs to be selected behind an interface.

- keep request and response types narrow
- prefer an associated error type
- avoid generic methods, `async fn`, and `Self` in return positions
- validate at the boundary before handing data to the next stage

### Adapter Scaffold

Give each import source its own module under `src/import_stage/`.

- parse source payload plus import-host context
- map source terms into canonical model fields
- emit structured diagnostics with source locations when available
- do not probe runtime-host state or mutate runtime-only paths

### Deterministic Rendering

The render stage must be pure and repeatable.

- accept an effective runtime model only
- produce a stable, ordered QEMU argument vector
- keep quoting and ordering canonical
- avoid side effects and host probes inside render

### Validation Before Execution

Validate early, then fail fast on contract violations.

- import stage validates source syntax and source-local semantics
- runtime resolution validates host-bound assumptions
- render stage assumes it receives a valid, effective runtime model
- warnings are acceptable for ignorable source fields; silent drops are not

## Design Checklist

Before proposing a new stage or helper, confirm:

1. Which stage owns the responsibility?
2. What is the smallest valid request/response shape?
3. What is validated here versus the next stage?
4. Is deterministic output required?
5. Does the change need a decision note because it affects shared conventions?

## Anti-Patterns

- combining import, runtime resolution, and rendering in one module
- letting adapters infer runtime-host facts
- making render depend on filesystem or environment discovery
- introducing broad shared abstractions before the pipeline has multiple implementations