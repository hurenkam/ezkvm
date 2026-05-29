# Schema Host Resource Boundary Slice

Status: Backlog
Date: 2026-05-29

## Purpose

Plan the first host/VM boundary slice for generic pass-through modeling so host-literal bindings stay in host config while VM config only carries stable resource references.

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

## Backlog Work Items

- Ripley: Define canonical `resources[]` host catalog contract (`id`, `type`, payload shape) and author fixture examples.
- Hicks: Specify VM-side `virtual_machine.resources[]` ID-only reference contract and unresolved-ID failure semantics.
- Bishop: Implement canonical validation guards that reject host-literal keys in VM guest-intent sections.
- Vasquez: Add deterministic host-resource resolution checks and resolver-focused regression tests.
- Newt: Expand corpus samples for PCI/USB-style pass-through without host-coordinate leakage in VM intent.
- Hudson: Produce validation evidence artifacts and traceable failure-path outputs for this slice.
- Apone: Coordinate dependency readiness, acceptance sign-off, and matrix/status synchronization.

## Dependencies

- [canonical-yaml-schema-contract](../../../dev/requirements/canonical-yaml-schema-contract.md)
- [canonical-yaml-validation](../features/done/canonical-yaml-validation.md)
- [input/felucia/108.conf](../../../input/felucia/108.conf)
- [input/coruscant/3101.conf](../../../input/coruscant/3101.conf)
- [input/coruscant/3101.qemu.cmd.split](../../../input/coruscant/3101.qemu.cmd.split)
- [input/felucia/108.qemu.cmd.split](../../../input/felucia/108.qemu.cmd.split)

## Test And Evidence

- Validate CT-005-style host-boundary behavior using [input/felucia/108.qemu.cmd.split](../../../input/felucia/108.qemu.cmd.split).
- Validate topology-sensitive resource binding behavior using [input/coruscant/3101.conf](../../../input/coruscant/3101.conf) and [input/coruscant/3101.qemu.cmd.split](../../../input/coruscant/3101.qemu.cmd.split).
- Evidence should include a host-resource resolution report or equivalent test log once implementation begins.