# Epic Q Completed Backlog (2026)

Date archived: 2026-05-23
Source of truth: this file (doc/planning/backlog lifecycle).

## Completed Tickets

| ID | Title | Completion Date | Notes |
|---|---|---|---|
| Q-01 | Define hierarchy-first Q35 topology schema and normalization | 2026-05-22 | Hierarchy-first machine-layout schema, normalization, and graph validation added |
| Q-02 | Implement dynamic Q35 port and bridge synthesizer | 2026-05-22 | Deterministic on-demand root-port, legacy-island, and UHCI/EHCI synthesis implemented |
| Q-03 | Integrate synthesized topology into portable command-builder path | 2026-05-22 | Portable command-builder and preflight paths aligned to synthesized topology behavior |
| Q-04 | Add tests, snapshots, and docs for dynamic topology behavior | 2026-05-22 | Fixture coverage, deterministic snapshots, and migration/docs updates completed |

## Ticket Definitions

### Q-01 Define hierarchy-first Q35 topology schema and normalization
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-topology, phase:2-hardening
Assignee: unassigned
Dependencies: B-55

Scope:
- Introduce a hierarchy-first machine layout model where parent bus is implied by nesting.
- Add normalization from existing flat placement inputs to canonical tree form.
- Preserve backward compatibility with existing VM/profile shapes.

Acceptance Criteria:
- Hierarchy-first topology schema is parsed and validated.
- Validation rejects invalid topology graphs (duplicate IDs, cycles, broken parent links).
- Existing configs remain valid without requiring migration.

Estimate: 2 days

Completion Notes (2026-05-22):
- Added hierarchy-first machine layout schema under `system.machine_layout` with nested `buses` and `devices` in `src/config/vm_schema/machine_layout.rs`.
- Added flat-node normalization input (`system.machine_layout.nodes`) with normalization into canonical hierarchy-first tree.
- Added topology validation for duplicate IDs, cycle detection, parent-child kind compatibility, and broken parent links.
- Wired validation to run through `VmConfig::canonical_machine_layout()` during config validation.
- Preserved backward compatibility by deriving inferred machine layout from existing flat bus/address placement data when `system.machine_layout` is absent.
- Added coverage in `src/config/tests/machine_layout.rs` for hierarchy parsing, node normalization, and invalid graph rejection cases.

### Q-02 Implement dynamic Q35 port and bridge synthesizer
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-topology, phase:2-hardening
Assignee: unassigned
Dependencies: Q-01

Scope:
- Generate required `ich9-pcie-port-*` root ports from resolved endpoint demand.
- Generate required `pci.N` legacy islands via `pcidmi` only when referenced.
- Generate required `uhci-*` companions only for active EHCI complexes.

Acceptance Criteria:
- Portable Q35 output emits only required root ports, legacy PCI islands, and UHCI companions.
- Generated IDs, addresses, and emission order are deterministic for identical inputs.
- Existing explicit bus/address assignments remain authoritative.

Estimate: 3 days

Completion Notes (2026-05-22):
- Added portable Q35 runtime synthesis in `src/qemu/command_builder/composition.rs` to emit required topology devices directly as `-device` args.
- Implemented deterministic on-demand `ich9-pcie-port-*` root-port emission from resolved host PCI endpoint demand, preserving explicit bus assignments.
- Implemented on-demand legacy PCI island synthesis through `i82801b11-bridge` plus only referenced `pci.N` `pci-bridge` devices.
- Implemented on-demand EHCI/UHCI companion synthesis so `uhci-*` devices are emitted only for active EHCI complexes.
- Kept Proxmox parity behavior unchanged while portable Q35 skips static `ezkvm-q35.cfg` runtime loading in favor of synthesized topology args.
- Added/updated unit and integration coverage for deterministic synthesis and portable Q35 hostpci/root-port behavior.

### Q-03 Integrate synthesized topology into portable command-builder path
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-topology, phase:2-hardening
Assignee: unassigned
Dependencies: Q-02

Scope:
- Route portable Q35 topology resolution through synthesized hierarchy output.
- Keep Proxmox parity path unchanged.
- Align preflight checks with synthesized effective topology.

Acceptance Criteria:
- Portable dry-run command output uses synthesized topology.
- Proxmox parity fixtures remain unchanged.
- Preflight warnings/errors reference synthesized buses and ports correctly.

Estimate: 2 days

Completion Notes (2026-05-22):
- Updated machine-layout rendering to consume effective `-readconfig` arguments from generated QEMU command output, so portable Q35 layout reflects synthesized topology directly.
- Updated portable runtime preflight to allow missing `ezkvm-q35.cfg` when portable Q35 synthesis is active, while preserving strict missing-readconfig failures for parity/static templates.
- Updated QEMU preflight warning checks to treat synthesized portable Q35 as the effective topology source instead of requiring static readconfig-backed bus definitions.
- Added/updated preflight and topology tests to verify synthesized portable behavior and parity/static guard behavior.

### Q-04 Add tests, snapshots, and docs for dynamic topology behavior
Status: Done
Milestone: Phase-2-Hardening
Labels: epic:q35-topology, phase:2-hardening
Assignee: unassigned
Dependencies: Q-03

Scope:
- Add unit/integration coverage for on-demand generation of `ich9-pcie-port-*`, `pci.N`, and `uhci-*`.
- Add deterministic snapshot coverage for repeated runs.
- Document hierarchy-first model and migration guidance.

Acceptance Criteria:
- Tests cover minimal and passthrough-heavy fixtures.
- Snapshot outputs remain stable across repeated runs.
- User/developer docs explain the dynamic synthesis behavior and constraints.

Estimate: 2 days

Completion Notes (2026-05-22):
- Extended portable Q35 integration coverage with a minimal fixture (`12-mixed-storage-buses.conf`) and a passthrough-heavy fixture (`12-portable-q35-hostpci.conf`) to verify on-demand synthesis of root ports, legacy islands, and EHCI/UHCI sets.
- Added deterministic topology snapshot checks across repeated runs for both minimal and passthrough-heavy portable fixtures.
- Updated developer architecture guidance to document hierarchy-first runtime synthesis contract, constraints, and determinism requirements.
- Updated user migration guidance for portable Q35 hierarchy-first synthesis and validation workflow (`import-proxmox`, `validate --show-machine-layout`, `start --dry-run`).

## Completion Detail

Milestone-level completion notes:
- `Q-01` provided schema/validation baseline for hierarchy-first topology.
- `Q-02` and `Q-03` delivered runtime synthesis integration in portable Q35 flows.
- `Q-04` completed regression coverage and documentation for dynamic topology behavior.

Archive note:
- Epic Q has no remaining active continuation tickets.

## Completion Detail

Milestone-level completion notes:
- `Q-01` provided schema/validation baseline for hierarchy-first topology.
- `Q-02` and `Q-03` delivered runtime synthesis integration in portable Q35 flows.
- `Q-04` completed regression coverage and documentation for dynamic topology behavior.

Archive note:
- Epic Q has no remaining active continuation tickets.