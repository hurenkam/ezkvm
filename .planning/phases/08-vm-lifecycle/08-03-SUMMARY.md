---
phase: 08-vm-lifecycle
plan: 03
subsystem: lifecycle
status: complete
completed: 2026-07-28
requirements_completed: [VMGR-03, VMGR-04, VMGR-05]
files_modified:
  - src/lifecycle/qmp.rs
  - src/lifecycle/stop.rs
  - src/lifecycle/kill.rs
  - src/lifecycle/reset.rs
  - src/main.rs
  - tests/qmp_client.rs
---

# Phase 8 Plan 08-03 Summary

Implemented QMP lifecycle control for `stop`, `kill`, and `reset`.

## What changed
- Added `QmpClient` with greeting handling, mandatory `qmp_capabilities` negotiation, typed command errors, EOF-tolerant `quit`, and fire-and-forget `system_reset`.
- Implemented `stop` with default indefinite wait, optional host-config/CLI escalation timeout, stale-handle rejection, UI-client cleanup, and state-file removal on success.
- Implemented `kill` with QMP `quit`, bounded wait, force-kill fallback, UI-client cleanup, and state-file removal.
- Implemented `reset` as QMP `system_reset` without waiting for a response or mutating VM state.
- Wired `src/main.rs` to call the real lifecycle verbs and added `stop --escalate-after <secs>` override support.
- Added `tests/qmp_client.rs` covering QMP handshake ordering, missing greeting, QMP command errors, EOF-on-quit tolerance, fire-and-forget reset, stop/kill/reset integration against `fake_qemu`, UI-client cleanup, and opt-in stop escalation.

## Verification
- `cargo build`
- `cargo test --test qmp_client -- --test-threads=1`

## Notes
- No `git add` or `git commit` commands were run.
- `STATE.md` and `ROADMAP.md` were not modified.
- Work stayed within the plan’s listed files.
