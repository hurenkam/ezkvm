# Epic B Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| B-01 | Port Proxmox parser into dedicated module | 2026-05-23 | Importer parity baseline sequence completed |
| B-02 | Add canonical mapper (intermediate model to current schema) | 2026-05-23 | Importer parity baseline sequence completed |
| B-03 | Add import CLI command | 2026-05-23 | Importer parity baseline sequence completed |
| B-04 | Add import validation and reporting | 2026-05-23 | Importer parity baseline sequence completed |
| B-05 | Integration tests for import-to-start pipeline | 2026-05-23 | Importer parity baseline sequence completed |
| B-06 | Parse and map efidisk0 for UEFI vars parity | 2026-05-23 | Importer parity baseline sequence completed |
| B-07 | Parse and map audio0 for SPICE HDA parity | 2026-05-23 | Importer parity baseline sequence completed |
| B-08 | Parse and map agent field for guest-agent plumbing | 2026-05-23 | Importer parity baseline sequence completed |
| B-09 | Parse and map args passthrough safe subset | 2026-05-23 | Importer parity baseline sequence completed |
| B-10 | Preserve machine and CPU feature fidelity | 2026-05-23 | Importer parity baseline sequence completed |
| B-11 | Improve network backend fidelity for Proxmox bridge/tap | 2026-05-23 | Importer parity baseline sequence completed |
| B-12 | Expand host PCI fidelity for multifunction devices | 2026-05-23 | Importer parity baseline sequence completed |
| B-13 | Add wakiza parity fixture and regression test | 2026-05-23 | Importer parity baseline sequence completed |
| B-14 | Improve CPU and Hyper-V fidelity for Windows guests | 2026-05-23 | Importer parity baseline sequence completed |
| B-15 | Expand network device fidelity (virtio-net-pci placement and queues) | 2026-05-23 | Importer parity baseline sequence completed |
| B-16 | Refine firmware and secure-boot mapping from efidisk metadata | 2026-05-23 | Importer parity baseline sequence completed |
| B-17 | Add SATA support | 2026-05-23 | Importer parity baseline sequence completed |
| B-18 | Serial port configuration | 2026-05-23 | Importer parity baseline sequence completed |
| B-19 | Support IOMMU/vIOMMU device definitions | 2026-05-23 | Importer parity baseline sequence completed |
| B-20 | Full mapping of hugepages settings | 2026-05-23 | Importer parity baseline sequence completed |
| B-21 | Materialize hugepages profile layer end-to-end | 2026-05-23 | Importer parity baseline sequence completed |
| B-22 | Add canonical VNC profile schema and headless-vnc layer | 2026-05-23 | Importer parity baseline sequence completed |
| B-23 | Add viommu and hidden-hypervisor tuning profile assignment | 2026-05-23 | Importer parity baseline sequence completed |
| B-24 | Tighten profile-aware compaction ownership boundaries | 2026-05-23 | Importer parity baseline sequence completed |
| B-25 | Expand profile-stack corpus and edge-case coverage | 2026-05-23 | Importer parity baseline sequence completed |
| B-26 | Decompose mapper orchestration and module boundaries | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-27 | Split profile-aware compaction by domain ownership | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-28 | Restrict Proxmox importer public surface to high-level API | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-29 | Add temporary over-size rationale and cleanup guardrails | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-30 | Audit profile-first compaction implementation status | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-31 | Expand profile inference coverage for Proxmox importer | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-32 | Omit deterministic fields in import-output mode | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-33 | Introduce explicit export modes (canonical, compact, debug-canonical) | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-34 | Extend compaction policies for repeated field omission | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-35 | Update schema to attach drives to controllers which belong to devices | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-36 | Add central host capability schema for portable runtime | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-37 | Define and implement runtime precedence contract | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-38 | Add portability preflight validation and error model | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-39 | Separate parity-only defaults from portable semantics | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-40 | Add explicit runtime target selection for Proxmox import | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-41 | Implement portable-linux runtime normalization for host-only literals | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-42 | Extend central config schema with host capability sections | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-43 | Implement runtime directory capability provider | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-44 | Implement firmware locator capability provider | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-45 | Implement swtpm capability provider | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-46 | Implement network backend helper capability provider | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-47 | Add optional Looking Glass capability provider | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-48 | Implement capability precedence contract and validation | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-49 | Add integration tests for host capability resolution | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-50 | Document portable mode operator guidance | 2026-05-23 | Hardening and capability-contract sequence completed |
| B-51 | Define Phase 3 distro validation matrix and success criteria | 2026-05-23 | Phase 3 cross-distro validation sequence completed |
| B-52 | Build reusable Phase 3 validation harness | 2026-05-23 | Phase 3 cross-distro validation sequence completed |
| B-53 | Execute Debian Trixie portable-runtime validation matrix | 2026-05-23 | Phase 3 cross-distro validation sequence completed |
| B-54 | Execute Ubuntu 26.04 portable-runtime validation matrix | 2026-05-23 | Phase 3 cross-distro validation sequence completed |

## Completion Detail

Completion status summary:
- Epic B tickets `B-01` through `B-54` are completed.
- Remaining follow-up ticket `B-55` remains in active planning at `doc/planning/backlog/active/B.md`.

Milestone-level completion notes:
- `B-01` to `B-25`: established importer parity baseline and profile-aware import/output behavior for Proxmox-backed workflows.
- `B-26` to `B-50`: completed hardening/refactor sequence including ownership boundaries, export-mode semantics, runtime-target contracts, capability providers, and host-resolution tests/docs.
- `B-51` to `B-54`: completed cross-distro validation preparation and execution evidence for Debian Trixie and Ubuntu 26.04 matrix runs.

Archive note:
- This archive keeps the full completed ticket index for Epic B while preserving the open continuation ticket (`B-55`) in active backlog.