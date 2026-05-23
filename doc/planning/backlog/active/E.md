# Epic E Active Backlog

Date: 2026-05-23
Source of truth: `doc/planning/backlog/active/E.md`

## Scope

Hardening and lifecycle stability work.

## Active Tickets

| ID | Title | Status | Depends On |
|---|---|---|---|
| E-01 | Regression suite expansion | Todo | B-05, C-05, D-03 |
| E-02 | Performance and stability checks | Todo | E-01 |
| E-03 | Beta release gating | Todo | E-01, E-02 |
| E-04 | Promote shutdown monitor into a VM-scoped lifecycle supervisor | In Progress | None |
| E-05 | Reconcile guest shutdown state with QMP and PID state | In Progress | E-04 |
| E-06 | Add shutdown lifecycle regression tests and traces | Todo | E-05 |

## Ticket Definitions

### E-01 Regression suite expansion
Status: Todo
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: B-05, C-05, D-03

Scope:
- Extend integration tests around profiles + import + hooks.

Acceptance Criteria:
- CI has deterministic, parallel-safe test execution.
- Snapshot updates are intentional and reviewed.

Estimate: 3 days

Planning Notes:
- Baseline hardening coverage.

### E-02 Performance and stability checks
Status: Todo
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: E-01

Scope:
- Measure startup overhead from hooks and import path.

Acceptance Criteria:
- Baseline metrics recorded.
- Overhead thresholds documented and met.

Estimate: 2 days

Planning Notes:
- Runtime quality validation.

### E-03 Beta release gating
Status: Todo
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: E-01, E-02

Scope:
- Feature flags and release notes.

Acceptance Criteria:
- Feature flags documented.
- Rollback plan validated.

Estimate: 1 day

Planning Notes:
- Release readiness gate.

### E-04 Promote shutdown monitor into a VM-scoped lifecycle supervisor
Status: In Progress
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: None

Scope:
- Turn the detached shutdown monitor into the per-VM owner for QMP shutdown, query-status reconciliation, and cleanup until the VM exits.
- Keep the lifecycle path shared between interactive and daemon starts.

Acceptance Criteria:
- A VM-scoped companion process owns shutdown observation until exit.
- QMP SHUTDOWN/POWERDOWN and process-exit transitions are handled in one place.

Estimate: 3 days

Planning Notes:
- Lifecycle supervision active work.

### E-05 Reconcile guest shutdown state with QMP and PID state
Status: In Progress
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: E-04

Scope:
- Distinguish "guest UI is gone" from "QEMU process still running" in status and stop flows.
- Surface QMP query-status, PID state, and shutdown reason in one output path.

Acceptance Criteria:
- `ezkvm status` can report QMP running versus shutdown instead of only process liveness.
- Tests cover running, shutdown, and socket-missing states.

Estimate: 2 days

Planning Notes:
- State-model alignment.

### E-06 Add shutdown lifecycle regression tests and traces
Status: Todo
Milestone: Phase-2-Hardening
Labels: epic:hardening, phase:2-hardening
Assignee: unassigned
Dependencies: E-05

Scope:
- Add tests and fixtures for shutdown event handling, delayed QMP shutdown, and companion-process exit.
- Capture logging or traces for shutdown stalls so the guest-versus-QEMU state gap is visible.

Acceptance Criteria:
- Tests cover both SHUTDOWN event and timeout/query-status fallback.
- Logging shows the transition from guest shutdown request to QEMU exit or stall.

Estimate: 2 days

Planning Notes:
- Trace and regression coverage.

## Completion Notes

- Move completed tickets to `doc/planning/backlog/done/YYYY/E.md`.