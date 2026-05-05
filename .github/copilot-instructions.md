# Project Copilot Instructions

Authoritative standard: follow doc/dev/CODING_GUIDELINES.md for all implementation and review work.

## Required Workflow For Code Changes

1. Before editing, consult the relevant sections of doc/dev/CODING_GUIDELINES.md.
2. Keep changes minimal and focused to the requested behavior.
3. After edits, perform documentation impact analysis and update user-facing docs in the same task when behavior/schema/defaults changed.
4. For refactors and code-change tasks with multi-file impact, run the `Docs Sync` agent as a final docs pass before final response.
5. When implementing or completing any backlog ticket (for example `A-01`, `B-38`, `D-02`), update `doc/backlog/TRACKING_BOARD.md` status in the same task even if no backlog document was explicitly requested.
6. When editing `doc/backlog/BACKLOG.md` or `doc/backlog/TRACKING_BOARD.md`, perform a backlog/tracking sync pass in the same task.
7. For backlog-ticket completion or backlog/tracking-board edits with dependency/status/registry impact, run the `Backlog Sync` agent before final response.
8. After edits, perform a guideline audit of changed files and report pass/fail findings.
9. If a guideline is intentionally not met, document a short justification in code comments or review notes.

## Rust Validation Requirements

When Rust code changes, run and report:
- cargo fmt --all --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --quiet

If any command cannot be run, state why and list what was verified instead.

## Review Output Contract

For review-style requests, present:
1. Findings ordered by severity with file paths.
2. Open questions/assumptions.
3. Brief change summary.

## Scope Notes

- Prefer explicit error handling; avoid unwrap/expect in production paths.
- Keep modules and functions small per thresholds in doc/dev/CODING_GUIDELINES.md.
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
