---
name: "Planning And Backlog Sync"
description: "Use when editing planning backlog or feature lifecycle files to keep status, dependencies, and placement synchronized in the same task."
applyTo: "doc/planning/backlog/**,doc/planning/features/ideas/**,doc/planning/features/prepared/**,doc/planning/features/done/**,doc/planning/**/*.md"
---

# Planning And Backlog Sync

When planning files are edited, synchronization is mandatory in the same task.

## Required Sync Check

Before finalizing planning edits:

1. Identify impacted tickets or feature documents.
2. Ensure status, dependencies, and titles are internally consistent for affected entries.
3. Ensure documents are in the correct lifecycle location under `doc/planning/features/{ideas,prepared,done}`.
4. If only one planning file changed, explicitly state why no companion planning file needed updates.

## Lifecycle Placement Rules

- `doc/planning/features/ideas/`: incomplete or exploratory items
- `doc/planning/features/prepared/`: ready for implementation
- `doc/planning/features/done/`: completed implementation records

When status changes, move the feature document to the matching folder in the same task.

## Execution Rule

For planning-impacting tasks, run a final planning-sync pass before the final response:

1. Reconcile impacted ticket/feature status and dependencies.
2. Reconcile file placement according to lifecycle state.
3. Report concise planning deltas and any residual ambiguity.

## Acceptance Criteria

- Affected planning entries are status-consistent.
- Feature documents are in the correct lifecycle folder.
- Final response includes a short planning delta summary.