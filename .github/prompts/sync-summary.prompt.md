---
name: Sync Summary
description: "Run a docs/planning sync pass and return a concise standardized summary for updates, no-impact cases, and residual risks."
argument-hint: "Which sync type should run (docs, planning, or both), and what changed?"
---

Produce a standardized synchronization summary using one or both of these agents:

- `Docs Sync`
- `Planning Sync`

## Inputs

- `sync_type`: `docs`, `planning`, or `both`
- `change_scope`: files changed and behavior/status deltas

## Usage Examples

- `sync_type=docs; change_scope=Updated config precedence and examples in src and etc files.`
- `sync_type=planning; change_scope=Moved feature from prepared to done and updated dependencies.`
- `sync_type=both; change_scope=Implemented feature, updated user behavior docs, and completed planning item.`

## Execution

1. If `sync_type=docs`, run `Docs Sync` for the provided `change_scope`.
2. If `sync_type=planning`, run `Planning Sync` for the provided `change_scope`.
3. If `sync_type=both`, run both passes and merge results.
4. Keep edits minimal and aligned with `doc/README.md` structure.

## Required Output Format

### Sync Type
- docs | planning | both

### Updated Files
- List updated files, or `none`

### Key Deltas
- Concise bullets of mismatches resolved

### No-Impact Statements
- Include explicit `no user-facing doc impact` and/or `no planning sync impact` when applicable, each with one-line reason

### Residual Risks / Open Questions
- List remaining ambiguity or follow-up actions, or `none`