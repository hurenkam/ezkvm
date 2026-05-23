# Epic K Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| K-01 | Build packaging baseline inventory and contract | 2026-05-23 | Packaging baseline, contract, and scope inventory established |
| K-02 | Create Debian packaging skeleton for ezkvm | 2026-05-23 | Initial Debian packaging skeleton and install layout added |

## Ticket Definitions

### K-01 Build packaging baseline inventory and contract
Status: Done
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening
Assignee: unassigned
Dependencies: B-54

Scope:
- Inventory required versus optional runtime dependencies for packaged ezkvm.
- Confirm filesystem/runtime path contract for packaged installs (`/usr/bin`, `/etc/ezkvm`, `/run/ezkvm`, optional `/var/lib/ezkvm`, `/var/log/ezkvm`).
- Lock one-package-per-architecture policy for Debian Trixie and Ubuntu Resolute.

Acceptance Criteria:
- Dependency inventory and path contract documented in preparation docs.
- Required vs optional dependency split is explicit and reviewable.
- One-package-per-architecture contract is recorded as packaging policy.

Estimate: 1 day

Completion Notes (2026-05-23):
- Packaging baseline, contract, and scope inventory established.

### K-02 Create Debian packaging skeleton for ezkvm
Status: Done
Milestone: Phase-3-Packaging
Labels: epic:packaging, phase:2-hardening
Assignee: unassigned
Dependencies: K-01

Scope:
- Add initial `debian/` metadata (`control`, `rules`, `changelog`, `install`, runtime directory policy) to build installable artifacts.
- Ensure conffile-safe behavior for admin-edited files under `/etc/ezkvm`.

Acceptance Criteria:
- `dpkg-buildpackage -us -uc -b` produces installable package artifacts.
- Installed file layout matches packaging contract.
- Config defaults are preserved across reinstall/upgrade scenarios.

Estimate: 3 days

Completion Notes (2026-05-23):
- Initial Debian packaging skeleton and install layout added.

## Completion Detail

Milestone-level completion notes:
- K-01 and K-02 established the packaging baseline and skeleton needed for cross-distro hardening.

Archive note:
- K-03 through K-06 remain active planning items and are not duplicated here.

## Completion Detail

Milestone-level completion notes:
- `K-01` and `K-02` completed the packaging foundation sequence.
- Remaining packaging hardening and validation work (`K-03` to `K-06`) remains active.

Archive note:
- Completed packaging foundation tickets are archived here; ongoing packaging execution is tracked in `doc/planning/backlog/active/K.md`.