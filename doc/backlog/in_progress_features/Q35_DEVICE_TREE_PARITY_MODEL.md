# Q35 Device Tree Parity Model

## Summary

Define and implement a deterministic internal Q35 device tree model in ezkvm that mirrors Proxmox QEMU behavior for bus and address placement across both import and runtime paths.

This feature aligns Proxmox config import and qemu-cmd import so both produce the same explicit placement model, then ensures runtime generation reproduces that model on any host while resolving host-specific paths at runtime.

## Motivation

Current behavior has improved with the qemu-cmd importer, but there is still a parity gap:

- Proxmox import and qemu-cmd import do not yet share a single Q35 placement core.
- qemu-cmd import still needs full topology-aware device extraction and runtime-target parity semantics.
- Placement determinism and profile compaction must be contract-driven to avoid hidden defaults and host drift.

Goal: one explicit internal model for placement, portable YAML output, deterministic runtime reproduction.

## Scope

- Analyze and codify Proxmox QEMU Q35 placement behavior and ezkvm parity rules.
- Make qemu-cmd import topology-aware for host PCI and storage/controller placement.
- Add runtime-target parity semantics to qemu-cmd import.
- Converge both importers on one shared Q35 placement and validation layer.
- Define assignment precedence and compact export contract so exported YAML remains deterministic and host-independent.
- Preserve host-specific path resolution as runtime concern.

## Non-Goals

- Replacing the existing runtime resolver with import-time host path hardcoding.
- Immediate full schema rewrite if parity can be reached through shared placement and validator convergence.
- Broad redesign of unrelated device models.

## Problem Statement

For reliable portability and reproducibility, ezkvm needs a single, explicit Q35 topology model where each imported or configured device has deterministic placement semantics.

Desired behavior:

1. Import phase
- Import from qemu-cmd and Proxmox config should assign explicit placement for each relevant device according to the same Q35 model rules.
- Import should support target semantics that reflect portable-linux and proxmox-parity behavior.

2. Save phase
- Compact YAML should remove only semantic defaults guaranteed by active profiles.
- Explicit placement needed for deterministic replay must not be lost.

3. Runtime phase
- YAML plus profiles reconstructs the same explicit internal Q35 model.
- Runtime resolver applies host-specific paths and host runtime details without changing placement intent.
- Generated QEMU command reproduces the same device tree on target host.

## Design Principles

1. Single placement authority
- Importers must share one placement core for Q35 assignment logic.

2. Explicit-over-implicit contract
- Imported explicit bus and address values take precedence unless invalid.

3. Deterministic replay
- Same input and same profiles produce same internal placement and equivalent runtime args.

4. Host-independent persisted config
- Persisted YAML captures topology intent, not host filesystem details.

5. Profile-driven compactness
- No hidden hardcoded defaults in the model; compaction relies on profile semantic defaults.

## Proposed Architecture

### A. qemu-cmd importer parity uplift

- Extend qemu-cmd mapper to extract host PCI and storage/controller topology details needed for Q35 placement.
- Add runtime-target support to qemu-cmd import path matching Proxmox import semantics.
- Preserve explicit placement from source command when present.

### B. Shared import placement core

- Refactor Q35 placement logic into shared import/common module.
- Use shared allocator/planner in both Proxmox and qemu-cmd importers.
- Add shared placement conflict validator called before final rendering.

### C. Placement precedence and compact contract

Define clear precedence:

1. explicit placement from source or VM config
2. profile placement defaults
3. runtime normalization only where contract explicitly permits

Compaction rule:

- Strip values only when semantically guaranteed by active profiles and preserving deterministic replay.

### D. Runtime host abstraction

- Keep host-specific path resolution in runtime resolver and host config.
- Optionally add host-selector ergonomics for qemu-cmd import only if needed, without embedding host paths in persisted YAML.

## Implementation Phases

### Phase 1: Baseline and acceptance matrix

- Capture current importer and runtime behavior for representative fixtures.
- Define explicit parity checks and determinism criteria.

### N-01 Baseline Acceptance Matrix and Invariants

This section defines the Phase 1 acceptance baseline for Q35 parity and determinism.

#### Representative Fixture Matrix

| Workload class | Source fixture | Import path | Output modes | Baseline parity expectations |
|---|---|---|---|---|
| Linux | `tests/fixtures/proxmox_import/12-mixed-storage-buses.conf` | Proxmox import | canonical, compact | Imported storage/controller placement is deterministic; runtime args preserve effective Q35 bus/address intent across repeated runs. |
| Windows | `tests/fixtures/qemu_cmd_import/01-wakiza.qemu.cmd` | qemu-cmd import | canonical, compact | Imported machine/device placement is deterministic; repeated import + command build yields stable Q35 placement semantics. |
| Passthrough-heavy | `tests/fixtures/proxmox_import/12-portable-q35-hostpci.conf` and `tests/fixtures/qemu_cmd_import/03-felucia-505.qemu.cmd` | Proxmox import and qemu-cmd import | canonical, compact | Host PCI placement intent is preserved when representable; generated runtime args retain deterministic bus/address outcomes for passthrough-relevant devices. |

