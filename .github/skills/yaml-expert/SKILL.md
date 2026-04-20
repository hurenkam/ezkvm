---
name: yaml-expert
description: "Use when creating, reviewing, debugging, refactoring, or validating YAML in ezkvm. Covers YAML syntax, quoting, mappings, sequences, schema-aware editing, serde compatibility, and configuration design."
argument-hint: "What YAML file, schema, config, or parsing issue should be handled?"
user-invocable: true
---

# YAML Expert

## What This Skill Produces
- A syntactically valid, schema-aware YAML recommendation or edit plan
- A concrete diagnosis for parsing, shape, quoting, or compatibility problems
- A minimal change that preserves readability and consumer compatibility
- Validation steps tied to the actual loader/schema/runtime behavior

## When to Use
- Creating or editing YAML configuration files
- Debugging YAML parsing/deserialization failures
- Reviewing YAML for correctness, clarity, and compatibility
- Aligning examples with implemented serde models

## Ezkvm-Specific Focus
YAML surfaces commonly include:
- VM and profile configuration under `etc/`
- Documentation examples under `doc/user/config/`
- Importer output/config examples in tests and docs

Key assumptions:
- YAML is deserialized through `serde_yaml`
- Some sections are polymorphic and require `type` tags
- Missing fields may map to defaults rather than hard errors

## Working Rules
- Start with the consumer of the YAML, not just the text.
- Preserve semantics before improving style.
- Prefer the narrowest change that satisfies schema and user goal.
- Do not invent unsupported fields/tags/section names.

## Workflow
1. Frame the YAML task and consuming schema.
2. Identify effective schema in code and docs.
3. Validate structure (indentation, mapping/sequence shape, keys, scalar types, required tags).
4. Branch by issue type (parse failure, schema mismatch, docs-example drift).
5. Recommend minimal corrected YAML or adjacent fix in docs/code when YAML-only change is insufficient.
6. Validate syntax plus schema alignment.

## Completion Checks
- YAML is valid and schema-aware.
- Proposed changes preserve or intentionally update semantics.
- Polymorphic sections include expected tags/fields.
- Docs/examples/code are not left contradictory.

## Response Pattern
1. Problem framing and schema assumptions
2. Most likely YAML or schema issue
3. Minimal corrected YAML or recommended change
4. Validation steps
5. Risks, ambiguities, or doc-code mismatches
