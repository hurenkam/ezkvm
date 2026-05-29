# Schema Host Resource Boundary Slice

Status: Prepared
Date: 2026-05-29

## Purpose

Implement the first host/VM boundary slice for generic pass-through modeling so host-literal bindings stay in host config while VM config only carries stable resource references.

## Requirement IDs

- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- RDR-002 Typed Internal Contracts
- RDR-006 Profile/Composition Friendly Config

## Scope

- Define host-side `resources[]` catalog entries with stable `id`, canonical `type`, and host-literal binding payload.
- Define VM-side `virtual_machine.resources[]` reference objects that carry `id` only.
- Enforce rejection of host-literal keys in canonical VM guest-intent sections.
- Preserve deterministic resolution of VM resource IDs against the host catalog.
- Cover PCI/USB-style examples without hard-coding host coordinates into canonical VM intent.

## Acceptance Criteria

- Host resources validate for required `id`, `type`, and host payload shape.
- VM resource references validate as ID-only objects and fail when a host resource ID is missing.
- Validation reports include the exact field path for rejected host literals or unresolved references.
- The slice demonstrates that generic pass-through modeling can be resolved without leaking host-specific details into the VM schema.

## Dependencies

- [canonical-yaml-schema-contract](../../../dev/requirements/canonical-yaml-schema-contract.md)
- [canonical-schema-conformance-matrix](canonical-schema-conformance-matrix.md)
- [input/felucia/108.conf](../../../../input/felucia/108.conf)
- [input/coruscant/3101.conf](../../../../input/coruscant/3101.conf)
- [input/coruscant/3101.qemu.cmd.split](../../../../input/coruscant/3101.qemu.cmd.split)
- [input/felucia/108.qemu.cmd.split](../../../../input/felucia/108.qemu.cmd.split)

## Test And Evidence

- Validate CT-005-style host-boundary behavior using [input/felucia/108.qemu.cmd.split](../../../../input/felucia/108.qemu.cmd.split).
- Validate topology-sensitive resource binding behavior using [input/coruscant/3101.conf](../../../../input/coruscant/3101.conf) and [input/coruscant/3101.qemu.cmd.split](../../../../input/coruscant/3101.qemu.cmd.split).
- Evidence should include a host-resource resolution report or equivalent test log once implementation begins.