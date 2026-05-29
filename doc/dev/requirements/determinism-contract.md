# Determinism Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define deterministic behavior requirements for effective model resolution and QEMU argument rendering.

## Requirement Traceability

- FR-005 Deterministic Command Generation
- FR-007 Dry-Run Preview
- FR-006 Validation Before Execution
- NFR-003 Reliability
- NFR-005 Observability
- RDR-005 Deterministic Runtime Rendering

## Determinism Requirements

1. Equivalent effective inputs must produce byte-equivalent argument vectors.
2. Argument ordering must follow a documented canonical order.
3. Normalized defaults must be applied consistently and explicitly.
4. Rendering must not depend on map iteration order or nondeterministic host calls.
5. Dry-run output must match launch-time rendering for the same effective model.

## Equivalence Definition

Two inputs are equivalent when:

- Canonical fixed machine layout is semantically equal after normalization.
- Runtime-host resolved capabilities and selected defaults are equal.
- Explicit overrides and profile layering results are equal.

## Canonicalization Rules

1. Keys in normalized diagnostics and dry-run output should be emitted in stable order.
2. Multi-value device options must preserve deterministic ordering.
3. Automatically assigned IDs should derive from stable deterministic inputs.

## Failure Handling

- If deterministic rendering cannot be guaranteed, execution must fail with a clear error.
- Error output must include the nondeterministic factor that prevented deterministic rendering.

## Minimal Example

```text
dry-run #1 args:
["-machine","q35","-cpu","host","-m","8192"]

dry-run #2 args:
["-machine","q35","-cpu","host","-m","8192"]
```

The two dry-run outputs must remain equal for equivalent inputs.