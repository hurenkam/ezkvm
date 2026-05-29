# ezkvm Initial Product Requirements

Status: Draft
Date: 2026-05-28

## 1. Product Intent

ezkvm is an easy-to-use drop-in replacement for running Proxmox-origin virtual machines on Linux distributions outside Proxmox, while remaining generic enough to support additional import sources over time.

## 2. Goals

- Run Proxmox-origin virtual machines on non-Proxmox Linux hosts with minimal migration friction.
- Keep the architecture import-source-agnostic so additional sources can be added without redesigning the core model.
- Use a simple YAML-based core configuration schema that stays close in size and readability to the originating VM configuration.
- Separate concerns between import-host assumptions, fixed guest machine model, and runtime-host assumptions.

## 3. Functional Requirements

### FR-001 Drop-In Runtime Experience

The system must allow a user to run a Proxmox-origin VM on supported Linux hosts with a workflow that is straightforward and operationally predictable.

### FR-002 Multiple Import Sources

The system must support import from at least these source families:

- Proxmox VM config
- Captured raw QEMU command line
- Libvirt-managed QEMU VM definitions

The architecture must allow additional import sources to be added via explicit import adapters.

### FR-003 Canonical YAML Core Config

The system must produce and consume a canonical YAML VM configuration schema.

The schema must be:

- Human-readable
- Explicit enough for deterministic runtime generation
- Compact enough to avoid unnecessary expansion compared to the source VM definition

### FR-004 Model Separation Pipeline

The system must enforce a three-part model:

1. Import-host-specific data
2. Fixed machine layout (canonical machine model)
3. Runtime-host-specific data

Import stage requirement:

- Import-host data + source VM definition -> fixed machine layout

Runtime stage requirement:

- Fixed machine layout + runtime-host data -> generated QEMU command line

### FR-005 Deterministic Command Generation

Given the same fixed machine layout and runtime-host data, generated QEMU arguments must be deterministic.

### FR-006 Validation Before Execution

The system must validate configuration and required runtime capabilities before launching execution.

### FR-007 Dry-Run Preview

The system should provide a dry-run mode that renders the effective QEMU command and reports capability resolution outcomes without launching the VM.

## 4. Non-Functional Requirements

### NFR-001 Usability

Configuration and CLI workflow should be understandable by operators familiar with QEMU/Proxmox VM concepts.

### NFR-002 Portability

The project must target Linux hosts and avoid hard dependencies on a single distribution-specific management stack.

### NFR-003 Reliability

Validation and runtime error reporting must be actionable and specific enough to diagnose missing host capabilities or incompatible settings.

### NFR-004 Extensibility

Import-source integration should be modular so new import adapters can be introduced with minimal impact on core runtime generation.

### NFR-005 Observability

Startup preflight, dry-run, and runtime failures should expose enough context for troubleshooting.

## 5. Out of Scope For Initial Requirements

- Hypervisor orchestration beyond QEMU/KVM
- Non-Linux runtime hosts
- Distribution packaging policy details

## 6. Acceptance Criteria (Initial)

- The requirements baseline is captured in `doc/dev/requirements` and reviewed.
- The documented import/runtime separation model is used as a constraint for subsequent architecture decisions.
- New feature planning references these requirements by requirement ID where applicable.
