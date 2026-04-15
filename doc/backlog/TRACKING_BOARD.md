# Convergence Tracking Board

Date: 2026-04-15  
Scope: Epics A-E (Phase 0 to Phase 2)  
Source: doc/backlog/BACKLOG.md

## Board Columns

- Todo
- In Progress
- In Review
- Done

## Labels

- epic:foundation
- epic:proxmox
- epic:hooks
- epic:extensibility
- epic:hardening
- phase:0-foundation
- phase:1-features
- phase:2-hardening
- sprint:1
- blocked
- ready

## Milestones

- Phase-0-Foundation
- Phase-1-Features
- Phase-2-Hardening

## Sprint 1 Focus

- A-01 Create convergence ADR set
- A-02 Define target module ownership map
- A-03 Create convergence tracking board
- B-01 Port Proxmox parser into dedicated module (start)

## Ticket Registry (A-01 to E-03)

| ID | Title | Epic | Milestone | Labels | Depends On | Assignee | Status |
|---|---|---|---|---|---|---|---|
| A-01 | Create convergence ADR set | A | Phase-0-Foundation | epic:foundation, phase:0-foundation, sprint:1 | None | unassigned | Done |
| A-02 | Define target module ownership map | A | Phase-0-Foundation | epic:foundation, phase:0-foundation, sprint:1 | A-01 | unassigned | Done |
| A-03 | Create convergence tracking board | A | Phase-0-Foundation | epic:foundation, phase:0-foundation, sprint:1 | None | unassigned | Done |
| B-01 | Port Proxmox parser into dedicated module | B | Phase-1-Features | epic:proxmox, phase:1-features, sprint:1 | A-01 | unassigned | Done |
| B-02 | Add canonical mapper (intermediate model to current schema) | B | Phase-1-Features | epic:proxmox, phase:1-features | B-01 | unassigned | Done |
| B-03 | Add import CLI command | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-04 | Add import validation and reporting | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-05 | Integration tests for import-to-start pipeline | B | Phase-1-Features | epic:proxmox, phase:1-features | B-03, B-04 | unassigned | Done |
| C-01 | Define hook contract and execution policy | C | Phase-1-Features | epic:hooks, phase:1-features | A-01 | unassigned | Todo |
| C-02 | Implement hook runner service | C | Phase-1-Features | epic:hooks, phase:1-features | C-01 | unassigned | Todo |
| C-03 | Add schema support for hook definitions | C | Phase-1-Features | epic:hooks, phase:1-features | C-01 | unassigned | Todo |
| C-04 | Wire hooks into start and stop flow | C | Phase-1-Features | epic:hooks, phase:1-features | C-02, C-03 | unassigned | Todo |
| C-05 | Hook tests (unit and integration) | C | Phase-1-Features | epic:hooks, phase:1-features | C-04 | unassigned | Todo |
| D-01 | Identify extension seams and trait interfaces | D | Phase-1-Features | epic:extensibility, phase:1-features | A-02 | unassigned | Todo |
| D-02 | Implement compile-time extension registry | D | Phase-1-Features | epic:extensibility, phase:1-features | D-01 | unassigned | Todo |
| D-03 | Port one concrete extension from v1 patterns | D | Phase-1-Features | epic:extensibility, phase:1-features | D-02 | unassigned | Todo |
| D-04 | Extensibility docs and examples | D | Phase-1-Features | epic:extensibility, phase:1-features | D-03 | unassigned | Todo |
| E-01 | Regression suite expansion | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | B-05, C-05, D-03 | unassigned | Todo |
| E-02 | Performance and stability checks | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | E-01 | unassigned | Todo |
| E-03 | Beta release gating | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | E-01, E-02 | unassigned | Todo |

## Dependency Graph

- A-01 -> B-01
- A-01 -> C-01
- A-02 -> D-01
- B-01 -> B-02 -> B-03
- B-02 -> B-04
- (B-03 and B-04) -> B-05
- C-01 -> C-02
- C-01 -> C-03
- (C-02 and C-03) -> C-04 -> C-05
- D-01 -> D-02 -> D-03 -> D-04
- (B-05 and C-05 and D-03) -> E-01 -> E-02 -> E-03

## Usage Notes

- When moving to an external board (GitHub Projects), copy this table directly.
- Keep this file as the fallback source of truth for dependencies and sprint focus.
- Update Assignee and Status fields during planning and standups.
