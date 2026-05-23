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
- [x] Move documents in small batches
- [x] Leave forwarding stubs at old paths
- [x] Update all internal links for moved docs
- [x] Validate navigation from `doc/README.md`, `doc/user/`, and `doc/community/`

Execution artifact:
- `doc/dev/workflow/process/DOC_TREE_PHASE2_USER_COMMUNITY_MOVE_MAP.md`

Progress note:
- Batch 1 completed on 2026-05-23 (examples/import docs moved with stubs and link updates).
- Batch 2 completed on 2026-05-23 (contribution guide moved to community with stub and hub link updates).
- Batch 3 completed on 2026-05-23 (reference and troubleshooting config docs moved with stubs and relative-link rewrites).
- Batch 4 completed on 2026-05-23 (Ubuntu networking guide moved with stub and navigation validation).

## Phase 3: Development Docs Normalization

- [x] Inventory development docs under `doc/dev/` and `doc/preparation/`
- [x] Classify each doc as architecture, workflow, domain-knowledge, or analysis
- [x] Move architecture docs into `doc/dev/architecture/`
- [x] Move process docs into `doc/dev/workflow/`
- [x] Move notes into `doc/dev/domain-knowledge/`
- [x] Move analysis reports into `doc/dev/analysis/`
- [x] Add/update indexes in each section

Execution artifact:
- `doc/dev/workflow/process/DOC_TREE_PHASE3_DEV_MOVE_MAP.md`

Progress note:
- Batch 1 completed on 2026-05-23 (core architecture/workflow docs, one domain note, and preparation analysis reports moved with forwarding stubs and reference updates).
- Batch 2 completed on 2026-05-23 (ADRs moved to `doc/dev/architecture/decisions/**`, process docs moved to `doc/dev/workflow/process/**`, with forwarding stubs and canonical-link updates).
- Phase 3 completed on 2026-05-23.

## Phase 4: Planning and Backlog Migration

- [x] Define feature-centric file format for active backlog (`A.md`, `B.md`, ...)
- [x] Build `doc/planning/backlog/active/` feature files
- [x] Build archive strategy under `doc/planning/backlog/done/`
- [x] Partition future work into `candidates`, `research-needed`, `icebox`
- [x] Add required icebox metadata fields to templates
- [x] Keep `BACKLOG.md` and `TRACKING_BOARD.md` synchronized during transition

Progress note:
- Batch 1 completed on 2026-05-23 (introduced first active/done feature-centric files for Epic L).
- Batch 2 completed on 2026-05-23 (added active/future templates, including required icebox fields).
- Batch 3 completed on 2026-05-23 (expanded active feature files for currently open epics B/C/D/E/K/L and documented archive strategy under `done/2026/`).
- Batch 4 completed on 2026-05-23 (archived completed epics A/B/D/K/L/M/N/Q under `doc/planning/backlog/done/2026/`).

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

- [x] No critical broken links in edited area
- [x] Root hub links remain valid
- [x] Canonical-path rules still hold
- [x] Backlog/tracking docs remain synchronized when planning docs are edited