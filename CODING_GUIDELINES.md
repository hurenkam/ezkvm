# Rust Coding Guidelines

## 1. Core Principles

- Keep behavior correct first, then optimize.
- Prefer readability and maintainability over cleverness.
- Keep changes small, testable, and easy to review.
- Avoid hidden side effects and implicit global state.

## 2. Project Structure

- Keep modules focused on one responsibility.
- Split large files when they become hard to navigate.
- Prefer small helper functions over long monolithic functions.
- Place tests close to the code they verify when practical.

Recommended thresholds:
- Struct length target: under 35 lines when possible.
- Function length target: under 35 lines when possible.
- File length target: under 250 lines when possible.
- Public function complexity: prefer simple control flow and explicit branches.

## 3. Naming and API Design

- Use descriptive names for types, functions, and variables.
- Name booleans as predicates (for example is_enabled, has_profile).
- Keep public APIs stable and explicit.
- Minimize public surface area unless external usage requires it.

## 4. Error Handling

- Never use unwrap or expect in production paths.
- Use anyhow for application-level error propagation.
- Use typed errors (for example thiserror) where domain errors matter.
- Include context in errors so failures are actionable.
- Validate inputs early and fail with clear messages.

## 5. Ownership and Borrowing

- Prefer borrowing over cloning.
- Clone only when ownership transfer is required.
- Avoid unnecessary allocations in hot paths.
- Use references and slices for read-only operations.

## 6. Collections and Iteration

- Prefer iterator adapters over manual loops when clarity improves.
- Use explicit loops when they are clearer than chained combinators.
- Keep data transformations straightforward and testable.
- Preserve deterministic ordering where output stability matters.

## 7. Configuration and Serialization

- Keep schema changes backward-compatible where possible.
- Document defaults, precedence, and merge behavior.
- Validate parsed configuration before runtime execution.
- Add tests for merge order, override precedence, and error paths.

## 8. Concurrency and Safety

- Prefer message-passing or scoped synchronization over shared mutable state.
- Keep lock scope minimal.
- Handle poisoned locks explicitly when recovery is acceptable.
- Avoid blocking operations on async runtimes unless isolated.

## 9. Logging and Observability

- Log meaningful state transitions and failure reasons.
- Avoid noisy logs in normal success paths.
- Include enough context to debug without reproducing blindly.
- Keep dry-run output deterministic and complete.

## 10. Testing Standards

- Add unit tests for logic branches and edge cases.
- Add integration tests for critical user workflows.
- Cover failure modes, not just success paths.
- Update tests whenever behavior changes.

Minimum checks before merge:
- cargo fmt --all
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test

## 11. Documentation Expectations

- Update user-facing docs for new behavior and flags.
- Keep examples valid and executable.
- Document migration and compatibility impacts.
- Explain non-obvious design choices in short comments.

## 12. Review Checklist

Before submitting a change, confirm:
- Code is formatted and clippy-clean.
- No unwrap or expect in production code paths.
- Errors contain actionable context.
- Tests cover changed behavior and regressions.
- Docs and examples are updated.
- Changes are minimal and focused.

## 13. Additional Points of attention
- keep mod.rs files clean, meaning they contain no struct, fn, or impl sections
- when several files (>=3) in a directory have a similar function, different from other files in that directory, group them in a new subdirectory
- keep large structs (>35 lines, or structs that have one or more impl or impl <trait> sections) together with their impl and related impl <trait> sections together in a single file per type
