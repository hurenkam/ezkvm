# Epic C Active Backlog

Date: 2026-05-23
Source of truth: `doc/planning/backlog/active/C.md`

## Scope

Hook system feature work.

## Active Tickets

| ID | Title | Status | Depends On |
|---|---|---|---|
| C-01 | Define hook contract and execution policy | Todo | A-01 |
| C-02 | Implement hook runner service | Todo | C-01 |
| C-03 | Add schema support for hook definitions | Todo | C-01 |
| C-04 | Wire hooks into start and stop flow | Todo | C-02, C-03 |
| C-05 | Hook tests (unit and integration) | Todo | C-04 |

## Ticket Definitions

### C-01 Define hook contract and execution policy
Status: Todo
Milestone: Phase-1-Features
Labels: epic:hooks, phase:1-features
Assignee: unassigned
Dependencies: A-01

Scope:
- Hook points: `pre_start`, `post_start`, `pre_stop`, `post_stop`.
- Define timeout, retries (if any), fail-open/fail-closed behavior.

Acceptance Criteria:
- Contract documented and approved in ADR.
- Logging and error surface format agreed.

Estimate: 2 days

Planning Notes:
- Foundation ticket.

### C-02 Implement hook runner service
Status: Todo
Milestone: Phase-1-Features
Labels: epic:hooks, phase:1-features
Assignee: unassigned
Dependencies: C-01

Scope:
- Central runner in runtime layer with structured events.

Acceptance Criteria:
- Supports sync execution and timeout enforcement.
- Logs include hook name, VM name, duration, outcome.

Estimate: 4 days

Planning Notes:
- Runtime execution engine.

### C-03 Add schema support for hook definitions
Status: Todo
Milestone: Phase-1-Features
Labels: epic:hooks, phase:1-features
Assignee: unassigned
Dependencies: C-01

Scope:
- Add hook config under canonical options path (or agreed path).

Acceptance Criteria:
- YAML parsing supports hook definitions.
- Invalid hook definitions rejected with actionable errors.

Estimate: 2 days

Planning Notes:
- Config model support.

### C-04 Wire hooks into start and stop flow
Status: Todo
Milestone: Phase-1-Features
Labels: epic:hooks, phase:1-features
Assignee: unassigned
Dependencies: C-02, C-03

Scope:
- Integrate runner into lifecycle orchestration in runtime start/stop handlers.

Acceptance Criteria:
- Hooks execute in correct order and policy.
- Dry-run behavior explicitly defined and tested.

Estimate: 3 days

Planning Notes:
- Lifecycle integration.

### C-05 Hook tests (unit and integration)
Status: Todo
Milestone: Phase-1-Features
Labels: epic:hooks, phase:1-features
Assignee: unassigned
Dependencies: C-04

Scope:
- Success, timeout, failure, and policy behavior tests.

Acceptance Criteria:
- Coverage includes all hook stages and failure modes.

Estimate: 3 days

Planning Notes:
- Regression coverage.

## Completion Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/C.md`.