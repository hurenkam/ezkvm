# Epic B Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth during transition: `doc/backlog/BACKLOG.md`, `doc/backlog/TRACKING_BOARD.md`

## Completed Tickets

| ID | Title |
|---|---|
| B-01 | Port Proxmox parser into dedicated module |
| B-02 | Add canonical mapper (intermediate model to current schema) |
| B-03 | Add import CLI command |
| B-04 | Add import validation and reporting |
| B-05 | Integration tests for import-to-start pipeline |
| B-06 | Parse and map efidisk0 for UEFI vars parity |
| B-07 | Parse and map audio0 for SPICE HDA parity |
| B-08 | Parse and map agent field for guest-agent plumbing |
| B-09 | Parse and map args passthrough safe subset |
| B-10 | Preserve machine and CPU feature fidelity |
| B-11 | Improve network backend fidelity for Proxmox bridge/tap |
| B-12 | Expand host PCI fidelity for multifunction devices |
| B-13 | Add wakiza parity fixture and regression test |
| B-14 | Improve CPU and Hyper-V fidelity for Windows guests |
| B-15 | Expand network device fidelity (virtio-net-pci placement and queues) |
| B-16 | Refine firmware and secure-boot mapping from efidisk metadata |
| B-17 | Add SATA support |
| B-18 | Serial port configuration |
| B-19 | Support IOMMU/vIOMMU device definitions |
| B-20 | Full mapping of hugepages settings |
| B-21 | Materialize hugepages profile layer end-to-end |
| B-22 | Add canonical VNC profile schema and headless-vnc layer |
| B-23 | Add viommu and hidden-hypervisor tuning profile assignment |
| B-24 | Tighten profile-aware compaction ownership boundaries |
| B-25 | Expand profile-stack corpus and edge-case coverage |
| B-26 | Decompose mapper orchestration and module boundaries |
| B-27 | Split profile-aware compaction by domain ownership |
| B-28 | Restrict Proxmox importer public surface to high-level API |
| B-29 | Add temporary over-size rationale and cleanup guardrails |
| B-30 | Audit profile-first compaction implementation status |
| B-31 | Expand profile inference coverage for Proxmox importer |
| B-32 | Omit deterministic fields in import-output mode |
| B-33 | Introduce explicit export modes (canonical, compact, debug-canonical) |
| B-34 | Extend compaction policies for repeated field omission |
| B-35 | Update schema to attach drives to controllers which belong to devices |
| B-36 | Add central host capability schema for portable runtime |
| B-37 | Define and implement runtime precedence contract |
| B-38 | Add portability preflight validation and error model |
| B-39 | Separate parity-only defaults from portable semantics |
| B-40 | Add explicit runtime target selection for Proxmox import |
| B-41 | Implement portable-linux runtime normalization for host-only literals |
| B-42 | Extend central config schema with host capability sections |
| B-43 | Implement runtime directory capability provider |
| B-44 | Implement firmware locator capability provider |
| B-45 | Implement swtpm capability provider |
| B-46 | Implement network backend helper capability provider |
| B-47 | Add optional Looking Glass capability provider |
| B-48 | Implement capability precedence contract and validation |
| B-49 | Add integration tests for host capability resolution |
| B-50 | Document portable mode operator guidance |
| B-51 | Define Phase 3 distro validation matrix and success criteria |
| B-52 | Build reusable Phase 3 validation harness |
| B-53 | Execute Debian Trixie portable-runtime validation matrix |
| B-54 | Execute Ubuntu 26.04 portable-runtime validation matrix |