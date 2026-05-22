---
description: "Use when editing Rust code, tests, config parsing, or refactors to enforce doc/dev/CODING_GUIDELINES.md thresholds and review checklist."
name: "Rust Guideline Enforcement"
applyTo: "src/**/*.rs, tests/**/*.rs"
---

# Rust Guideline Enforcement

Always validate changed Rust files against doc/dev/CODING_GUIDELINES.md.

## Mandatory Checks On Relevant Changes

- No unwrap/expect in production paths.
- Function length target under 35 lines when practical, otherwise justify.
- File length target under 250 lines when practical, otherwise justify.
- Keep mod.rs files as module wiring only.
- Add/update tests when behavior changes.
- Keep docs/examples aligned with behavior changes.
- For config serialization changes, enforce compact-by-default output and verify omission-equivalent behavior with roundtrip and merge/override tests.

## Execution Expectations

For implementation tasks that touch Rust files:
- Run format, clippy, and tests at intake and exit for refactors/behavior changes.
- Report what was run and the result.
- If a check is skipped, explain why and provide a safe alternative verification.

Validation commands:
- cargo fmt --all --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --quiet

Required reporting format:
- Intake: pass/fail per command
- Exit: pass/fail per command
- Delta: newly failing, newly fixed, unchanged failing

Completion rule:
- Do not finalize as complete when exit validation regresses versus intake baseline.

If unresolved failures remain (including failures believed unrelated), include an explicit deferral block:
- Identifier: <failing test/check>
- Reproduce: <exact command>
- Suspected first bad commit: <sha or unknown>
- Unrelated rationale: <one line>
- User decision: <fix now | defer>
- Tracking reference: <ticket or board row>

## Review Expectations

When reviewing or summarizing changes, include explicit guideline compliance notes for:
- error handling
- maintainability/size thresholds
- tests and docs impact
