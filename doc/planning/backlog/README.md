# Backlog Lifecycle

Target lifecycle layout:
- `active/`: current todo, in-progress, and in-review work
- `done/`: completed work archive
- `future/`: uncommitted future work

Canonical source of truth:
- Ticket definitions and active status now live in `doc/planning/backlog/active/<EPIC>.md`.
- Completed ticket history lives in `doc/planning/backlog/done/YYYY/<EPIC>.md`.
- Future, uncommitted work lives in `doc/planning/backlog/future/` buckets.

Legacy reference policy:
- `doc/backlog/` is retained as a historical reference set and should not receive new planning updates.
- When discrepancies exist, treat `doc/planning/backlog/` as authoritative.