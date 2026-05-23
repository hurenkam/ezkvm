# Epic D Active Backlog

Date: 2026-05-23
Source of truth: `doc/planning/backlog/active/D.md`

## Scope

Extensibility implementation remaining work.

## Active Tickets

| ID | Title | Status | Depends On |
|---|---|---|---|
| D-02 | Implement compile-time extension registry | Todo | D-01 |
| D-03 | Port one concrete extension from v1 patterns | Todo | D-02 |
| D-04 | Extensibility docs and examples | Todo | D-03 |

## Ticket Definitions

### D-02 Implement compile-time extension registry
Status: Todo
Milestone: Phase-1-Features
Labels: epic:extensibility, phase:1-features
Assignee: unassigned
Dependencies: D-01

Scope:
- Register extension implementations without runtime plugin loading.

Acceptance Criteria:
- Default registry compiles with no behavior change when unused.
- Extension execution path covered by tests.

Estimate: 3 days

Planning Notes:
- Registry implementation.

### D-03 Port one concrete extension from v1 patterns
Status: Todo
Milestone: Phase-1-Features
Labels: epic:extensibility, phase:1-features
Assignee: unassigned
Dependencies: D-02

Scope:
- Use a real extension use-case (for example Proxmox mapping strategy variant).

Acceptance Criteria:
- Demonstrates extension lifecycle and fallback behavior.

Estimate: 2 days

Planning Notes:
- First migration sample.

### D-04 Extensibility docs and examples
Status: Todo
Milestone: Phase-1-Features
Labels: epic:extensibility, phase:1-features
Assignee: unassigned
Dependencies: D-03

Scope:
- Add developer guide: how to add an extension safely.

Acceptance Criteria:
- Includes sample implementation and test template.

Estimate: 1 day

Planning Notes:
- Documentation and examples.

## Completion Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/D.md`.