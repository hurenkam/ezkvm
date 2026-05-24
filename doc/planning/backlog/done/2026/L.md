# Epic L Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| L-01 | Define target documentation taxonomy and migration policy | 2026-05-23 | Taxonomy and migration policy published |
| L-02 | Add new doc-tree skeleton and navigation hubs | 2026-05-23 | Target skeleton and navigation hubs added |
| L-03 | Migrate user and community-facing docs to target structure | 2026-05-23 | User/community migration batches completed |
| L-04 | Migrate development docs to architecture/workflow/domain/analysis model | 2026-05-23 | Phase 3 normalization completed |
| L-05 | Migrate backlog representation to feature-centric active/done/future model | 2026-05-23 | Feature-centric planning lifecycle and templates completed |
| L-06 | Create issues landing, intake guides, and triage flow docs | 2026-05-23 | Issues landing, intake guides, triage, and known-issues docs published |
| L-07 | Update Copilot helpers and run consistency/link validation | 2026-05-23 | Helper references synchronized and deprecated forwarding stubs removed |

## Ticket Definitions

### L-01 Define target documentation taxonomy and migration policy
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: None

Scope:
- Define approved target documentation taxonomy for user/community/dev/planning/issues audiences.
- Define naming, ownership, and canonical-path rules.
- Publish migration mapping from current paths to target paths.

Acceptance Criteria:
- Approved taxonomy and migration policy committed in developer docs.
- Mapping table covers all current top-level doc areas.
- Policy includes deprecation and redirect/stub strategy.

Estimate: 1 day

Completion Notes (2026-05-23):
- Published taxonomy and migration policy at `doc/dev/workflow/process/DOC_TREE_TAXONOMY_AND_MIGRATION_POLICY.md`.
- Added canonical-path rules, ownership model, phased migration policy, and current-to-target mapping table.

### L-02 Add new doc-tree skeleton and navigation hubs
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-01

Scope:
- Create top-level target directories and landing pages.
- Add audience-based navigation and ownership hints.
- Add canonical entrypoint from `doc/README.md`.

Acceptance Criteria:
- Target top-level directories exist with minimal index docs.
- `doc/README.md` links to all audience landing pages.
- Navigation supports old and new readers during transition.

Estimate: 1 day

Completion Notes (2026-05-23):
- Started implementation; feature document moved to `doc/planning/epics/in-progress/`.
- Created target skeleton directories for `community`, `dev/{architecture,workflow,domain-knowledge,analysis}`, `planning/backlog/{active,done,future}`, `planning/epics`, and `issues`.
- Added landing hub README files for each target section and updated `doc/README.md` navigation.

### L-03 Migrate user and community-facing docs to target structure
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-02

Scope:
- Move user/operator docs into target `user` structure.
- Separate contribution/participation guidance into `community` docs.
- Add temporary compatibility stubs for moved pages.

Acceptance Criteria:
- User/operator content is discoverable from user landing page.
- Contribution docs are discoverable from community landing page.
- Moved docs have redirects/stubs and no critical orphaned links.

Estimate: 3 days

Execution Notes (2026-05-23):
- Batch 1: moved user docs for examples and import guides to `doc/user/how-to/**`, added forwarding stubs at old paths, and updated links in `doc/user/config/README.md`.
- Batch 2: moved contributing guide to `doc/community/contribution-guide/README.md`, added forwarding stub at `doc/dev/CONTRIBUTING.md`, and updated hub/community references.

Completion Notes (2026-05-23):
- Batch 3: moved remaining config reference docs to `doc/user/reference/config/**` and troubleshooting to `doc/user/troubleshooting/config.md`, with forwarding stubs and link rewrites.
- Batch 4: moved Ubuntu bridge guide to `doc/user/how-to/networking/ubuntu-netplan-bridge.md` and added forwarding stub at old path.
- Navigation validation completed across `doc/README.md`, `doc/user/`, and `doc/community/`.
- L-03 acceptance criteria satisfied.

### L-04 Migrate development docs to architecture/workflow/domain/analysis model
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-02

Scope:
- Reorganize dev docs into architecture, workflow, domain-knowledge, and analysis areas.
- Migrate existing domain notes and analysis reports into dedicated locations.
- Document maintenance workflow for domain-knowledge notes.

Acceptance Criteria:
- Dev docs are split by intent with clear landing pages.
- Domain-knowledge notes include metadata and validation guidance.
- Analysis reports are separated from normative architecture/process docs.

Estimate: 3 days

