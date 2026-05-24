# Epic N Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| N-01 | Define Q35 parity acceptance matrix and invariants | 2026-05-23 | Q35 parity baseline and invariants documented across fixture classes |
| N-02 | Extend qemu-cmd importer with host PCI and storage placement mapping | 2026-05-23 | Host PCI and storage/controller placement mapping expanded with warnings/tests |
| N-03 | Add runtime-target support to qemu-cmd importer | 2026-05-23 | Runtime-target contract threaded through CLI, I/O, mapper, and docs |
| N-04 | Converge Proxmox and qemu-cmd imports on shared Q35 placement planner | 2026-05-23 | Shared Q35 placement planner adopted across both importers |
| N-05 | Add shared placement conflict validator and precedence contract | 2026-05-23 | Shared conflict validator and precedence contract wired into importer validation |
| N-06 | Harden compact export determinism and host-independence contract | 2026-05-22 | Deterministic compact round-trip and host-independence regression coverage hardened |

## Ticket Definitions

### N-01 Define Q35 parity acceptance matrix and invariants
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: M-09

Scope:
- Define acceptance baselines for Proxmox import, qemu-cmd import, and runtime command generation parity for Q35 bus/address assignments.
- Capture representative fixtures and expected deterministic placement outcomes.
- Link this baseline to `doc/planning/epics/implemented/Q35_DEVICE_TREE_PARITY_MODEL.md` as implementation contract.

Acceptance Criteria:
- Baseline matrix covers at least one Linux, one Windows, and one passthrough-heavy fixture.
- Invariants for placement determinism and parity are documented and testable.
- Regression expectations are explicit for compact and canonical output modes.

Estimate: 1 day

Completion Notes (2026-05-23):
- Added N-01 acceptance matrix baseline and invariants to `doc/planning/epics/implemented/Q35_DEVICE_TREE_PARITY_MODEL.md`.
- Baseline matrix now includes Linux, Windows, and passthrough-heavy fixtures across Proxmox import, qemu-cmd import, and runtime command generation parity checks.
- Defined explicit, testable invariants for Q35 bus/address determinism and cross-import parity.
- Added explicit compact/canonical output-mode regression expectations and allowed-difference contract.

### N-02 Extend qemu-cmd importer with host PCI and storage placement mapping
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: N-01

Scope:
- Map qemu-cmd host PCI devices and storage/controller placement details into canonical config shape.
- Preserve explicit source bus/address assignments when present and valid.
- Add structured warnings for unsupported placement/device patterns.

Acceptance Criteria:
- Imported qemu-cmd configs include host PCI and storage/controller placement for representative fixtures.
- Mapping preserves explicit placement fields from source command where representable.
- New fixture and integration tests cover mapped and warning paths.

Estimate: 3 days

Completion Notes (2026-05-23):
- Extended `src/import/qemu_cmd/mapper.rs` to map `-device vfio-pci` into canonical `host.pci` entries, preserving explicit bus/addr/id/multifunction/x-vga/romfile fields when provided.
- Added storage/controller placement mapping for representative qemu-cmd shapes: SCSI/SATA controllers from `-device`, and drive attachment mapping from `-device` plus `-drive`/`-blockdev` sources into canonical `devices.drives` with controller, bus, unit, scsi_id, boot_index, and rotation_rate placement fields.
- Added structured warnings for ambiguous or unsupported storage mapping cases (for example unresolved drive-node references and malformed storage placement values).
- Added mapper and integration coverage to validate host PCI and storage/controller mapping on representative fixtures (`01-wakiza`, `03-felucia-505`) while keeping qemu-cmd import/command-build test flows green.

### N-03 Add runtime-target support to qemu-cmd importer
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: N-02, B-40

Scope:
- Add qemu-cmd import runtime-target selection aligned with existing Proxmox import semantics.
- Thread runtime target through CLI, IO pipeline, and mapper decisions.
- Keep default behavior deterministic and documented.

Acceptance Criteria:
- `import-qemu-cmd` supports runtime target selection for portable-linux and proxmox-parity semantics.
- Integration tests verify runtime-target branching behavior.
- User docs capture flag behavior and expected output differences.

Estimate: 2 days

Completion Notes (2026-05-23):
- Added qemu-cmd runtime-target contract and plumbing across CLI, import I/O options, and mapper execution path (`portable-linux` default, `proxmox-parity` explicit opt-in).
- Added `--runtime-target` support to `import-qemu-cmd` command parsing and command handler output reporting.
- Implemented runtime-target mapping branch for netdev helper paths: `portable-linux` omits Proxmox-specific helper script paths while `proxmox-parity` preserves source parity fields.
- Added mapper and integration coverage to verify runtime-target branching behavior and preserve existing import/validate/command-build flows.
- Updated qemu-cmd user guide with runtime-target semantics, examples, and expected output differences.

