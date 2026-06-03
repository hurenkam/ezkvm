---
name: Module Design
description: "Shape a new ezkvm_v3 module or stage boundary before implementation, using the current pipeline conventions."
argument-hint: "What module or stage are you designing, and what boundary or behavior must it satisfy?"
---

Design the module or stage before writing code.

## Inputs

- target path or stage name
- purpose and ownership boundary
- required inputs, outputs, and diagnostics
- known constraints from decisions, history, and architecture docs

## Workflow

1. Read `.squad/decisions.md`, the agent history, `doc/dev/architecture/coding-guidelines.md`, `doc/README.md`, and the relevant architecture docs. Also read the nearest `README.md` in the `src` subtree for the module or stage path being designed.
	- Any `src/**/README.md` is manually authored design and requirement documentation for its subtree. Do not edit it unless the user explicitly requests edits to that file and confirms.
2. Place the work in the existing pipeline: `config_importer`, `runtime_config`, `runtime_model`, or `config_exporter`.
3. Define the smallest responsible unit: one module, one trait boundary, or one adapter family.
4. Specify the public contract: request shape, output shape, error shape, validation boundary, and whether the boundary must stay synchronous or object-safe.
5. Call out what must stay deterministic, what may vary by host or source, and what must be rejected early.
6. If the design changes team conventions or file placement, record a decision note before implementation.

## Output

Return a concise design brief with:

- proposed module path
- responsibility statement
- public API or trait shape
- validation and error handling rules
- test cases that should exist before merge
- documentation impact, if any

## Guardrails

- Do not design across multiple stages unless a boundary change is the point of the task.
- Do not add shared abstractions until at least two real implementations need them.
- Keep the proposal aligned with the current stage contracts and naming used in this repo.