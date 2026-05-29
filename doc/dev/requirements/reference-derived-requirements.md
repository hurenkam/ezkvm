# Reference-Derived Requirements (Generic Only)

Status: Draft
Date: 2026-05-28

This document records additional requirements derived from the reference repository after filtering out implementation-specific assumptions.

## Source Inputs Reviewed

- Reference repository root README
- `doc/dev/architecture/architecture-guidelines.md`
- `doc/dev/analysis/codebase-analysis.md`
- `doc/dev/analysis/ezkvm-comparison.md`

## Imported Generic Requirements

### RDR-001 Direct QEMU/KVM Orientation

The product should focus on direct QEMU/KVM execution rather than requiring a libvirt abstraction layer.

### RDR-002 Typed Internal Contracts

Internal machine/config contracts should be strongly typed to reduce runtime ambiguity and improve validation quality.

### RDR-003 Layered Modular Architecture

The codebase should keep unidirectional flow between CLI/orchestration, config/model, command generation, and host adapters.

### RDR-004 Fail-Fast Validation

Validation should occur before execution to reduce late runtime failure where possible.

### RDR-005 Deterministic Runtime Rendering

Equivalent inputs should render equivalent command lines and machine layout decisions.

### RDR-006 Profile/Composition Friendly Config

The configuration model should support composition mechanisms (for example profile layering or reusable fragments) without sacrificing clarity of the final effective model.

### RDR-007 Environment-Aware Runtime Resolution

Runtime host capability and path resolution should be explicit and diagnosable.

## Explicitly Excluded As Non-Generic

The following categories were intentionally excluded from imported requirements because they are reference-implementation-specific:

- Proxmox-specific runtime paths, slot placements, and parity invariants
- Packaging constraints tied to specific distro targets
- Legacy architecture decisions that constrain redesign freedom

## Mapping Into Initial Product Requirements

- RDR-001 -> FR-001, FR-002
- RDR-003 -> NFR-004
- RDR-004 -> FR-006
- RDR-005 -> FR-005
- RDR-006 -> FR-003
- RDR-007 -> NFR-003, NFR-005
