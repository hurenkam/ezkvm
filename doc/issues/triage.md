# Triage Workflow

## Purpose

Define how reported problems and feature requests are classified, prioritized, and linked into planning.

## Triage Steps

1. Confirm issue type:
   - problem/bug
   - feature request
   - docs/process issue
2. Check for duplicates or known issues.
3. Assess severity and priority.
4. Assign an owner or owning area.
5. Link to an existing backlog ticket or create/route a new planning item.

## Severity Model

| Severity | Meaning |
|---|---|
| S0 | Blocks core usage, causes data loss, or prevents install/start for primary workflows |
| S1 | Major feature broken or severe regression with workaround unavailable or unacceptable |
| S2 | Important defect with workaround available or limited blast radius |
| S3 | Minor defect, edge case, or documentation issue |

## Priority Guidance

| Priority | Meaning |
|---|---|
| P0 | Must be addressed immediately |
| P1 | Should be scheduled in the next active cycle |
| P2 | Valuable but not urgent |
| P3 | Nice-to-have / deferred |

## Ownership Areas

- `user-docs`: user/operator documentation issues
- `dev-docs`: architecture/workflow/domain/analysis docs
- `planning`: backlog, issue flow, or tracking problems
- `proxmox-import`: importer/parity issues
- `qemu-cmd-import`: qemu-cmd importer issues
- `runtime`: runtime/process/device lifecycle issues
- `packaging`: distro packaging/installability issues

## Backlog Linkage

- Confirmed implementation work should map to a backlog ticket in the owning file under `doc/planning/backlog/active/` or `doc/planning/backlog/done/YYYY/`.
- Future/uncommitted requests should be routed to the appropriate `doc/planning/backlog/future/` bucket.
- Known recurring issues should also be listed in `doc/issues/known-issues.md`.