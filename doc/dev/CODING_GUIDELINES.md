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
- Struct length target: under 35 lines when possible. If not possible, document in comments the reason.
- Function length target: under 35 lines when possible. If not possible, document in comments the reason.
- File length target: under 250 lines when possible. If not possible, document in comments the reason.
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

When repeatedly accessing structured keys in loops or dense transformation code, prefer a small local key variable over reconstructing the same owned key expression many times.

Desired pattern example:

```rust
let interface_key = Value::String("interface".to_string());
let drives_key = Value::String("drives".to_string());

if let Some(Value::Sequence(drives)) = controller_map.get(drives_key.clone()) {
  for drive in drives {
    if let Value::Mapping(map) = drive {
      let is_ide = map.get(interface_key.clone()).and_then(Value::as_str) == Some("ide");
      // ...
    }
  }
}
```

Avoid patterns that repeatedly rebuild identical key values inside the same block, especially in config/YAML mapping code.

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

Serialization policy for YAML/JSON config output:
- Prefer compact output by omitting fields that are set to their semantic default.
- Use serde defaults and `skip_serializing_if` consistently for `Option`, empty collections, and default scalar values when omission preserves behavior.
- Treat omission semantics as part of the schema contract: absence must deserialize to the same runtime behavior as explicit defaults.
- Preserve explicit values only when they are required to override profile/merge defaults.
- Preserve explicit values only when they are required to override profile/merge defaults.
- For any serialization compactness change, add or update tests for roundtrip equivalence and merge/override behavior.

Import output compactness rule:
- A field may be omitted from import output only when a profile assigned during import guarantees its restoration at runtime.
- Import-specific defaults must not live in mapper code. Proxmox runtime values not stored in `.conf` files belong in dedicated Proxmox profiles (`proxmox-base`, `proxmox-windows`, `proxmox-q35-uefi`).
- Device IDs (drives, networks) must be set from the Proxmox source key (e.g. `"scsi0"`, `"net0"`) to preserve boot-order lookup semantics.
- Preserve Proxmox runtime integration paths and identifiers when VMID is known:
  - tap interface names: `tap<vmid>i<index>`
  - pid file path: `/var/run/qemu-server/<vmid>.pid`
  - guest agent socket path (import/parity): `/var/run/qemu-server/<vmid>.qga`
- Preserve Proxmox-compatible PCI topology for imported devices when defaults are expected by guest OS behavior (for example NIC `bus/addr` placement and guest-agent controller placement).
- Preserve Proxmox-compatible serial topology for guest agent and SPICE/vdagent channels to avoid Looking Glass keyboard/input regressions.
- For Proxmox import changes that affect runtime arguments, update fixture snapshots and verify dry-run parity against captured Proxmox command lines.
- See ADR-0002 for the full single-defaults contract.

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
- Production `mod.rs` files are wiring-only, with implementation bodies moved into focused sibling modules.

## 13. Additional Points of attention
- keep mod.rs files clean, meaning they contain no struct, fn, or impl sections
- when several files (>=3) in a directory have a similar function, different from other files in that directory, group them in a new subdirectory
- keep `impl From<A> for B` in the same file as the `struct A` and `impl A` sections
- keep large structs (>35 lines, or structs that have one or more `impl` or `impl <Trait>` sections) together with their impl and related `impl <Trait>` sections together in a single file per type
- adhere to SOLID principles:
  - Single responsibility principle
  - Open / Closed principle
  - Liskov substitution principle
  - Interface segregation principle
  - Dependency inversion principle
