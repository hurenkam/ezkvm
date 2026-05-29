---
name: "Rust Guideline Enforcement"
description: "Use when editing Rust source or tests to enforce baseline safety, maintainability, validation, and docs impact expectations."
applyTo: "src/**/*.rs,tests/**/*.rs,Cargo.toml"
---

# Rust Guideline Enforcement

Use this guidance for Rust implementation and review tasks.

## Baseline Quality Expectations

- Prefer explicit error handling over panics in production paths.
- Keep functions and modules focused and reasonably small.
- Add or update tests when behavior changes.
- Keep examples and documentation aligned with behavior changes.

## Validation Expectations

For Rust code changes, run and report these commands when feasible:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --quiet`

If a command cannot be run, explicitly state why and what was verified instead.

## Reporting Expectations

When summarizing Rust changes, include:

1. What changed and why.
2. Validation performed and results.
3. Documentation impact statement (`updated` or `no user-facing doc impact` with reason).

## Scope Note

This helper is intentionally generic and does not encode implementation-specific architecture or runtime policy.