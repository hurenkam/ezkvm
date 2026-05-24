---
name: "Backlog And Planning Sync"
description: "Use when editing planning backlog lifecycle files or epic feature docs, or when completing a backlog ticket (for example A-01/B-38), to require same-task synchronization of ticket status and dependencies in planning documents."
applyTo: "doc/planning/backlog/active/**, doc/planning/backlog/done/**, doc/planning/backlog/future/**, doc/planning/epics/prepared/**, doc/planning/epics/in-progress/**, doc/planning/epics/implemented/**, doc/planning/epics/postponed/**"
---

# Backlog And Tracking Sync

When planning backlog files are edited, or when implementation work completes a backlog ticket, synchronization is mandatory in the same task.

## Required Sync Check

Before finalizing changes:

1. Identify affected ticket IDs and owning epic files under `doc/planning/backlog/active/` or `doc/planning/backlog/done/YYYY/`.
2. Ensure ticket title, dependencies, and status are aligned between the summary table and ticket definitions in the owning file.
3. Update future buckets when uncommitted tickets are added, removed, renamed, or re-scoped.
4. If only one planning file needed changes, explicitly state why no companion file required update.

## Completion Trigger Rule

When a task implements or completes a backlog ticket ID:

1. Update the corresponding ticket status in its owning planning backlog file in the same task.
2. Move ticket definitions between `active/` and `done/YYYY/` when status crosses completion boundaries.
3. Run a backlog-sync pass even if only code files were edited.

## Minimum Alignment Rules

- Every active backlog ticket should appear in `doc/planning/backlog/active/` under its owning epic file.
- Completed tickets should appear in `doc/planning/backlog/done/YYYY/` under the owning epic file.
- Dependencies and status must be consistent between summary tables and ticket definitions for affected tickets.
- `future/` buckets should not contain committed active or completed work.

## Execution Rule

For edits affecting either file, run a final backlog-sync pass before the final response:

- identify changed or impacted ticket IDs
- reconcile registry rows
- reconcile dependency graph edges
- report a concise backlog-sync delta summary

Use the `Backlog Sync` custom agent when the change touches multiple tickets or when planning backlog files differ.

## Feature Document Sync

Feature design documents in `doc/planning/epics/` subdirectories must stay aligned with ticket status:

| Directory | Expected ticket status |
|---|---|
| `doc/planning/epics/prepared/` | Todo or Ready (not yet started) |
| `doc/planning/epics/in-progress/` | In Progress |
| `doc/planning/epics/implemented/` | Done |
| `doc/planning/epics/postponed/` | Blocked or Postponed |

When completing a backlog ticket that has a feature document in `doc/planning/epics/in-progress/`, move that document to `doc/planning/epics/implemented/` in the same task.

When starting work on a ticket that has a feature document in `doc/planning/epics/prepared/`, move that document to `doc/planning/epics/in-progress/` in the same task.

When deferring a ticket, move its feature document to `doc/planning/epics/postponed/` and add a brief deferral note to the document.