Execution Notes (2026-05-23):
- Started implementation; tracking status set to In Progress.
- Batch 1 completed: moved core architecture/workflow docs, one domain-knowledge note, and two preparation analysis reports to canonical dev taxonomy paths.
- Added forwarding stubs at all moved old paths and updated section indexes plus affected cross-links.
- Batch 2 completed: moved ADRs to `doc/dev/architecture/decisions/**` and process docs to `doc/dev/workflow/process/**`, with forwarding stubs and canonical-link updates.
- Added metadata/validation maintenance guidance in `doc/dev/domain-knowledge/README.md` for domain-note consistency.
- Migration artifact created: `doc/dev/workflow/process/DOC_TREE_PHASE3_DEV_MOVE_MAP.md`.

Completion Notes (2026-05-23):
- Development docs are now split by intent across architecture, workflow, domain-knowledge, and analysis sections.
- Domain-knowledge maintenance guidance (metadata + validation expectations) is documented in `doc/dev/domain-knowledge/README.md`.
- Analysis reports are separated from normative architecture/process docs under `doc/dev/analysis/**`.
- L-04 acceptance criteria satisfied.

### L-05 Migrate backlog representation to feature-centric active/done/future model
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-01, L-02

Scope:
- Introduce feature-level backlog files (`A.md`, `B.md`, etc.) for active planning.
- Keep active backlog focused on todo/in-progress/in-review.
- Move completed work to done archives and future/uncommitted work to future buckets.
- Define semantics for `future/candidates`, `future/research-needed`, and `future/icebox`.

Acceptance Criteria:
- Active backlog no longer depends on a monolithic ticket-only view.
- Completed and future work are clearly separated from active planning.
- Icebox entries require rationale, re-entry trigger, and review cadence.

Estimate: 4 days

Execution Notes (2026-05-23):
- Started implementation; tracking status set to In Progress.
- Batch 1 completed: introduced feature-centric planning files for Epic L:
	- active: `doc/planning/backlog/active/L.md`
	- done archive: `doc/planning/backlog/done/2026/L.md`
- Updated planning backlog readmes to expose initial feature files and archive location.
- Batch 2 completed: defined reusable templates for:
	- active feature files: `doc/planning/backlog/active/FEATURE_TEMPLATE.md`
	- future candidates: `doc/planning/backlog/future/candidates/TEMPLATE.md`
	- future research-needed: `doc/planning/backlog/future/research-needed/TEMPLATE.md`
	- future icebox: `doc/planning/backlog/future/icebox/TEMPLATE.md`
- Updated future bucket readmes to point to templates and enforce required item fields.
- Batch 3 completed: expanded active feature-centric files to cover currently open epics (`B.md`, `C.md`, `D.md`, `E.md`, `K.md`, `L.md`).
- Batch 4 completed: archived completed feature-centric files for epics with finished work under `doc/planning/backlog/done/2026/`.

Completion Notes (2026-05-23):
- Active planning now has feature-centric files for open epics under `doc/planning/backlog/active/`.
- Completed work is archived under `doc/planning/backlog/done/2026/`.
- Future backlog buckets have explicit templates, including required icebox metadata fields.
- L-05 acceptance criteria satisfied.

### L-06 Create issues landing, intake guides, and triage flow docs
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-02

Scope:
- Add issue landing docs for problems and feature requests.
- Define triage model, severity/priority conventions, and known-issues handling.
- Cross-link issue flow with planning tickets.

Acceptance Criteria:
- Users and developers have a clear issue/reporting entrypoint.
- Triage workflow and ownership are documented.
- Known issues and resolution status are discoverable.

Estimate: 2 days

Completion Notes (2026-05-23):
- Expanded `doc/issues/README.md` into a usable entrypoint with links for problem reports, feature requests, triage, and known issues.
- Added `doc/issues/report-a-problem.md`, `doc/issues/request-a-feature.md`, `doc/issues/triage.md`, and `doc/issues/known-issues.md`.
- Documented severity/priority guidance, ownership areas, backlog linkage, and known-issues maintenance rules.
- L-06 acceptance criteria satisfied.

### L-07 Update Copilot helpers and run consistency/link validation
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:docs-ia, phase:2-hardening
Assignee: unassigned
Dependencies: L-03, L-04, L-05, L-06

Scope:
- Update instructions/skills/agents to reference new doc paths and domain note locations.
- Run documentation consistency checks and fix broken references.
- Remove deprecated stubs after transition stabilization criteria are met.

Acceptance Criteria:
- Copilot helper applyTo and references match new doc layout.
- Link and reference checks pass for migrated docs.
- Deprecation cleanup plan executed or explicitly deferred with rationale.

Estimate: 2 days

Completion Notes (2026-05-23):
- Updated Copilot/customization references to canonical workflow, architecture, analysis, and issue-document paths.
- Updated lingering documentation references outside migration artifacts so active helper/docs links resolve to current canonical locations.
- Ran a consistency sweep for old helper/doc path references; remaining hits are limited to intentional migration-history and mapping documents.
- Removed deprecated forwarding stubs after canonical references were updated and consistency verification passed.
- L-07 acceptance criteria satisfied.