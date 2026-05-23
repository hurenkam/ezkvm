# Epic K Active Backlog

Date: 2026-05-23
Source of truth: `doc/planning/backlog/active/K.md`

## Scope

Cross-distro packaging hardening and validation.

## Active Tickets

| ID | Title | Status | Depends On |
|---|---|---|---|
| K-03 | Harden dependency policy for cross-distro installability | Todo | K-02 |
| K-04 | Execute dual-distro package validation matrix | Todo | K-03 |
| K-05 | Add packaging CI and release gate enforcement | Todo | K-04 |
| K-06 | Package non-root runtime group and directory ownership policy | Todo | K-03 |

## Ticket Definitions

### K-03 Harden dependency policy for cross-distro installability
Status: Todo
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening, ready
Assignee: unassigned
Dependencies: K-02

Scope:
- Keep hard `Depends` on shared Debian/Ubuntu baseline only.
- Move optional integrations to `Recommends` where practical.
- Use alternative dependency expressions for naming differences between distros.

Acceptance Criteria:
- Package dependencies resolve on Debian Trixie and Ubuntu Resolute without distro-specific package forks.
- Any unavoidable deltas are documented with rationale.
- Dependency policy is documented for maintainers.

Estimate: 2 days

Planning Notes:
- Dependency contract hardening.

### K-04 Execute dual-distro package validation matrix
Status: Todo
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening
Assignee: unassigned
Dependencies: K-03

Scope:
- Validate build, lint, install, dependency resolution, runtime readiness, and conffile preservation on Debian Trixie and Ubuntu Resolute.
- Capture repeatable validation evidence and operator notes.

Acceptance Criteria:
- Validation matrix completed for both distros with captured artifacts.
- `lintian` and install/runtime checks pass for required scenarios.
- Required runtime readiness checks pass with packaged defaults.

Estimate: 2 days

Planning Notes:
- Debian + Ubuntu validation.

### K-05 Add packaging CI and release gate enforcement
Status: Todo
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening
Assignee: unassigned
Dependencies: K-04

Scope:
- Add CI checks for package build/lint and install smoke validation.
- Add release gating requiring dual-distro packaging evidence before merge/release.

Acceptance Criteria:
- CI enforces packaging checks on relevant changes.
- Release checklist includes dual-distro packaging gate.
- Packaging regressions are blocked before release.

Estimate: 2 days

Planning Notes:
- Automation and release guardrails.

### K-06 Package non-root runtime group and directory ownership policy
Status: Todo
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening
Assignee: unassigned
Dependencies: K-03

Scope:
- During package installation, create system group `ezkvm`.
- Ensure ezkvm runtime directories are created/maintained with `0775` permissions and group ownership `ezkvm`.
- Update user documentation to state that user-mode execution requires membership in group `ezkvm`.
- Optional: add udev automation to assign group `ezkvm` and mode `0770` for `/dev/mapper/*-vm--*` LVM device nodes.

Acceptance Criteria:
- Package install/upgrade idempotently ensures `ezkvm` group exists and runtime directories have the expected permissions and group ownership.
- Non-root user-mode runtime paths (sockets, pid files, logs) are writable when the user is in group `ezkvm`.
- User-facing docs include explicit steps to add a user to group `ezkvm` and verify effective membership.
- Optional udev automation is either implemented and documented, or explicitly deferred with rationale.

Estimate: 2 days

Planning Notes:
- Runtime ownership policy.

## Completion Notes

- Keep this file limited to `Todo`, `In Progress`, and `In Review` work.
- Move completed tickets to `doc/planning/backlog/done/YYYY/K.md`.