# Epic K Active Backlog

Date: 2026-05-23
Source of truth during transition: `doc/backlog/BACKLOG.md`, `doc/backlog/TRACKING_BOARD.md`

## Scope

Cross-distro packaging hardening and validation.

## Active Tickets

| ID | Title | Status | Depends On | Notes |
|---|---|---|---|---|
| K-03 | Harden dependency policy for cross-distro installability | Todo | K-02 | Dependency contract hardening |
| K-04 | Execute dual-distro package validation matrix | Todo | K-03 | Debian + Ubuntu validation |
| K-05 | Add packaging CI and release gate enforcement | Todo | K-04 | Automation and release guardrails |
| K-06 | Package non-root runtime group and directory ownership policy | Todo | K-03 | Runtime ownership policy |

## Transition Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/K.md`.