#### Testable Invariants

1. Deterministic import invariant
- For a fixed source fixture and runtime-target, repeated import runs produce semantically equivalent placement assignments.

2. Deterministic runtime invariant
- For a fixed imported config and profile set, repeated dry-run command generation produces equivalent Q35 bus/address placement outcomes.

3. Cross-import parity invariant
- Where Proxmox and qemu-cmd inputs encode overlapping topology intent, normalized placement intent must match after import.

4. Explicit placement precedence invariant
- Explicit source placement values (bus/address) are preserved unless invalid by contract.

5. Warning stability invariant
- Output-mode differences do not change warning meaning or placement semantics.

#### Compact vs Canonical Regression Expectations

1. Canonical mode
- Must emit all explicit placement fields required to reconstruct the same Q35 intent without profile assumptions.

2. Compact mode
- May omit only semantic defaults guaranteed by active profiles.
- Must not omit values whose removal changes deterministic replay or placement intent.

3. Allowed differences between modes
- Field omission where the omitted value is profile-guaranteed.
- Ordering/formatting differences that do not alter reconstructed placement semantics.

4. Disallowed differences between modes
- Any change in resolved Q35 bus/address outcomes after round-trip import and runtime build.
- Any change in warning class/meaning for the same fixture and target.

### Phase 2: qemu-cmd parity completion

- Add runtime-target support to qemu-cmd import CLI and pipeline.
- Implement host PCI and storage/controller placement mapping in qemu-cmd importer.
- Ensure explicit placement preservation and stable IDs.

### Phase 3: Shared placement convergence

- Extract shared Q35 placement planner for both importers.
- Add shared pre-export conflict validator.
- Align importer and runtime root-port policy or document/test intentional divergence.

### Phase 4: Compact/profile determinism hardening

- Implement and test precedence contract.
- Ensure compact export strips only safe semantic defaults.
- Add round-trip determinism tests.

### Phase 5: Optional advanced topology synthesis

- Reassess need for hierarchy-first schema and dynamic synthesis after shared convergence.
- If needed, implement behind feature gate with compatibility normalization.

## Required Code Areas

- src/cli/commands/import_qemu_cmd.rs
- src/import/qemu_cmd/io.rs
- src/import/qemu_cmd/mapper.rs
- src/import/proxmox/mapper/topology.rs
- src/import/common/mod.rs and new shared placement modules
- src/qemu/command_builder/composition.rs
- src/state/runtime_resolver.rs
- etc/profiles.d/proxmox-base.yaml
- etc/profiles.d/proxmox-portable-q35.yaml

## Validation Plan

1. Intake baseline
- cargo fmt --all --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --quiet

2. Importer parity tests
- qemu-cmd runtime-target branching
- qemu-cmd host PCI mapping
- qemu-cmd storage/controller mapping

3. Cross-import consistency
- Proxmox config import and qemu-cmd import yield equivalent placement for overlapping source data.

4. Round-trip determinism
- import to compact YAML to runtime args preserves placement deterministically.

5. Conflict detection
- explicit placement and profile-placement collisions fail before export.

6. Exit validation
- re-run fmt, clippy, test and report delta versus intake baseline.

## Documentation Impact

Update in same task as behavior changes:

- doc/dev/adr/ADR-0005-q35-topology-contract.md
- user docs for import behavior, placement precedence, and compact semantics
- importer command docs reflecting runtime-target and placement behavior

## Backlog and Tracking Requirements

When this feature is picked up:

- Ensure related backlog tickets and tracking-board registry/dependency/status are synced in the same task.
- Keep this feature doc in `in_progress_features` while linked tickets are active.
- Move to implemented_features when all linked tickets are complete.

## Risks and Mitigations

1. Divergence between import-time and runtime placement policies
- Mitigation: shared planner and shared validator, parity snapshot tests.

2. Over-compaction removing required explicit placement
- Mitigation: contract-driven compaction tests with replay assertions.

3. Host path leakage into portable YAML
- Mitigation: strict separation of placement model and runtime resolver responsibilities.

## Exit Criteria

- qemu-cmd import supports runtime-target parity semantics and topology-aware host PCI plus storage mapping.
- Both importers use shared Q35 placement logic and shared conflict validation.
- Placement precedence and compaction contract are implemented and documented.
- Cross-import and round-trip determinism tests are green.
- User-facing docs and backlog tracking are synchronized.
