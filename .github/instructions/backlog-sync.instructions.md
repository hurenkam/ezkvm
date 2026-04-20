---
name: "Backlog And Tracking Sync"
description: "Use when editing doc/backlog/BACKLOG.md or doc/backlog/TRACKING_BOARD.md, or when completing a backlog ticket (for example A-01/B-38), to require same-task synchronization of ticket registry, dependency graph, and status between the planning documents."
applyTo:
  - "doc/backlog/BACKLOG.md"
  - "doc/backlog/TRACKING_BOARD.md"
---

# Backlog And Tracking Sync

When either backlog planning document is edited, or when implementation work completes a backlog ticket, synchronization is mandatory in the same task.

## Required Sync Check

Before finalizing changes:

1. Compare `doc/backlog/BACKLOG.md` and `doc/backlog/TRACKING_BOARD.md` for affected ticket IDs.
2. Ensure ticket presence, title, dependencies, and coarse status are aligned.
3. Update the tracking board registry and dependency graph when backlog items are added, removed, renamed, completed, or re-sequenced.
4. If only one file needed changes, explicitly state why the counterpart file required no update.

## Completion Trigger Rule

When a task implements or completes a backlog ticket ID:

1. Mark the corresponding row in `doc/backlog/TRACKING_BOARD.md` as `Done` (or the requested status) in the same task.
2. Update the board date when status changed.
3. Run a backlog-sync pass even if only code files were edited.

## Minimum Alignment Rules

- Every active backlog ticket should appear in the tracking board registry.
- Tracking board dependency edges must match backlog dependency declarations for affected tickets.
- Completed implementation work should update tracking status in the same task when the task maps to a backlog item.
- Registry headings and scope notes must not claim outdated ticket ranges.

## Execution Rule

For edits affecting either file, run a final backlog-sync pass before the final response:

- identify changed or impacted ticket IDs
- reconcile registry rows
- reconcile dependency graph edges
- report a concise backlog-sync delta summary

Use the `Backlog Sync` custom agent when the change touches multiple tickets or when backlog and tracking board differ.