# Development Requirements

This directory contains the initial requirements baseline for ezkvm.

## Documents

- `product-requirements.md`: Product goals, functional requirements, non-functional requirements, and acceptance criteria.
- `reference-derived-requirements.md`: Additional generic requirements derived from analysis of the reference implementation and documentation.
- `canonical-yaml-schema-contract.md`: Normative contract for canonical YAML structure and validation semantics.
- `determinism-contract.md`: Normative contract for deterministic effective-model and command rendering behavior.

## Related Architecture Contracts

- `../architecture/import-adapters.md`: Import adapter boundary and extension contract.
- `../architecture/model-separation-pipeline.md`: Stage contract for import, runtime resolution, and rendering.

## Scope Rules

- Requirements in this directory describe product behavior and architectural constraints.
- Implementation details, low-level design choices, and code ownership belong in architecture and workflow documentation.
- User-facing operational guidance belongs in `doc/user`.
