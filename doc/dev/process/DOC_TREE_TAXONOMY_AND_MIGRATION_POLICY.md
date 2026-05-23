# Documentation Tree Taxonomy and Migration Policy

Date: 2026-05-23
Owner: Documentation maintainers
Related backlog ticket: L-01

## Purpose

Define the target documentation taxonomy, ownership model, canonical path rules, and migration policy for moving from the current documentation tree to the target audience-and-lifecycle model.

## Target Taxonomy

```text
doc/
  README.md
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
    epics/
  issues/
```

## Audience and Ownership

| Area | Primary audience | Primary owner | Content type |
|---|---|---|---|
| `doc/user/` | VM operators and end users | Maintainers of user-facing behavior | How-to, reference, troubleshooting |
| `doc/community/` | Contributors and project participants | Project maintainers | Contribution, review, governance |
| `doc/dev/architecture/` | Developers | Architecture maintainers | Invariants, ADRs, module boundaries |
| `doc/dev/workflow/` | Developers | Engineering process maintainers | Way-of-working and quality gates |
| `doc/dev/domain-knowledge/` | Developers | Domain maintainers | Generic domain notes (QEMU/Q35/Proxmox/host OS) |
| `doc/dev/analysis/` | Developers | Feature or investigation authors | Time-bound analysis reports and deep dives |
| `doc/planning/` | Maintainers and contributors | Planning maintainers | Roadmap, backlog lifecycle, feature progress |
| `doc/issues/` | Users and developers | Triage maintainers | Bug/feature intake, triage, known issues |

## Canonical Path Rules

1. User-operational guidance belongs in `doc/user/`.
2. Participation/contribution guidance belongs in `doc/community/`.
3. Engineering policy and architecture belong in `doc/dev/architecture/` and `doc/dev/workflow/`.
4. Generic technical domain facts belong in `doc/dev/domain-knowledge/`.
5. Analysis artifacts, comparisons, and investigations belong in `doc/dev/analysis/`.
6. Backlog and roadmap lifecycle documents belong in `doc/planning/`.
7. Issue intake and triage process documentation belongs in `doc/issues/`.

Conflict resolution rule:
- If the same content appears in multiple places, the canonical location is the one determined by rules 1-7, and other locations must contain a short redirect/stub only.

## Backlog Representation Policy

1. Active backlog should be feature-centric (`A.md`, `B.md`, etc.), not ticket-file-centric.
2. Active backlog includes only `Todo`, `In Progress`, and `In Review` work.
3. Completed work moves to `planning/backlog/done/`.
4. Future/uncommitted work moves to `planning/backlog/future/`.

Future buckets:
- `candidates/`: likely future work with plausible planning horizon.
- `research-needed/`: items blocked on discovery/validation.
- `icebox/`: intentionally parked items with low urgency or uncertain ROI.

Required fields for `icebox` entries:
- parked reason
- re-entry trigger
- last reviewed date
- review cadence

## Migration Policy

1. Migrate in phases, one audience area at a time.
2. Keep old paths as temporary stubs during a stabilization window.
3. Every moved document must leave a forwarding stub at the old path until references are updated.
4. Update cross-links and helper references in the same phase as moves.
5. Prefer small migration batches to limit link break blast radius.

Stabilization window policy:
- Keep stubs for at least one full migration phase after moves affecting the corresponding audience area.
- Remove stubs only after link/reference verification passes.

## Current to Target Mapping

This table maps the current top-level documentation areas to target locations.

| Current path | Target path | Notes |
|---|---|---|
| `doc/README.md` | `doc/README.md` | Keep as top-level navigation hub; rewrite links by target audience |
| `doc/user/**` | `doc/user/**` and `doc/community/**` | Split participation docs out of user area where applicable |
| `doc/dev/ARCHITECTURE_GUIDELINES.md` | `doc/dev/architecture/` | Move into architecture section with stable index |
| `doc/dev/CODING_GUIDELINES.md` | `doc/dev/workflow/` | Treat as workflow/process policy |
| `doc/dev/CONTRIBUTING.md` | `doc/community/contribution-guide/` | Participation guidance should live in community area |
| `doc/dev/MODULE_OWNERSHIP.md` | `doc/dev/architecture/` | Module boundaries and ownership model |
| `doc/dev/EXTENSIBILITY_SEAMS.md` | `doc/dev/architecture/` | Architecture-level seam policy |
| `doc/dev/adr/**` | `doc/dev/architecture/decisions/**` | Keep ADRs together under architecture decisions |
| `doc/dev/notes/**` | `doc/dev/domain-knowledge/**` | Domain reference notes move under domain-knowledge |
| `doc/dev/process/**` | `doc/dev/workflow/**` | Process/how-to-maintain docs |
| `doc/preparation/**` | `doc/dev/analysis/**` | Analysis and comparisons are development analysis artifacts |
| `doc/backlog/**` | `doc/planning/backlog/**` and `doc/planning/epics/**` | Backlog + feature plan lifecycle split |
| `doc/issues/**` | `doc/issues/**` | Retain area, normalize intake/triage layout |

## Out-of-Tree Notes Policy

Repository-root `notes/**` files are not canonical documentation paths. During migration, each retained note should be moved into either:
- `doc/dev/analysis/` (investigation/report), or
- `doc/planning/` (planning artifact), or
- `doc/dev/domain-knowledge/` (generic domain note).

## Approval and Change Control

Changes to this policy require:
1. update proposal in `doc/dev/process/`
2. synchronized backlog/tracking updates when planning impact exists
3. explicit note in migration log describing impact on existing links

Execution tracking checklist:
- `doc/dev/process/DOC_TREE_MIGRATION_CHECKLIST.md`