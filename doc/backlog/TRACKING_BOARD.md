# Convergence Tracking Board

Date: 2026-05-21  
Scope: Epics A-E, K, M, Q (Phase 0 to Phase 3)  
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
- epic:packaging
- epic:q35-topology
- epic:qemu-cmd-import
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
- Phase-3-Packaging

## Sprint 1 Focus

- A-01 Create convergence ADR set
- A-02 Define target module ownership map
- A-03 Create convergence tracking board
- B-01 Port Proxmox parser into dedicated module (start)

## Ticket Registry

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
| B-30 | Audit profile-first compaction implementation status | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-29 | unassigned | Done |
| B-31 | Expand profile inference coverage for Proxmox importer | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-23 | unassigned | Done |
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
| B-42 | Extend central config schema with host capability sections | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-41 | unassigned | Done |
| B-43 | Implement runtime directory capability provider | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-42 | unassigned | Done |
| B-44 | Implement firmware locator capability provider | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-43 | unassigned | Done |
| B-45 | Implement swtpm capability provider | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-44 | unassigned | Done |
| B-46 | Implement network backend helper capability provider | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-45 | unassigned | Done |
| B-47 | Add optional Looking Glass capability provider | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-46 | unassigned | Done |
| B-48 | Implement capability precedence contract and validation | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-47 | unassigned | Done |
| B-49 | Add integration tests for host capability resolution | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-48 | unassigned | Done |
| B-50 | Document portable mode operator guidance | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-49 | unassigned | Done |
| B-51 | Define Phase 3 distro validation matrix and success criteria | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening, ready | B-50 | unassigned | Done |
| B-52 | Build reusable Phase 3 validation harness | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-51 | unassigned | Done |
| B-53 | Execute Debian Trixie portable-runtime validation matrix | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-52 | unassigned | Done |
| B-54 | Execute Ubuntu 26.04 portable-runtime validation matrix | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-52 | unassigned | Done |
| B-55 | Execute Arch Linux portable-runtime validation matrix and publish runbooks | B | Phase-2-Hardening | epic:proxmox, phase:2-hardening | B-53, B-54 | unassigned | Todo |
| Q-01 | Define hierarchy-first Q35 topology schema and normalization | Q | Phase-2-Hardening | epic:q35-topology, phase:2-hardening | B-55 | unassigned | Todo |
| Q-02 | Implement dynamic Q35 port and bridge synthesizer | Q | Phase-2-Hardening | epic:q35-topology, phase:2-hardening | Q-01 | unassigned | Todo |
| Q-03 | Integrate synthesized topology into portable command-builder path | Q | Phase-2-Hardening | epic:q35-topology, phase:2-hardening | Q-02 | unassigned | Todo |
| Q-04 | Add tests, snapshots, and docs for dynamic topology behavior | Q | Phase-2-Hardening | epic:q35-topology, phase:2-hardening | Q-03 | unassigned | Todo |
| M-01 | Define qemu-cmd importer boundary and public contracts | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | B-40 | unassigned | Done |
| M-02 | Extract importer-common orchestration helpers | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-01 | unassigned | Done |
| M-03 | Implement qemu-cmd parser and intermediate model | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-01 | unassigned | Done |
| M-04 | Implement qemu-cmd mapper and warning taxonomy | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-03 | unassigned | Done |
| M-05 | Implement qemu-cmd import I/O pipeline | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-02, M-04 | unassigned | Done |
| M-06 | Add import-qemu-cmd CLI command | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-05 | unassigned | Done |
| M-07 | Add qemu-cmd fixtures and integration coverage | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-06 | unassigned | Done |
| M-08 | Publish standalone qemu-cmd import documentation | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-06 | unassigned | Done |
| M-09 | Harden qemu-cmd profile inference and parity assertions | M | Phase-2-Hardening | epic:qemu-cmd-import, phase:2-hardening | M-07 | unassigned | Done |
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
| E-04 | Promote shutdown monitor into a VM-scoped lifecycle supervisor | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | None | unassigned | In Progress |
| E-05 | Reconcile guest shutdown state with QMP and PID state | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | E-04 | unassigned | In Progress |
| E-06 | Add shutdown lifecycle regression tests and traces | E | Phase-2-Hardening | epic:hardening, phase:2-hardening | E-05 | unassigned | Todo |
| K-01 | Build packaging baseline inventory and contract | K | Phase-3-Packaging | epic:packaging, phase:2-hardening | B-54 | unassigned | Done |
| K-02 | Create Debian packaging skeleton for ezkvm | K | Phase-3-Packaging | epic:packaging, phase:2-hardening | K-01 | unassigned | Done |
| K-03 | Harden dependency policy for cross-distro installability | K | Phase-3-Packaging | epic:packaging, phase:2-hardening, ready | K-02 | unassigned | Todo |
| K-04 | Execute dual-distro package validation matrix | K | Phase-3-Packaging | epic:packaging, phase:2-hardening | K-03 | unassigned | Todo |
| K-05 | Add packaging CI and release gate enforcement | K | Phase-3-Packaging | epic:packaging, phase:2-hardening | K-04 | unassigned | Todo |
| K-06 | Package non-root runtime group and directory ownership policy | K | Phase-3-Packaging | epic:packaging, phase:2-hardening | K-03 | unassigned | Todo |

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
- B-41 -> B-42 -> B-43 -> B-44 -> B-45 -> B-46 -> B-47 -> B-48 -> B-49 -> B-50
- B-50 -> B-51 -> B-52
- B-52 -> B-53
- B-52 -> B-54
- (B-53 and B-54) -> B-55
- B-55 -> Q-01 -> Q-02 -> Q-03 -> Q-04
- B-40 -> M-01
- M-01 -> M-02
- M-01 -> M-03 -> M-04
- (M-02 and M-04) -> M-05 -> M-06
- M-06 -> M-07 -> M-09
- M-06 -> M-08
- C-01 -> C-02
- C-01 -> C-03
- (C-02 and C-03) -> C-04 -> C-05
- D-01 -> D-02 -> D-03 -> D-04
- (B-05 and C-05 and D-03) -> E-01 -> E-02 -> E-03
- E-04 -> E-05 -> E-06
- B-54 -> K-01 -> K-02 -> K-03 -> K-04 -> K-05
- K-03 -> K-06

## Usage Notes

- When moving to an external board (GitHub Projects), copy this table directly.
- Keep this file as the fallback source of truth for dependencies and sprint focus.
- Update Assignee and Status fields during planning and standups.
