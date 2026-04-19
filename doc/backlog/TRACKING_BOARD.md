# Convergence Tracking Board

Date: 2026-04-19  
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
| B-06 | Parse and map efidisk0 for UEFI vars parity | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-07 | Parse and map audio0 for SPICE HDA parity | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-08 | Parse and map agent field for guest-agent plumbing | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-09 | Parse and map args passthrough safe subset | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02, B-04 | unassigned | Done |
| B-10 | Preserve machine and CPU feature fidelity | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-11 | Improve network backend fidelity for Proxmox bridge/tap | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-12 | Expand host PCI fidelity for multifunction devices | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-02 | unassigned | Done |
| B-13 | Add wakiza parity fixture and regression test | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-06, B-07, B-08, B-09, B-10, B-11, B-12 | unassigned | Done |
| B-14 | Improve CPU and Hyper-V fidelity for Windows guests | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-10 | unassigned | Done |
| B-15 | Expand network device fidelity (virtio-net-pci placement and queues) | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-11 | unassigned | Done |
| B-16 | Refine firmware and secure-boot mapping from efidisk metadata | B | Phase-1-Features | epic:proxmox, phase:1-features, ready | B-06 | unassigned | Done |
| B-17 | Add SATA support | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-18 | Serial port configuration | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-19 | Support IOMMU/vIOMMU device definitions | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-20 | Full mapping of hugepages settings | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02 | unassigned | Done |
| B-21 | Materialize hugepages profile layer end-to-end | B | Phase-1-Features | epic:proxmox, phase:1-features | B-20 | unassigned | Done |
| B-22 | Add canonical VNC profile schema and headless-vnc layer | B | Phase-1-Features | epic:proxmox, phase:1-features | B-02, B-05 | unassigned | Done |
| B-23 | Add viommu and hidden-hypervisor tuning profile assignment | B | Phase-1-Features | epic:proxmox, phase:1-features | B-10, B-19 | unassigned | Done |
| B-24 | Tighten profile-aware compaction ownership boundaries | B | Phase-1-Features | epic:proxmox, phase:1-features | B-21 | unassigned | Done |
| B-25 | Expand profile-stack corpus and edge-case coverage | B | Phase-1-Features | epic:proxmox, phase:1-features | B-24 | unassigned | Done |
| B-26 | Decompose mapper orchestration and module boundaries | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening, ready | B-25 | unassigned | Done |
| B-27 | Split profile-aware compaction by domain ownership | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-26 | unassigned | Done |
| B-28 | Restrict Proxmox importer public surface to high-level API | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-26 | unassigned | Done |
| B-29 | Add temporary over-size rationale and cleanup guardrails | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-26, B-27, B-28 | unassigned | Done |
| B-30 | Audit profile-first compaction implementation status | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-29 | unassigned | Todo |
| B-31 | Expand profile inference coverage for Proxmox importer | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-23 | unassigned | Todo |
| B-32 | Omit deterministic fields in import-output mode | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-29 | unassigned | Done |
| B-33 | Introduce explicit export modes (canonical, compact, debug-canonical) | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-30, B-32 | unassigned | Done |
| B-34 | Extend compaction policies for repeated field omission | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-29 | unassigned | Done |
| B-35 | Update schema to attach drives to controllers which belong to devices | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-30 | unassigned | Done |
| B-36 | Add central host capability schema for portable runtime | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-33, D-01 | unassigned | Done |
| B-37 | Define and implement runtime precedence contract | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-36 | unassigned | Done |
| B-38 | Add portability preflight validation and error model | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-36, B-37 | unassigned | Done |
| B-39 | Separate parity-only defaults from portable semantics | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-38 | unassigned | Done |
| B-40 | Add explicit runtime target selection for Proxmox import | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-39 | unassigned | Done |
| B-41 | Implement portable-linux runtime normalization for host-only literals | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-39, B-40 | unassigned | Done |
| C-01 | Define hook contract and execution policy | C | Phase-1-Features | epic:hooks, phase:1-features | A-01 | unassigned | Todo |
| C-02 | Implement hook runner service | C | Phase-1-Features | epic:hooks, phase:1-features | C-01 | unassigned | Todo |
| C-03 | Add schema support for hook definitions | C | Phase-1-Features | epic:hooks, phase:1-features | C-01 | unassigned | Todo |
| C-04 | Wire hooks into start and stop flow | C | Phase-1-Features | epic:hooks, phase:1-features | C-02, C-03 | unassigned | Todo |
| C-05 | Hook tests (unit and integration) | C | Phase-1-Features | epic:hooks, phase:1-features | C-04 | unassigned | Todo |
| D-01 | Identify extension seams and trait interfaces | D | Phase-1-Features | epic:extensibility, phase:1-features | A-02 | unassigned | Done |
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
- B-02 -> B-06
- B-02 -> B-07
- B-02 -> B-08
- (B-02 and B-04) -> B-09
- B-02 -> B-10
- B-02 -> B-11
- B-02 -> B-12
- (B-06 and B-07 and B-08 and B-09 and B-10 and B-11 and B-12) -> B-13
- B-10 -> B-14
- B-11 -> B-15
- B-06 -> B-16
- B-02 -> B-17
- B-02 -> B-18
- B-02 -> B-19
- B-02 -> B-20
- B-20 -> B-21
- (B-02 and B-05) -> B-22
- (B-10 and B-19) -> B-23
- B-21 -> B-24
- B-24 -> B-25
- B-25 -> B-26
- B-26 -> B-27
- B-26 -> B-28
- (B-26 and B-27 and B-28) -> B-29
- B-29 -> B-30
- B-23 -> B-31
- B-29 -> B-32
- (B-30 and B-32) -> B-33
- B-29 -> B-34
- B-30 -> B-35
- (B-33 and D-01) -> B-36
- B-36 -> B-37
- (B-36 and B-37) -> B-38
- B-38 -> B-39
- B-39 -> B-40
- (B-39 and B-40) -> B-41
- C-01 -> C-02
- C-01 -> C-03
- (C-02 and C-03) -> C-04 -> C-05
- D-01 -> D-02 -> D-03 -> D-04
- (B-05 and C-05 and D-03) -> E-01 -> E-02 -> E-03

## Usage Notes

- When moving to an external board (GitHub Projects), copy this table directly.
- Keep this file as the fallback source of truth for dependencies and sprint focus.
- Update Assignee and Status fields during planning and standups.
