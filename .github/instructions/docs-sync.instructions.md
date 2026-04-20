---
name: "Documentation Sync Enforcement"
description: "Use when refactoring or changing behavior in code/config paths to require documentation impact analysis and same-task documentation updates in doc/user, doc/CONFIGURATION.md, or README.md."
applyTo:
  - "src/**/*.rs"
  - "tests/**/*.rs"
  - "etc/**/*.yaml"
  - "examples/**/*.yaml"
  - "Cargo.toml"
---

# Documentation Sync Enforcement

For behavior-changing edits in matched files, documentation synchronization is mandatory in the same task.

## Required Documentation Impact Check

Before finalizing code changes:

1. Determine whether user-visible behavior, schema, flags, defaults, precedence, runtime paths, or examples changed.
2. If yes, identify impacted docs and update them in the same task.
3. If no docs are needed, state "no user-facing doc impact" and provide a one-line reason in the final summary.

## Documentation Targets

Check and update relevant docs from this list:

- `doc/user/config/*.md` for end-user configuration behavior
- `doc/CONFIGURATION.md` for schema/defaults/precedence changes
- `README.md` for CLI usage, commands, and examples
- `doc/preparation/*.md` or `doc/backlog/*.md` when architecture/backlog intent changes are part of the request

## Execution Rule

For refactors and code-change tasks, run a final docs-sync pass before the final response:

- map changed files to likely impacted documentation
- apply required edits
- report a concise doc delta summary (files and key updates)

Use the `Docs Sync` custom agent when the change spans multiple modules or docs.

## Acceptance Criteria

- Code/config changes and documentation updates are in the same task/PR.
- Documentation reflects current behavior and compatibility notes.
- Examples are valid for the current schema.
- Final response includes either:
  - updated documentation file list, or
  - explicit no-impact statement with reason.
