# Epic C Active Backlog

Date: 2026-05-23
Source of truth during transition: `doc/backlog/BACKLOG.md`, `doc/backlog/TRACKING_BOARD.md`

## Scope

Hook system feature work.

## Active Tickets

| ID | Title | Status | Depends On | Notes |
|---|---|---|---|---|
| C-01 | Define hook contract and execution policy | Todo | A-01 | Foundation ticket |
| C-02 | Implement hook runner service | Todo | C-01 | Runtime execution engine |
| C-03 | Add schema support for hook definitions | Todo | C-01 | Config model support |
| C-04 | Wire hooks into start and stop flow | Todo | C-02, C-03 | Lifecycle integration |
| C-05 | Hook tests (unit and integration) | Todo | C-04 | Regression coverage |

## Transition Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/C.md`.