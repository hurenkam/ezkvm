# ezkvm_v3 Re-Implementation Project Plan

## Intent

Re-implement ezkvm from scratch to preserve design freedom while still learning from the previous implementation located at `~/Workspace/ezkvm`.

## Guiding Principles

- Keep an open mind on architecture, workflows, and UX choices.
- Reuse knowledge, not assumptions, from the previous implementation.
- Capture decisions and trade-offs early so they can be revised intentionally.
- Prefer incremental milestones with working outcomes.

## Initial Scope

- Establish project documentation and decision-making guardrails.
- Build a minimal, end-to-end baseline implementation path.
- Define where prior implementation insights are referenced and where they are intentionally ignored.

## Workstreams

### 1. Discovery and Comparative Learning

- Inventory key concepts and capabilities from `~/Workspace/ezkvm`.
- Record what should be carried over as lessons learned.
- Record open questions and design alternatives for v3.

### 2. Architecture and Design Choices

- Define core domain boundaries and module ownership.
- Select data, state, and orchestration patterns.
- Document a first-pass architecture decision record (ADR) set.

### 3. Implementation Baseline

- Create a thin vertical slice that proves the core flow.
- Add basic testing, linting, and developer feedback loops.
- Ensure the baseline is easy to iterate on.

### 4. Iterative Expansion

- Prioritize next features from validated usage scenarios.
- Expand capabilities in small increments.
- Keep documentation synchronized with implementation changes.

## Milestones

1. Foundation Ready
   - Copilot/documentation guardrails in place.
   - Initial plan and working conventions established.
2. Baseline Slice Complete
   - Core workflow runs end to end.
   - Basic quality gates are active.
3. First Feature Batch
   - High-priority capabilities implemented with tests.
   - Decision log updated with observed trade-offs.

## Risks and Mitigations

- Risk: Overfitting to old implementation patterns.
  - Mitigation: Explicitly document alternatives before selecting a design.
- Risk: Scope growth before baseline stabilizes.
  - Mitigation: Gate new feature work behind baseline completion.
- Risk: Documentation drift.
  - Mitigation: Update docs as part of definition-of-done for each change.

## Requirement Traceability Policy

- Backlog and feature planning entries must link to one or more requirement IDs from `doc/dev/requirements/product-requirements.md` (for example: FR-001, NFR-003).
- Traceability records must be maintained in `doc/planning/backlog/requirements-traceability-matrix.md` as items are added or status changes.
- If an item is operational or maintenance work with no direct requirement mapping, mark Requirement ID as `N/A` and add a short rationale in the evidence field.
- A planning item is not ready for implementation unless requirement linkage and evidence placeholders are present.

## Immediate Next Steps

1. Execute [schema-host-resource-boundary-slice](backlog/schema-host-resource-boundary-slice.md) as the next host/VM separation slice now that [canonical-yaml-validation](features/done/canonical-yaml-validation.md) is complete.
2. Expand corpus-backed validation baselines as adapter coverage grows.
3. Keep development standards and validation/reporting guidance synchronized with implementation changes.
