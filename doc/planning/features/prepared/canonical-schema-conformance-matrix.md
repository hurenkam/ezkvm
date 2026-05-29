# Canonical Schema Conformance Matrix

Status: Prepared
Date: 2026-05-29

## Purpose

Turn the canonical YAML contract into an executable conformance matrix with corpus-backed validation for the core schema shape, rendering order, and diagnostics.

## Requirement IDs

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- NFR-001 Usability
- RDR-002 Typed Internal Contracts
- RDR-006 Profile/Composition Friendly Config

## Scope

- Map CT-001 through CT-005 from [canonical-yaml-schema-contract](../../../dev/requirements/canonical-yaml-schema-contract.md) into named validation cases.
- Validate canonical `metadata` and `virtual_machine` shape, required core fields, and ordering-sensitive sections.
- Confirm deterministic diagnostics for field-path violations and uniqueness failures.
- Keep corpus expectations tied to the documented host-resource boundary and topology rules.

## Acceptance Criteria

- Each matrix entry has a stable test name, input corpus, expected result, and explicit contract reference.
- Validation covers required fields, chipset/family consistency, resource-ID uniqueness, and host-literal rejection in canonical guest-intent sections.
- Output reports include field path plus reason for each failure class.
- The matrix is sufficient to gate contract-first changes before implementation work lands.

## Dependencies

- [canonical-yaml-schema-contract](../../../dev/requirements/canonical-yaml-schema-contract.md)
- [requirements-traceability-matrix](../../backlog/requirements-traceability-matrix.md)
- [input/felucia/108.conf](../../../../input/felucia/108.conf)
- [input/coruscant/3101.conf](../../../../input/coruscant/3101.conf)
- [input/coruscant/3101.qemu.cmd.split](../../../../input/coruscant/3101.qemu.cmd.split)
- [input/coruscant/902.qemu.cmd.split](../../../../input/coruscant/902.qemu.cmd.split)
- [input/felucia/108.qemu.cmd.split](../../../../input/felucia/108.qemu.cmd.split)

## Test And Evidence

- CT-001: [input/felucia/108.conf](../../../../input/felucia/108.conf)
- CT-002: [input/coruscant/3101.conf](../../../../input/coruscant/3101.conf)
- CT-003: [input/coruscant/3101.qemu.cmd.split](../../../../input/coruscant/3101.qemu.cmd.split)
- CT-004: [input/coruscant/902.qemu.cmd.split](../../../../input/coruscant/902.qemu.cmd.split)
- CT-005: [input/felucia/108.qemu.cmd.split](../../../../input/felucia/108.qemu.cmd.split)
- Evidence should include the validation report or test log produced from this matrix once implementation starts.