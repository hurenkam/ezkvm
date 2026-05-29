# Requirements Traceability Matrix

Purpose: keep requirement-to-planning linkage explicit and lightweight for day-to-day planning.

## How To Use

1. Add a row when creating a backlog item or feature document.
2. Keep the Feature/Backlog item field as a clickable planning path.
3. Update status and owner whenever work state changes.
4. Add evidence link when implementation, validation, or review artifacts exist.

## Governance (Lightweight)

- Every active planning item should map to at least one requirement ID or `N/A` with rationale.
- Use requirement IDs exactly as defined in product requirements (for example: FR-001, NFR-002).
- Keep evidence links practical: PR, commit, test report, design note, or issue.
- Review this table during planning sync when items move across ideas, prepared, and done.

## Matrix Template

| Requirement ID | Feature/Backlog item | Status | Owner | Evidence link |
| --- | --- | --- | --- | --- |
| FR-001 | [example-feature-name](../features/ideas/example-feature-name.md) | ideas | unassigned | pending |
| FR-004 | [example-runtime-pipeline](../features/prepared/example-runtime-pipeline.md) | prepared | unassigned | pending |
| NFR-003 | [example-validation-improvements](../backlog/example-validation-improvements.md) | backlog | unassigned | pending |
| N/A (maintenance) | [example-refactor-task](../backlog/example-refactor-task.md) | backlog | unassigned | rationale: internal cleanup |
