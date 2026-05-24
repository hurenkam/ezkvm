# Project Copilot Instructions

Authoritative standard: follow doc/dev/workflow/coding-guidelines.md for all implementation and review work.

## Feature Document Lifecycle

Feature design documents live under `doc/planning/epics/` and follow a status-based directory convention:

| Directory | Meaning |
|---|---|
| `doc/planning/epics/prepared/` | Design complete; ready to be picked up for implementation |
| `doc/planning/epics/in-progress/` | Actively being implemented |
| `doc/planning/epics/implemented/` | Fully implemented; kept for reference |
| `doc/planning/epics/postponed/` | Deferred; not to be picked up in the near future |

Reference and strategy documents (codebase analysis, phase roadmaps, historical comparisons) that are not feature specs stay in `doc/dev/analysis/`.

When creating a new feature design document, place it directly in the appropriate directory. When status changes:
- Move to `doc/planning/epics/in-progress/` when implementation begins.
- Move to `doc/planning/epics/implemented/` in the same task as marking the related backlog ticket(s) Done.
- Move to `doc/planning/epics/postponed/` when deferring; include a brief note explaining the reason.

## Required Workflow For Code Changes

1. Before editing, consult the relevant sections of doc/dev/workflow/coding-guidelines.md.
2. Keep changes minimal and focused to the requested behavior.
3. Before changing any implicit/default behavior (including config defaults, topology placement defaults, fallback paths, or auto-assigned bus/addr/unit values), ask the user for explicit approval in the current conversation.
4. If a task is urgent and a default-behavior change appears necessary, pause and present the exact proposed default change plus expected runtime impact before editing.
5. After edits, perform documentation impact analysis and update user-facing docs in the same task when behavior/schema/defaults changed.
6. For refactors and code-change tasks with multi-file impact, run the `Docs Sync` agent as a final docs pass before final response.
7. When implementing or completing any backlog ticket (for example `A-01`, `B-38`, `D-02`), update the owning file under `doc/planning/backlog/active/` or `doc/planning/backlog/done/YYYY/` in the same task.
8. When implementing or completing any backlog ticket that has a feature document in `doc/planning/epics/in-progress/`, move that document to `doc/planning/epics/implemented/` in the same task.
9. When editing planning backlog lifecycle files (`doc/planning/backlog/active/**`, `doc/planning/backlog/done/**`, `doc/planning/backlog/future/**`), perform a backlog sync pass in the same task.
10. For backlog-ticket completion or planning-backlog edits with dependency/status impact, run the `Backlog Sync` agent before final response.
11. After edits, perform a guideline audit of changed files and report pass/fail findings.
12. If a guideline is intentionally not met, document a short justification in code comments or review notes.

## Rust Validation Requirements

When Rust code changes, run and report:
- cargo fmt --all --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --quiet

If any command cannot be run, state why and list what was verified instead.

## Validation Lifecycle Requirements

For refactors or behavior-changing Rust tasks, validation is a two-phase gate:

1. **Intake baseline (before edits)**
	- Run and report:
	  - cargo fmt --all --check
	  - cargo clippy --all-targets --all-features -- -D warnings
	  - cargo test --quiet
	- Record whether each command is pass/fail.

2. **Exit validation (after edits)**
	- Re-run the same commands.
	- Report pass/fail for each command.
	- Report the delta: newly failing checks/tests, newly fixed checks/tests, unchanged failing checks/tests.

3. **Completion gate**
	- Do not finalize as complete if exit validation regresses versus intake baseline.
	- If intake already had failures, unresolved failures must be explicitly handled per the unresolved-failure escalation rule.

## Unresolved-Failure Escalation Rule

If a failing check/test appears unrelated to current refactoring work:

1. Do not ignore it or silently defer it.
2. Ask the user whether to fix it in the current task.
3. Do not mark the task complete until one of the following is true:
	- the failure is fixed in the current task, or
	- the user explicitly approves deferral and the failure is documented with a tracking reference.

When deferral is approved, include in the response:
- failing test/check identifier
- exact reproduce command
- suspected first bad commit (if known)
- short rationale for unrelated classification
- user-approved deferral note
- tracking reference (for example the owning row/definition in `doc/planning/backlog/active/*.md` or `doc/planning/backlog/done/YYYY/*.md`)

## Review Output Contract

For review-style requests, present:
1. Findings ordered by severity with file paths.
2. Open questions/assumptions.
3. Brief change summary.

## Scope Notes

- Prefer explicit error handling; avoid unwrap/expect in production paths.
- Keep modules and functions small per thresholds in doc/dev/workflow/coding-guidelines.md.
- Keep mod.rs files as module wiring only (no struct/fn/impl bodies).
- For config serialization, follow compact-by-default output: omit fields at semantic defaults unless explicit values are needed for merge/override behavior.

## Customization Registry

### Agents

- `Docs Sync`: use for final documentation synchronization pass on multi-file refactors and behavior changes.
- `Backlog Sync`: use when backlog/tracking board status, dependencies, or registry metadata change.
- `Cross-Distro Deb Reviewer`: use when reviewing packaging changes for one-package compatibility across Debian Trixie and Ubuntu 26.04.

### Skills

- `cross-distro-deb-packaging`: use when creating or updating ezkvm Debian packaging intended to run on both Debian Trixie and Ubuntu 26.04 from one binary package per architecture.
- `debian-trixie`: use for Debian Trixie packaging and architecture-specific distribution guidance.
- `ubuntu-resolute`: use for Ubuntu 26.04 host/runtime/path guidance and packaging validation.
- `q35-topology-review`: use when reviewing or planning Q35 PCIe and legacy PCI hierarchy, bridge topology, hotplug paths, or imported slot stability.

### Packaging Guard

- For packaging-related edits, apply `.github/instructions/cross-distro-deb-package-guard.instructions.md`.
- Treat dual-distro validation status (`pass`, `partial`, `not run`) as required final-response output for packaging changes.

### Input Reference Data Guard

- For changes to `src/**/*.rs`, `input/**/etc/**`, or `etc/**/*.yaml`, apply `.github/instructions/input-reference-data.instructions.md`.
- Immutable files (`input/**/*.conf`, `input/**/*.qemu.cmd`, `input/**/*.swtpm.cmd`) must never be modified.
- Derived files (`input/**/*.yaml`, `input/**/*.ezkvm.qemu.cmd`, `input/**/*.ezkvm.swtpm.cmd`) may need regeneration; **always ask the user before regenerating**.
