---
name: design-pattern-expert
description: "Use when selecting, reviewing, applying, or refactoring software design patterns in ezkvm. Covers composition, strategy, trait-based polymorphism, builders, factories, state modeling, separation of concerns, and pattern tradeoffs."
argument-hint: "What design problem, refactor, or pattern choice should be analyzed?"
user-invocable: true
---

# Design Pattern Expert

## What This Skill Produces
- A concrete recommendation for the right pattern, or for avoiding a pattern when unnecessary
- A minimal architectural change plan aligned with existing code
- Explicit tradeoffs around complexity, extensibility, testability, and clarity
- Validation steps to confirm the pattern improves the design

## When to Use
- Choosing how to model a new subsystem
- Reviewing whether an existing abstraction is appropriate
- Refactoring duplicated logic into a clearer structure
- Deciding between traits, enums, structs, generics, builders, factories, or composition
- Checking whether a design is too abstract, too coupled, or too rigid

## Ezkvm-Specific Focus
Prefer recommendations that fit established repository patterns:
- Section-based composition for QEMU argument generation
- Trait-based polymorphism for runtime-selectable behavior
- Tagged runtime selection using `typetag` and YAML-backed trait objects
- OS abstraction boundaries to isolate host interactions
- Modular boundaries that keep import/config/runtime responsibilities clear

## Rust and GoF Mapping
Use GoF names as shorthand, but implement in Rust-native forms.

| GoF Pattern | Rust-First Expression | Prefer This When | Prefer Alternative When |
|---|---|---|---|
| Strategy | Trait object or enum-dispatch behavior family | Runtime-selectable behavior | Variant set is closed: use enum + `match` |
| Factory Method | Constructor functions returning concrete types | Construction is simple and local | Product families vary together: use abstract factory |
| Builder | Focused `with_*` helpers or staged builders | Many optional fields and invariants | Struct literal/constructor is already clear |
| Adapter | Newtype wrapper implementing expected trait | Integrating external shape without leaks | Minor mismatch: conversion helper is enough |
| Facade | Boundary module exposing simpler API | Hide OS/process/protocol complexity | Boundary would hide operationally critical behavior |
| State | Enum state machine with explicit transitions | Lifecycle correctness is central | Open-ended plugin state model: trait objects may fit |
| Command | Request structs/enums with execute path | CLI/RPC dispatch | Direct call flow is simpler |

## Rust-First Decision Ladder
Before selecting a named pattern, evaluate in this order:
1. Plain structs and functions
2. Composition of small modules/components
3. Enum + exhaustive `match` for closed variation
4. Generics for compile-time polymorphism
5. Trait objects for runtime-extensible variation
6. Macros/codegen only when repetition is structural and stable

## Working Rules
- Start from the design problem, not a favorite pattern name.
- Prefer the simplest structure that makes domain behavior clearer.
- Use patterns to reduce real coupling or duplication.
- Preserve repo conventions unless there is a concrete reason to change.
- Require concrete pain signals before introducing new abstractions.

## Anti-Patterns to Avoid
- Pattern-by-name refactors without measured pain
- Trait-object overuse for closed variant sets
- Inheritance-like trait hierarchies forced into Rust
- Abstractions that relocate complexity without reducing it

## Response Pattern
1. Problem framing and design pressure
2. Most suitable pattern or anti-pattern recommendation
3. Minimal architectural change
4. Validation steps
5. Tradeoffs and alternatives
