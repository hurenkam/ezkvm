---
name: serde-schema
description: "Use when designing, reviewing, debugging, or refactoring Rust serde schemas and YAML mapping in ezkvm. Covers defaults, flatten, untagged enums, tagged variants, typetag compatibility, backward compatibility, and config-model evolution."
argument-hint: "What serde schema, YAML mapping, or deserialization issue should be analyzed?"
user-invocable: true
---

# Serde Schema

## What This Skill Produces
- A concrete diagnosis of how Rust serde models map to YAML
- A minimal Rust, YAML, or docs change recommendation
- Backward-compatibility and default-behavior analysis
- Validation steps tied to deserialization tests and runtime consumers

## When to Use
- Designing or changing serde-backed structs and enums
- Debugging YAML deserialization failures
- Reviewing schema evolution for compatibility risk
- Working with `flatten`, tagged enums, untagged enums, or custom defaults
- Auditing `typetag`-based trait object deserialization

## Ezkvm-Specific Focus
Pay particular attention to:
- Top-level configuration composition and section defaults
- Missing-field default behavior
- `typetag` with `tag = "type"` for polymorphic sections
- `flatten` and `untagged` interactions that can create ambiguous YAML
- Drift between code, docs, and sample YAML

## Working Rules
- Start from Rust consumers and tests, not only YAML examples.
- Preserve backward compatibility unless explicit breaking change is requested.
- Prefer explicit, stable schemas over clever ambiguous deserialization behavior.
- Keep the narrowest schema change that solves the problem.

## Workflow
1. Frame the schema task and assumptions.
2. Inspect the effective contract (Rust types, serde attributes, helper behavior, tests, docs/examples).
3. Identify exact schema shape and discriminators.
4. Branch by issue type (defaults, tagged variants, flatten/untagged ambiguity, doc drift).
5. Recommend the smallest correct change.
6. Validate compatibility and runtime-consumer expectations.

## Completion Checks
- Schema description matches actual serde behavior.
- Compatibility is preserved unless intentional breakage is requested.
- Defaults, tags, and flattened shapes are explicit and defensible.
- Tests or validation steps cover changed behavior.
- Docs/examples are not left in contradiction with code.

## Response Pattern
1. Problem framing and current schema assumptions
2. Effective serde behavior and likely failure point
3. Minimal recommended change
4. Validation steps
5. Compatibility risks and remaining ambiguities
