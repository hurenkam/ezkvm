# Phase 3 Development Docs Move Map

Date: 2026-05-23
Related checklist: `doc/dev/workflow/process/DOC_TREE_MIGRATION_CHECKLIST.md`
Related ticket: L-04

## Inventory and Classification

### Architecture
- `doc/dev/ARCHITECTURE_GUIDELINES.md` -> `doc/dev/architecture/architecture-guidelines.md`
- `doc/dev/MODULE_OWNERSHIP.md` -> `doc/dev/architecture/module-ownership.md`
- `doc/dev/EXTENSIBILITY_SEAMS.md` -> `doc/dev/architecture/extensibility-seams.md`
- `doc/dev/adr/**` -> `doc/dev/architecture/decisions/**`

### Workflow
- `doc/dev/CODING_GUIDELINES.md` -> `doc/dev/workflow/coding-guidelines.md`
- `doc/dev/process/**` -> `doc/dev/workflow/process/**`

### Domain Knowledge
- `doc/dev/notes/QEMU_BUS_AND_ADDR_ASSIGNMENT.md` -> `doc/dev/domain-knowledge/qemu-bus-and-addr-assignment.md`

### Analysis
- `doc/preparation/CODEBASE_ANALYSIS.md` -> `doc/dev/analysis/codebase-analysis.md`
- `doc/preparation/EZKVM_COMPARISON.md` -> `doc/dev/analysis/ezkvm-comparison.md`

## Batch Log

### Batch 1 (2026-05-23)

Moved documents:
- `doc/dev/ARCHITECTURE_GUIDELINES.md` -> `doc/dev/architecture/architecture-guidelines.md`
- `doc/dev/MODULE_OWNERSHIP.md` -> `doc/dev/architecture/module-ownership.md`
- `doc/dev/EXTENSIBILITY_SEAMS.md` -> `doc/dev/architecture/extensibility-seams.md`
- `doc/dev/CODING_GUIDELINES.md` -> `doc/dev/workflow/coding-guidelines.md`
- `doc/dev/notes/QEMU_BUS_AND_ADDR_ASSIGNMENT.md` -> `doc/dev/domain-knowledge/qemu-bus-and-addr-assignment.md`
- `doc/preparation/CODEBASE_ANALYSIS.md` -> `doc/dev/analysis/codebase-analysis.md`
- `doc/preparation/EZKVM_COMPARISON.md` -> `doc/dev/analysis/ezkvm-comparison.md`

Transition actions applied:
- forwarding stubs added at all moved source paths
- dev section landing pages updated to canonical destinations
- ADR and architecture-document references updated for moved paths

### Batch 2 (2026-05-23)

Moved documents:
- `doc/dev/adr/ADR-0001-base-selection.md` -> `doc/dev/architecture/decisions/ADR-0001-base-selection.md`
- `doc/dev/adr/ADR-0002-import-normalization-contract.md` -> `doc/dev/architecture/decisions/ADR-0002-import-normalization-contract.md`
- `doc/dev/adr/ADR-0003-hooks-policy.md` -> `doc/dev/architecture/decisions/ADR-0003-hooks-policy.md`
- `doc/dev/adr/ADR-0004-trait-seam-policy.md` -> `doc/dev/architecture/decisions/ADR-0004-trait-seam-policy.md`
- `doc/dev/adr/ADR-0005-q35-topology-contract.md` -> `doc/dev/architecture/decisions/ADR-0005-q35-topology-contract.md`
- `doc/dev/process/DOC_TREE_TAXONOMY_AND_MIGRATION_POLICY.md` -> `doc/dev/workflow/process/DOC_TREE_TAXONOMY_AND_MIGRATION_POLICY.md`
- `doc/dev/process/DOC_TREE_MIGRATION_CHECKLIST.md` -> `doc/dev/workflow/process/DOC_TREE_MIGRATION_CHECKLIST.md`
- `doc/dev/process/DOC_TREE_PHASE2_USER_COMMUNITY_MOVE_MAP.md` -> `doc/dev/workflow/process/DOC_TREE_PHASE2_USER_COMMUNITY_MOVE_MAP.md`
- `doc/dev/process/DOC_TREE_PHASE3_DEV_MOVE_MAP.md` -> `doc/dev/workflow/process/DOC_TREE_PHASE3_DEV_MOVE_MAP.md`

Transition actions applied:
- forwarding stubs added at all moved source paths
- architecture/workflow indexes updated to canonical section paths
- references updated in hub, contribution, and feature-plan docs