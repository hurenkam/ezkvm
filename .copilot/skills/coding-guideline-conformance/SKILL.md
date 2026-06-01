---
name: "coding-guideline-conformance"
description: "Conformance gate for changed Rust files against doc/dev/architecture/coding-guidelines.md"
domain: "project-conventions"
confidence: "high"
source: "manual"
---

## Context

Use this skill whenever a task adds or modifies Rust files.

Primary source of truth:
- `doc/dev/architecture/coding-guidelines.md`

This skill is a pre-finalization gate: it verifies that changed code still conforms to project coding boundaries and review expectations.

## Required Inputs

- Changed Rust file list (from current task scope or git diff)
- `doc/dev/architecture/coding-guidelines.md`

## Conformance Checklist

For each changed Rust file:

1. **Responsibility check**
- File/module has one clear stage responsibility.
- Parsing, validation, and transformation logic are not mixed in one file unless explicitly intended.

2. **Interface check**
- Interfaces are explicit and narrow.
- Visibility is minimized (`pub(crate)`/private where possible).

3. **Stage module structure check**
- Stage boundaries in `mod.rs` prioritize shared boundary types/traits/errors.
- Implementation helpers live in dedicated implementation/helper modules.

4. **Scope/size check**
- File remains reviewable in one pass (guideline target around 350 lines).
- If growth exceeds this, justify or split.

5. **Test alignment check**
- Behavior or module-boundary changes include test updates.

6. **Docs alignment check**
- If behavior changed, docs are synchronized per docs-sync rules.

## Output Format

When reporting results:

- `Conformance: pass` or `Conformance: findings`
- If findings exist: list each finding with file path and concrete fix
- Note any accepted deferred mismatches with rationale

## Anti-Patterns

- Skipping conformance review because lint/tests passed
- Treating `mod.rs` as a catch-all implementation file
- Leaving widened visibility without reason
- Making boundary refactors without corresponding tests
