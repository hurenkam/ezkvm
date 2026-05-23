# Documentation Information Architecture Restructure

## Purpose

Define and implement a documentation information architecture that separates audience concerns, keeps planning artifacts maintainable, and gives issues a clear landing place.

## Improvement Story

As an ezkvm maintainer and contributor,
I want the documentation tree to be organized by audience and lifecycle,
so that users can find usage guidance quickly, contributors can find participation and development guidance predictably, and planning/issue workflows remain easy to maintain as the project grows.

## Goals

- Separate user/operator docs from participation docs.
- Separate development docs (architecture, way-of-working, domain knowledge, analysis reports) from planning docs.
- Rework backlog representation so active work stays small and archived/future work moves out of the active backlog surface.
- Provide an issue landing area for both user-reported and developer-reported problems and feature requests.

## Non-Goals

- No behavior or runtime changes to ezkvm itself.
- No schema/model changes for VM configuration.
- No forced rewrite of all historical documents in one step.

## Target Documentation Model

```text
doc/
  user/
  community/
  dev/
    architecture/
    workflow/
    domain-knowledge/
    analysis/
  planning/
    backlog/
      active/
      done/
      future/
  issues/
```

Design intent:
- `user/`: usage and operator outcomes.
- `community/`: contribution and participation workflows.
- `dev/`: engineering architecture/process/domain reference material.
- `planning/`: roadmap, backlog lifecycle, and delivery tracking.
- `issues/`: intake and triage workflows for bugs and feature requests.

## Backlog Model Changes

- Use feature-level files (`A.md`, `B.md`, etc.) in active planning views rather than one file per ticket.
- Keep active planning focused on todo/in-progress/in-review only.
- Move completed work to `planning/backlog/done/`.
- Keep future/uncommitted work in `planning/backlog/future/` with subareas:
  - `candidates/`: likely near/mid-term but not committed.
  - `research-needed/`: requires investigation before commitment.
  - `icebox/`: intentionally parked work with low urgency/uncertain ROI and explicit re-entry triggers.

## Transition Plan

L-01 execution artifact:
- `doc/dev/process/DOC_TREE_TAXONOMY_AND_MIGRATION_POLICY.md`

L-02 execution artifact:
- `doc/dev/process/DOC_TREE_MIGRATION_CHECKLIST.md`

### Phase 1: Alignment and scaffolding

1. Freeze target taxonomy and naming conventions.
2. Add top-level navigation hubs and ownership notes.
3. Add migration mapping table from current paths to target paths.

### Phase 2: User/community split

1. Move and normalize current user-facing docs into `doc/user/` target substructure.
2. Extract participation/contribution docs into `doc/community/`.
3. Add cross-links and redirects/stubs at old locations.

### Phase 3: Developer docs normalization

1. Split development docs into architecture/workflow/domain-knowledge/analysis sections.
2. Migrate and tag existing domain notes and analysis reports.
3. Add maintenance workflow for keeping domain knowledge current.

### Phase 4: Planning and backlog migration

1. Create `planning/backlog/active|done|future` skeleton.
2. Convert backlog representation to feature-level files with ticket tables.
3. Archive completed and future tickets away from active surface.

### Phase 5: Issues landing and triage

1. Create issue landing docs for bug reports and feature requests.
2. Define triage flow and known-issue handling.
3. Cross-link issue flow to planning tickets.

### Phase 6: Tooling and guardrails

1. Update Copilot instructions/skills/agents to use new paths and domain-knowledge location.
2. Run link/consistency checks and fix residual references.
3. Remove deprecated stubs after stabilization window.

## Risks and Mitigations

1. Link breakage during moves.
   - Mitigation: staged redirects/stubs and link checks per phase.
2. Confusion during dual-path transition.
   - Mitigation: migration map and temporary canonical-path banner notes.
3. Drift between backlog docs and tracking views.
   - Mitigation: same-task sync and explicit dependency graph updates.

## Acceptance Criteria

- New taxonomy is documented and approved.
- Migration plan is executed in phases with minimal navigation regressions.
- Backlog active view is significantly smaller and feature-centric.
- Completed and future work are stored outside active backlog views.
- Issue landing and triage docs are available and linked from doc root.
- Copilot helpers resolve and reference the new doc paths reliably.

## Backlog Mapping

This improvement story is implemented by tickets `L-01` through `L-07` in `doc/backlog/BACKLOG.md` and tracked in `doc/backlog/TRACKING_BOARD.md`.