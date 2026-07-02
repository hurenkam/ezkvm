---
name: "Rust Guideline Enforcement"
description: "Use when editing Rust source or tests to enforce baseline safety, maintainability, validation, and docs impact expectations."
applyTo: "src/**/*.rs,tests/**/*.rs,Cargo.toml"
---

# Rust Guideline Enforcement

Use this guidance for Rust implementation and review tasks.

If the task involves module naming, file boundaries, or helper abstractions, consult [coding-guidelines.md](../../doc/dev/architecture/coding-guidelines.md) first.

When changing code in `src/**`, read the nearest in-scope `README.md` under `src/**` as design and requirements input for that subtree.

- Treat `src/**/README.md` files as manually authored and read-only by default.
- Edit a `src/**/README.md` only when the user explicitly requests edits to that file and confirms.

## Baseline Quality Expectations

- Prefer explicit error handling over panics in production paths.
- Keep functions and modules focused and reasonably small.
- Add or update tests when behavior changes.
- Keep examples and documentation aligned with behavior changes.

## Coding Guideline Conformance Gate

For any task that creates or modifies Rust files, run a coding-guideline conformance check against `doc/dev/architecture/coding-guidelines.md` before finalizing.

Minimum required checks on changed Rust files:

- module/file responsibility still matches a single stage concern
- interfaces remain narrow and explicit
- stage `mod.rs` files keep boundary types/traits/errors and avoid accumulating implementation helpers
- `mod` declarations are restricted to `mod.rs` files (with crate-root `lib.rs`/`main.rs` as the practical exception)
- inline `#[cfg(test)] mod tests { ... }` modules in non-`mod.rs` implementation files are an allowed policy exception
- submodule declarations are private (`mod x;`) and exported surface is defined with `pub use ...`
- no `impl` blocks are added to `mod.rs` files
- `struct` and corresponding `impl` are colocated in the same file whenever practical
- module boundary changes include corresponding test updates

If a mismatch is found:

- either fix it in the same task, or
- explicitly report the remaining mismatch and why it was deferred.

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
3. Coding-guideline conformance status (`pass` or findings with file references).
4. Documentation impact statement (`updated` or `no user-facing doc impact` with reason).

## Scope Note

This helper is intentionally generic and does not encode implementation-specific architecture or runtime policy.