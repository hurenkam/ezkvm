# Contributing (Developer Workflow)

This document defines the minimum workflow for architecture-safe contributions.

## Required Reading

1. `doc/dev/CODING_GUIDELINES.md`
2. `doc/dev/ARCHITECTURE_GUIDELINES.md`
3. `doc/dev/MODULE_OWNERSHIP.md`
4. Relevant ADRs in `doc/dev/adr/`
5. `doc/dev/EXTENSIBILITY_SEAMS.md` when touching import or runtime extension boundaries

## Standard Workflow

1. Open or link a tracked task.
2. Confirm layer ownership impact before coding.
3. Implement minimal, focused change.
4. Add/update tests for behavior changes.
5. Run quality gates:
   - `cargo fmt --all --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --quiet`
6. Update docs when user-facing or architecture behavior changes.

## Layer-Boundary Review Checklist

Use this checklist in PR review for architecture-affecting changes.

1. Dependency direction respected: `CLI -> Config -> QEMU -> Runtime -> OS`
2. No circular module dependencies introduced.
3. `cli/` changes do not absorb schema or process execution internals.
4. `config/` changes keep validation/merge/schema responsibilities local.
5. `qemu/` changes keep deterministic command-building concerns local.
6. `runtime/` orchestration remains coordination-only (no schema rewrites).
7. `state/` remains metadata storage/access only.
8. `import/` outputs canonical schema only (no alternate runtime schema).
9. Anti-patterns avoided: cross-layer callbacks, mutable singletons, hidden side effects.
10. At least one integration or regression test covers the changed layer path.

## Documentation Expectations

1. Link new architecture decisions in `doc/dev/adr/`.
2. Update `doc/dev/MODULE_OWNERSHIP.md` if ownership boundaries change.
3. Update user docs if commands/behavior change.