### N-04 Converge Proxmox and qemu-cmd imports on shared Q35 placement planner
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: N-03

Scope:
- Extract/import common Q35 placement planner so both importers use one allocation model.
- Align planner behavior with runtime command-builder constraints and deterministic ordering.
- Preserve existing importer boundary rules while sharing importer-agnostic placement logic.

Acceptance Criteria:
- Both importers call shared placement planner module.
- Cross-import parity tests show equivalent placement for overlapping source semantics.
- No regression in existing Proxmox parity fixtures.

Estimate: 3 days

Completion Notes (2026-05-23):
- Extracted shared Q35 placement planner into `src/import/common/q35_placement.rs` with importer-agnostic runtime-target and host PCI root-port allocation contracts.
- Refactored Proxmox importer topology planner (`src/import/proxmox/mapper/topology.rs`) to delegate machine/readconfig handling, legacy bus selection, and host PCI default bus allocation through the shared planner.
- Updated qemu-cmd importer host PCI placement path (`src/import/qemu_cmd/mapper.rs`) to use the shared planner for Q35 portable root-port allocation decisions.
- Added cross-import integration parity coverage in `tests/integration/qemu_cmd_import.rs` verifying overlapping host PCI placement semantics remain aligned between Proxmox and qemu-cmd imports for representative wakiza fixture flow.
- Preserved existing Proxmox import behavior while converging placement decision logic into a single shared planner module.

### N-05 Add shared placement conflict validator and precedence contract
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: N-04

Scope:
- Add pre-export validation for bus/address collisions after profile merge and explicit placement resolution.
- Define and implement precedence contract: explicit placement first, then profile defaults, then allowed normalization.
- Emit actionable validation errors when conflicts are detected.

Acceptance Criteria:
- Conflicting placement assignments fail before export with deterministic diagnostics.
- Precedence rules are codified in code and tests.
- Both importers invoke the shared validator.

Estimate: 2 days

Completion Notes (2026-05-23):
- Added shared placement conflict validator in `src/import/common/q35_placement.rs` that checks merged effective config for PCI bus/address collisions and drive attachment conflicts before export.
- Codified placement precedence contract in importer-common (`explicit > profile-default > normalization`) and applied it in qemu-cmd host PCI placement resolution.
- Wired shared placement validation through `src/import/common/validate.rs`, so both Proxmox and qemu-cmd import pipelines fail early with deterministic, actionable diagnostics.
- Added shared validator unit coverage for precedence ordering and conflict diagnostics.

### N-06 Harden compact export determinism and host-independence contract
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-parity, phase:2-hardening
Assignee: unassigned
Dependencies: N-05

Scope:
- Ensure compact mode omits only semantic defaults guaranteed by active profiles.
- Add round-trip checks for import -> compact YAML -> runtime args determinism.
- Verify host-specific paths stay runtime-resolved and are not persisted into portable topology intent.

Acceptance Criteria:
- Round-trip parity tests pass for representative fixtures across output modes.
- Compact output remains deterministic and replay-safe.
- Docs updated for placement precedence, compaction behavior, and host-specific runtime resolution.

Estimate: 2 days

Completion Notes (2026-05-22):
- Added output-mode hardening regression coverage for both importers to assert compact replay-safety and runtime-argument determinism across repeated imports.
- Added portable target host-independence assertions ensuring compact outputs do not persist Proxmox host runtime paths (for example `/var/run/qemu-server` and `/usr/libexec/qemu-server/*`) into portable topology intent.
- Preserved output-mode runtime equivalence checks (canonical/compact/debug) while extending deterministic compact round-trip guarantees on representative fixtures.

## Completion Detail

Milestone-level completion notes:
- `N-01` to `N-03` established Q35 parity baseline and runtime-target alignment for qemu-cmd import.
- `N-04` and `N-05` converged placement logic into shared planner/validator contracts.
- `N-06` finalized deterministic compact-export and host-independence hardening.

Archive note:
- Epic N has no remaining active continuation tickets.

## Completion Detail

Milestone-level completion notes:
- `N-01` through `N-03` established Q35 parity baseline and runtime-target alignment for qemu-cmd import.
- `N-04` and `N-05` converged placement logic into shared planner/validator contracts.
- `N-06` finalized deterministic compact-export and host-independence hardening.

Archive note:
- Epic N has no remaining active continuation tickets.