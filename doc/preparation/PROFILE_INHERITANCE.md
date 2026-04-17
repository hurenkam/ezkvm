# Profile Inheritance Design

## Purpose

This document captures the proposed changes to the profile system so profiles can include other profiles while preserving current merge semantics and backward compatibility.

## Summary

The recommended approach is to model profiles as a dependency graph instead of a flat list.

- Add `profiles` to profile YAML files.
- Resolve includes recursively before merge.
- Detect cycles and missing includes with explicit errors.
- De-duplicate included profiles while preserving deterministic order.
- Keep VM-level `profiles` behavior unchanged.

## Proposed Schema

Profile files gain an optional field:

```yaml
profiles:
  - base-profile
  - another-profile
```

All existing profile fields remain unchanged.

Naming note:

- VM files and profile files both use `profiles`.
- In VM files, `profiles` is the top-level profile stack to apply.
- In profile files, `profiles` is the list of base profiles that profile depends on.

No new VM-level schema is required. Existing VM files continue using:

```yaml
profiles:
  - profile-a
  - profile-b
```

## Resolution Model

For each VM profile entry:

1. Load the profile.
2. Resolve its `profiles` first (depth-first).
3. Apply the profile itself.

This gives intuitive precedence:

- Included base profiles apply first.
- Including profile overrides included profiles.
- VM-local config overrides all profiles.

## Merge Precedence

Effective order:

1. Deepest included profiles (base first)
2. Including profile
3. Next profile from VM `profiles` list
4. VM-local values

This preserves current expectations while adding profile composition.

## Error Handling

### Cycle detection

Track recursion stack during resolution.

If a cycle is found, fail with path-aware error, for example:

- `profile cycle detected: a -> b -> a`

### Missing referenced profile

If a referenced profile is not found, fail with profile name and searched path information.

## Determinism and De-duplication

If a profile is reached through multiple dependency paths:

- Apply it once.
- Preserve deterministic traversal/order.

This keeps merge outcomes stable for tests and snapshots.

## Implementation Targets

Primary resolution and merge path:

- `src/config/vm_schema/vm_config.rs`
- `src/config/loader/merge.rs`

Importer paths should remain compatible with resolved profile stacks:

- `src/import/proxmox/profile_compact.rs`
- `src/import/proxmox/io.rs`

## Validation Plan

Add tests for:

1. Happy path inheritance with VM override.
2. Multi-profile order (`profiles: [a, b]` applies `a`, then `b`, then child).
3. Duplicate references through different branches (applies once).
4. Missing referenced profile errors.
5. Direct and indirect cycle detection.
6. Importer compatibility and round-trip behavior with compacted YAML.

## Why This Approach

- Fits existing merge model instead of introducing a new abstraction.
- Backward compatible for existing profile files.
- Maintains deterministic behavior for reproducible tests and snapshots.
- Keeps VM schema simple while enabling reusable profile composition.
