# Epic B Active Backlog

Date: 2026-05-23
Source of truth: `doc/planning/backlog/active/B.md`

## Scope

Proxmox import convergence remaining active work.

## Active Tickets

| ID | Title | Status | Depends On |
|---|---|---|---|
| B-55 | Execute Arch Linux portable-runtime validation matrix and publish runbooks | Todo | B-53, B-54 |

## Ticket Definitions

### B-55 Execute Arch Linux portable-runtime validation matrix and publish runbooks
Status: Todo
Milestone: Phase-2-Hardening
Labels: epic:proxmox, phase:2-hardening
Assignee: unassigned
Dependencies: B-53, B-54

Scope:
- Run the required Phase 3 matrix on Arch Linux as the path-variability stress case.
- Prioritize capability discovery, preflight clarity, and user-mode networking fallback before bridge-helper sign-off.
- Consolidate Debian, Ubuntu, and Arch results into operator runbooks and support guidance.

Acceptance Criteria:
- Arch results include complete captured artifacts for all required scenarios.
- Operator runbooks document distro-specific package/setup differences and expected diagnostics.
- Phase 3 summary explicitly states which capabilities are required, optional, and non-gating.
- Portable-mode support claim for Debian, Ubuntu, and Arch is backed by captured validation evidence.

Estimate: 3 days

Planning Notes:
- Remaining open ticket in Epic B.

## Completion Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/B.md`.