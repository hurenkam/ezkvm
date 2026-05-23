# Epic E Active Backlog

Date: 2026-05-23
Source of truth during transition: `doc/backlog/BACKLOG.md`, `doc/backlog/TRACKING_BOARD.md`

## Scope

Hardening and lifecycle stability work.

## Active Tickets

| ID | Title | Status | Depends On | Notes |
|---|---|---|---|---|
| E-01 | Regression suite expansion | Todo | B-05, C-05, D-03 | Baseline hardening coverage |
| E-02 | Performance and stability checks | Todo | E-01 | Runtime quality validation |
| E-03 | Beta release gating | Todo | E-01, E-02 | Release readiness gate |
| E-04 | Promote shutdown monitor into a VM-scoped lifecycle supervisor | In Progress | None | Lifecycle supervision active work |
| E-05 | Reconcile guest shutdown state with QMP and PID state | In Progress | E-04 | State-model alignment |
| E-06 | Add shutdown lifecycle regression tests and traces | Todo | E-05 | Trace and regression coverage |

## Transition Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/E.md`.