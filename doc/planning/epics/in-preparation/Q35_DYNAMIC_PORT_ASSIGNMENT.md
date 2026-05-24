# Q35 Dynamic Port Assignment

Status: In Preparation
Date: 2026-05-12
Related backlog items: Q-01, Q-02, Q-03, Q-04
Replaces planning scope of: B-56

## Problem Statement

Portable Q35 mode currently relies on a static readconfig topology (`/usr/share/ezkvm/ezkvm-q35.cfg`) that predeclares a fixed set of:

- `ich9-pcie-port-*` root ports
- legacy PCI bridge buses (`pci.0..pci.3`)
- ICH9 USB companion controllers (`uhci-*` under `ehci` / `ehci-2`)

This guarantees compatibility but over-defines ports that some guests never use. For passthrough-heavy or topology-sensitive guests, this can expose unnecessary devices and make runtime shape less intentional than required.

## Goal

Generate required Q35 topology elements on demand from a hierarchy-first machine layout model, while preserving deterministic naming and guest-visible behavior.

Specifically:

- Synthesize only required `ich9-pcie-port-*` ports.
- Synthesize only required legacy PCI islands (`pci.N`) when devices are placed there.
- Synthesize only required `uhci-*` companions for active EHCI complexes.
- Keep deterministic, repeatable output for identical resolved configs.
- Keep Proxmox parity behavior unchanged unless explicitly routed through the new path.

## Hierarchy-First Model

Parent bus is implied by tree position, not repeated on every leaf.

Example shape (abbreviated):

```yaml
system:
  machine_layout:
    buses:
      - id: pcie.0
        devices:
          - id: ich9-pcie-port-1
            driver: pcie-root-port
            addr: "1c.0"

          - id: ehci
            driver: ich9-usb-ehci1
            addr: "1d.7"
            buses:
              - id: ehci.0
                devices:
                  - id: uhci1
                    driver: ich9-usb-uhci1
                    addr: "1d.0"

          - id: pcidmi
            driver: i82801b11-bridge
            addr: "1e.0"
            buses:
              - id: pci.0
                devices:
                  - id: pci.0
                    driver: pci-bridge
                    addr: "1.0"
```

This aligns with `ezkvm validate --show-machine-layout` output where topology is naturally represented as a tree.

## Dynamic Generation Strategy

### 1) `ich9-pcie-port-*` generation

Inputs:

- Effective endpoint placement requests that need root ports (for example `hostpci` pcie endpoints without explicit bus, or explicit requests for `ich9-pcie-port-*`).

Algorithm:

- Compute required root-port count from resolved endpoint demand.
- Allocate sequentially and deterministically:
  - IDs: `ich9-pcie-port-1..N`
  - addresses: `1c.0`, `1c.1`, `1c.2`, `1c.3`, then continue with next available deterministic slot policy.
- Emit only the first `N` ports.
- Preserve stable mapping order by sorted/declared endpoint order.

### 2) `pci.N` island generation

Inputs:

- Effective device placements referencing legacy buses (`pci.0`, `pci.1`, ...).

Algorithm:

- Build required bus index set from resolved placements.
- Ensure `pcidmi` (`i82801b11-bridge`) is emitted when set is non-empty.
- Emit `pci-bridge` devices only for referenced buses.
- Deterministic mapping:
  - bus `pci.N` <-> bridge id `pci.N`
  - parent `pcidmi`
  - deterministic chassis numbering from index.

### 3) `uhci-*` generation

Inputs:

- Presence of USB devices requiring ICH9 EHCI complexes.
- Optional explicit requests for EHCI complex selection.

Algorithm:

- Emit EHCI complex only when needed.
- For each active EHCI complex, emit the required UHCI companions only.
- Keep deterministic naming and firstport mapping:
  - `uhci1/2/3` under `ehci.0`
  - `uhci-4/5/6` under `ehci-2.0`
- If no USB devices resolve to an EHCI complex, emit no companion UHCI devices.

## Determinism Contract

For the same resolved VM configuration:

- generated topology node IDs are identical;
- parent-child links are identical;
- emitted `-device` order is identical;
- address assignment and naming are stable across runs.

## Compatibility Rules

- Proxmox parity mode remains on existing readconfig behavior until explicitly migrated.
- Portable mode can opt into dynamic hierarchy synthesis.
- Existing explicit bus/address values in VM config remain authoritative.
- Preflight warnings continue to guard references to buses not synthesized by the final topology.

## Implementation Plan

### Q-01 Define hierarchy layout schema and normalization

- Introduce a topology tree schema where parent bus is implied by nesting.
- Add normalization pass from existing flat-style placement into tree form.
- Keep backward compatibility with existing config shapes.

Acceptance:

- Schema parses hierarchy-first topology profiles.
- Validation rejects cycles/duplicate IDs/invalid parent-child relationships.
- Existing configs remain valid.

### Q-02 Implement dynamic topology synthesizer

- Build planner that computes required `ich9-pcie-port-*`, `pci.N`, and `uhci-*` sets.
- Emit topology nodes from resolved device placement demand.
- Separate fixed chipset anchors (for example `pcidmi`) from on-demand leaves.

Acceptance:

- Unused ports are not emitted.
- Required ports are emitted with deterministic IDs and addresses.
- No regressions in guest-visible placement semantics for covered fixtures.

### Q-03 Wire command builder and importer integration

- Route portable Q35 path through synthesizer output.
- Preserve parity-mode path unchanged.
- Update placement helpers/preflight checks to consume synthesized topology.

Acceptance:

- Portable Q35 dry-run output uses synthesized topology.
- Parity path remains unchanged for existing snapshots.
- Mixed explicit/implicit placement rules behave deterministically.

### Q-04 Add regression coverage and docs

- Add unit and integration tests for dynamic synthesis counts and naming stability.
- Add matrix cases for passthrough-heavy and minimal VMs.
- Update user/dev docs with hierarchy-first model and troubleshooting.

Acceptance:

- Tests cover root-port, pci-island, and UHCI on-demand emission.
- Repeated runs produce identical output snapshots.
- Documentation explains migration and compatibility behavior.

## Risks and Mitigations

- Risk: accidental guest-visible topology drift.
  - Mitigation: fixture snapshots and explicit invariants in tests.
- Risk: mismatch between preflight expectations and synthesized buses.
  - Mitigation: derive preflight validation from final synthesized graph.
- Risk: parity workflow regressions.
  - Mitigation: keep parity path isolated and test-gated.

## Out of Scope

- Broad replacement of Proxmox parity readconfig assets.
- Non-Q35 machine topology redesign.
- Runtime hotplug policy changes.
