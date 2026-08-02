---
phase: 08-vm-lifecycle
plan: 01
subsystem: infra
tags: [rust, clap, yaml, qemu, lifecycle]
requires: []
provides:
  - clap-driven VM lifecycle CLI skeleton with start/status tracer path
  - host.yaml loading, detached-process spawn helpers, and YAML VM state persistence
  - fake_qemu test binary plus lifecycle/security integration tests
affects: [vm-lifecycle, qmp, swtpm, ui-client]
tech-stack:
  added: [clap, serde_json, libc]
  patterns: [yaml-backed host config, detached child spawning, PID-based stale-state cleanup]
key-files:
  created: [src/lifecycle/mod.rs, src/lifecycle/host_config.rs, src/lifecycle/process.rs, src/lifecycle/start.rs, src/lifecycle/status.rs, src/lifecycle/vm_handle.rs, src/bin/fake_qemu.rs, tests/vm_lifecycle.rs, tests/cli_security.rs]
  modified: [Cargo.toml, src/lib.rs, src/main.rs]
key-decisions:
  - "Host lifecycle state lives in host-configured state_dir as YAML VmHandle files."
  - "The tracer path uses detached qemu spawning plus PID-only staleness cleanup."
patterns-established:
  - "Lifecycle entrypoints live under src/lifecycle/ and are exposed from the library for direct tests."
  - "Integration tests use the compiled fake_qemu sibling binary instead of host-installed qemu."
requirements-completed: [VMGR-01]
coverage:
  - id: D1
    description: "Start/status tracer path loads host.yaml, starts a detached VM process, and persists/read-cleans YAML state."
    requirement: "VMGR-01"
    verification:
      - kind: integration
        ref: "tests/vm_lifecycle.rs#start_persists_handle_and_status_detects_staleness"
        status: pass
      - kind: other
        ref: "cargo build"
        status: pass
    human_judgment: false
  - id: D2
    description: "CLI rejects path-traversal vm names before joining filesystem paths."
    requirement: "VMGR-01"
    verification:
      - kind: integration
        ref: "tests/cli_security.rs#cli_surfaces_invalid_vm_name_without_panicking"
        status: pass
      - kind: integration
        ref: "tests/cli_security.rs#resolve_vm_name_rejects_and_accepts_expected_cases"
        status: pass
    human_judgment: false
duration: pending
completed: 2026-07-28
status: complete
---

# Phase 8: VM Lifecycle Summary

**CLI start/status tracer wiring now loads host.yaml, starts detached qemu processes, and tracks VM lifecycle state in YAML.**

## Performance

- **Duration:** pending
- **Started:** 2026-07-28T15:11:52+02:00
- **Completed:** 2026-07-28
- **Tasks:** 2
- **Files modified:** 12+

## Accomplishments
- Replaced the demo `main.rs` with a `clap` CLI exposing `start`, `stop`, `kill`, `reset`, and `status` verbs.
- Added lifecycle modules for host config loading, secure state-dir creation, detached child spawning, YAML `VmHandle` persistence, and stale-state cleanup.
- Added the compiled `fake_qemu` stub plus integration tests covering tracer start/status flow and `<vm-name>` path-traversal rejection.

## Task Commits

No commits were created because this repository has a standing zero-auto-commit policy and this execution honored it.

## Files Created/Modified
- `Cargo.toml` - adds `clap`, `serde_json`, and `libc`.
- `src/main.rs` - real CLI entrypoint and error handling.
- `src/lib.rs` - exports the new lifecycle module.
- `src/lifecycle/*.rs` - lifecycle config/process/state/start/status modules and placeholders for later plans.
- `src/bin/fake_qemu.rs` - host-independent qemu stub used by integration tests.
- `tests/vm_lifecycle.rs` - tracer-path integration coverage.
- `tests/cli_security.rs` - traversal-regression and CLI error-path coverage.

## Decisions Made
- Kept later-plan modules present as placeholders so subsequent work can extend the lifecycle tree without touching the module map.
- Used the existing ezkvm YAML pipeline for both `host.yaml` parsing and `VmHandle` persistence.

## Deviations from Plan

- `stop`/`kill`/`reset` are routed through placeholder module functions that still `todo!(...)`; this keeps the CLI wiring stable for Plan 08-03 while leaving current scope limited to the tracer path.

## Issues Encountered

- None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Ready for swtpm readiness polling, QMP command handling, and UI-client launch work on top of the tracer path.
- No STATE.md or ROADMAP.md changes were made here; the orchestrator can update them after all wave agents finish.
