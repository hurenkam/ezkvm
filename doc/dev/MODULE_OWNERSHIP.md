# Module Ownership Map

Date: 2026-04-15  
Status: Active  
Related: `doc/dev/ARCHITECTURE_GUIDELINES.md`, `doc/dev/adr/ADR-0001-base-selection.md`

## Purpose

This document defines ownership boundaries for core modules and enforces unidirectional dependency flow:

`CLI -> Config -> QEMU -> Runtime -> OS`

This map is used during design and review to prevent cross-layer coupling and architecture drift.

## Ownership Matrix

| Module | Responsibility | Owns | Must Not Own | Primary Code References |
|---|---|---|---|---|
| `cli/` | Parse user input and dispatch use-cases | Command shape, argument validation, command routing | Schema internals, QEMU flag composition, direct process orchestration details | `src/main.rs`, `src/cli/types.rs`, `src/cli/commands.rs` |
| `config/` | Define/validate schema and produce runtime-ready config | Schema types, load/merge logic, validation, central config | CLI parsing, process spawn/kill, device hotplug execution | `src/config/entrypoint.rs`, `src/config/loader/mod.rs`, `src/config/validation.rs` |
| `qemu/` | Convert typed config into deterministic QEMU command/process operations | Argument builders, command manager, process execution/inspection | CLI and schema loading concerns | `src/qemu/manager.rs`, `src/qemu/executor.rs`, `src/qemu/process/mod.rs` |
| `runtime/` | Coordinate lifecycle workflows around QEMU and auxiliary tools | Start/stop orchestration, dry-run flow, auxiliary launch sequencing | Schema mutation, argument parsing | `src/cli/runtime/start.rs`, `src/cli/runtime/auxiliary/mod.rs`, `src/cli/runtime/ops.rs` |
| `import/` | Normalize external config formats into canonical schema | Parser adapters, intermediate import models, canonical mapping contracts | Runtime orchestration and direct QEMU invocation | `src/import/mod.rs` (module boundary), `doc/dev/adr/ADR-0002-import-normalization-contract.md` |
| `state/` | Persist and query local runtime metadata | PID cache, config cache, log/session path conventions | Business logic for config merge or command routing | `src/state/pid.rs`, `src/state/cache.rs`, `src/state/paths.rs` |

## Dependency Direction

Allowed direction:

1. `cli -> config`
2. `cli -> runtime`
3. `runtime -> qemu`
4. `runtime -> state`
5. `qemu -> state`
6. `import -> config`

Disallowed direction:

1. `config -> cli`
2. `config -> runtime`
3. `qemu -> cli`
4. `state -> cli/config/runtime/qemu`
5. `runtime -> config` mutation after validation (except read-only access)

## Anti-Patterns (Do Not Introduce)

1. Circular imports between module layers
2. Cross-layer callbacks that bypass orchestrators
3. Global singleton coordinators for mutable state ownership
4. Business rules embedded in adapter/process code
5. CLI handlers directly mutating schema internals

## Review Checks for Layer Correctness

Use this quick review set on every architecture-affecting change:

1. Does the change introduce a new dependency edge? If yes, is it in allowed direction?
2. Is schema logic still in `config/` and out of `cli/` and `runtime/`?
3. Is process/QEMU orchestration still in `qemu/` + `runtime/`?
4. Does `state/` remain storage-only and free of business rules?
5. If touching `import/`, does output remain canonical schema only?

## Versioning Notes

- 2026-04-15: Initial ownership map introduced as part of Phase 0 / A-02.
- 2026-04-15: Added reserved `src/import/mod.rs` boundary to anchor Phase 1 import work.
