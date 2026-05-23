# Documentation Tree Migration Checklist

Date: 2026-05-23
Owner: Documentation maintainers
Related backlog epic: L

Use this checklist to track migration from the current `doc/` structure to the target audience-and-lifecycle model.

## Phase 1: Alignment and Scaffolding

- [x] Publish taxonomy and migration policy
- [x] Define canonical path and ownership rules
- [x] Create target skeleton directories
- [x] Add audience landing pages
- [x] Update `doc/README.md` to hub-style navigation
- [x] Move feature story to `doc/backlog/in_progress_features/` when implementation starts

## Phase 2: User and Community Split

- [x] Inventory user/operator documents under `doc/user/`
- [x] Identify participation docs to move to `doc/community/`
- [x] Create move map for each source document
- [ ] Move documents in small batches
- [ ] Leave forwarding stubs at old paths
- [ ] Update all internal links for moved docs
- [ ] Validate navigation from `doc/README.md`, `doc/user/`, and `doc/community/`

Execution artifact:
- `doc/dev/process/DOC_TREE_PHASE2_USER_COMMUNITY_MOVE_MAP.md`

Progress note:
- Batch 1 completed on 2026-05-23 (examples/import docs moved with stubs and link updates).
- Batch 2 completed on 2026-05-23 (contribution guide moved to community with stub and hub link updates).

## Phase 3: Development Docs Normalization

- [ ] Inventory development docs under `doc/dev/` and `doc/preparation/`
- [ ] Classify each doc as architecture, workflow, domain-knowledge, or analysis
- [ ] Move architecture docs into `doc/dev/architecture/`
- [ ] Move process docs into `doc/dev/workflow/`
- [ ] Move notes into `doc/dev/domain-knowledge/`
- [ ] Move analysis reports into `doc/dev/analysis/`
- [ ] Add/update indexes in each section

## Phase 4: Planning and Backlog Migration

- [ ] Define feature-centric file format for active backlog (`A.md`, `B.md`, ...)
- [ ] Build `doc/planning/backlog/active/` feature files
- [ ] Build archive strategy under `doc/planning/backlog/done/`
- [ ] Partition future work into `candidates`, `research-needed`, `icebox`
- [ ] Add required icebox metadata fields to templates
- [ ] Keep `BACKLOG.md` and `TRACKING_BOARD.md` synchronized during transition

## Phase 5: Issues Landing and Triage

- [ ] Create issue reporting guide for problems
- [ ] Create feature request guide
- [ ] Document triage flow and severity levels
- [ ] Create known issues index and maintenance policy
- [ ] Link issues flow to planning/backlog tracking

## Phase 6: Helper and Link Consistency

- [ ] Update Copilot instructions for new paths
- [ ] Update skills/agents with new documentation locations
- [ ] Run link/reference verification
- [ ] Remove obsolete stubs after stabilization-window criteria are met

## Validation Checklist (run after each phase)

- [ ] No critical broken links in edited area
- [ ] Root hub links remain valid
- [ ] Canonical-path rules still hold
- [ ] Backlog/tracking docs remain synchronized when planning docs are edited