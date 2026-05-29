---
name: "Docs Sync Enforcement"
description: "Use when implementation or configuration changes may alter user-visible behavior, defaults, precedence, schema, or examples. Require documentation impact analysis and same-task doc updates."
applyTo: "src/**,tests/**,etc/**,examples/**,doc/**/*.md,README.md"
---

# Docs Sync Enforcement

When behavior changes, documentation synchronization is mandatory in the same task.

## Required Documentation Impact Check

Before finalizing implementation changes:

1. Determine whether user-visible behavior, schema, flags, defaults, precedence, runtime paths, or examples changed.
2. If yes, identify impacted documentation and update it in the same task.
3. If no docs are needed, state "no user-facing doc impact" with a one-line reason.
4. If the task resolves recurring operator failures, add or update a concise troubleshooting entry in user docs.

## Documentation Targets

Use `doc/README.md` as placement authority, then update relevant targets:

- `doc/user/**` for end-user behavior and operations
- `doc/dev/**` for development-facing behavior, architecture, and constraints
- `README.md` for onboarding, usage, and command examples
- `doc/planning/**` only when planning intent or scope changed as part of the request

## Execution Rule

For multi-file implementation tasks, perform a final docs-sync pass before the final response:

1. Map changed files and behavior deltas to likely impacted docs.
2. Apply minimal documentation updates.
3. Report concise documentation deltas and residual risks.

## Acceptance Criteria

- Behavior-changing implementation updates and docs updates occur in the same task.
- Documentation reflects current behavior and examples.
- Final response includes either:
  - updated documentation file list, or
  - explicit no-impact statement with reason.