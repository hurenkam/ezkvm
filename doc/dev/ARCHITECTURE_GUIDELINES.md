# Architecture Guidelines

This document defines architecture best practices for this repository. It complements `doc/dev/CODING_GUIDELINES.md` by describing how to organize code and components at the system level.

## 1. Code Architecture

### Goal
Keep the codebase discoverable, modular, and testable by using a predictable filesystem layout and consistent placement of source, tests, and documentation.

### Canonical Layout

```text
.
├── src/
│   ├── main.rs                 # binary entrypoint
│   ├── lib.rs                  # crate root and module exports
│   ├── cli/                    # CLI parsing and command handlers
│   ├── config/                 # schema, loading, merge, validation
│   ├── qemu/                   # command construction and process control
│   ├── network/                # network helpers and tooling
│   ├── storage/                # storage operations and helpers
│   ├── state/                  # runtime state, pid/log paths, cache
│   └── device.rs               # hotplug and passthrough helpers
├── tests/
│   ├── integration_tests.rs    # integration test entrypoint
│   ├── integration/            # integration test modules
│   ├── config_tests.rs         # config test entrypoint
│   └── config/                 # config-focused test modules
├── examples/                   # runnable and documented sample configs
├── etc/                        # environment-specific config artifacts
├── input/                      # fixtures and external reference inputs
├── notes/                      # design notes and planning docs
├── README.md                   # project overview and usage
├── doc/
│   ├── user/CONFIG.md          # user-facing config contract
│   └── dev/
│       ├── CODING_GUIDELINES.md
│       └── ARCHITECTURE_GUIDELINES.md
```

### Placement Rules
- Put production Rust code under `src/` only.
- Keep module entrypoints (`mod.rs` where used) as wiring-only files.
- Place integration tests in `tests/` with one entrypoint per test family and module files in subdirectories.
- Keep long-term reference docs in root or `notes/`; keep generated, obsolete, or temporary artifacts out of primary docs paths.
- Place user-consumable examples in `examples/`; place external comparison fixtures under `input/`.

### Naming and File Granularity
- Use domain-based module names (`config`, `qemu`, `state`) over technical names (`utils`, `helpers`) when possible.
- Prefer focused files with one primary responsibility.
- Split files that become hard to navigate; use orchestrator modules to compose focused submodules.

### Test Location Strategy
- Unit tests live next to source modules when they verify local behavior.
- Integration tests live under `tests/` when they validate cross-component behavior.
- Use fixture files for parity tests rather than embedding large command strings inline.

## 2. System Architecture

### Goal
Use a modular architecture with loose interfaces and explicit data contracts so that core behavior can evolve without tight coupling.

### Recommended Pattern
Use a layered modular monolith pattern with ports-and-adapters influence:
- Core/domain modules express VM configuration, validation, and command intent.
- Adapter modules implement host interactions (QEMU process, filesystem, network tools, QMP socket I/O).
- CLI is an application boundary that translates user intent into domain operations.

### Component Model
- `cli`: command parsing and orchestration entrypoint.
- `config`: schema types, loading, profile merge, and validation.
- `qemu`: command building and process lifecycle management.
- `device`: runtime hotplug/passthrough orchestration (QMP-based operations).
- `storage`: disk and snapshot operations.
- `network`: bridge/firewall/stats tooling.
- `state`: runtime metadata and path conventions.

### Interaction Principles
- Prefer unidirectional flow:
  - `cli` -> `config` -> `qemu`/`device`/`storage`/`network` -> OS/QEMU.
- Keep interfaces coarse enough to be stable, but explicit enough to validate inputs.
- Use typed config structs and strongly typed helper functions instead of map-like dynamic passing.
- Return explicit `Result` values with actionable context at component boundaries.

### Loose Interface Rules
- Do not let low-level adapters call upward into CLI or command parser modules.
- Keep serialization/deserialization concerns inside config boundary modules.
- Isolate side effects (process execution, socket I/O, file mutation) behind narrow function interfaces.
- For protocol adapters (for example QMP), centralize framing, handshake, and error parsing in one client abstraction.

### Reliability Patterns
- Apply fail-fast validation before runtime execution.
- Use deterministic command generation for regression comparability.
- Handle external command/protocol errors as structured failures, not best-effort logs.

### Proxmox Runtime Parity Invariants
- For Proxmox imports, preserve host/runtime integration anchors that Proxmox tooling depends on:
  - tap naming format (`tap<vmid>i<net-index>`)
  - pid file path (`/var/run/qemu-server/<vmid>.pid`)
  - runtime socket paths under `/var/run/qemu-server/`
- Preserve stable PCI topology for imported devices when Proxmox behavior depends on it (for example network and guest-agent controller placement).
- Keep guest-agent and SPICE/vdagent serial topology compatible with Proxmox to avoid input-channel regressions in Looking Glass workflows.
- Treat dry-run parity against captured Proxmox command lines as a required architecture-level regression check for import changes.

### Q35 PCIe and Legacy PCI Topology Contract

For `q35` machine types, treat topology as a compatibility contract rather than only a generated command detail.

- Default to a PCIe-first layout: place PCIe-capable devices behind `pcie-root-port` or PCIe downstream ports.
- Keep PCIe and legacy PCI hierarchies separated: use `pcie-pci-bridge` plus `pci-bridge` for legacy PCI device islands.
- Keep topology flat by default: prefer root-port fanout to deep switch trees unless bus-count constraints require switches.
- Preserve imported guest-visible slot identity for sensitive devices (network, GPU, guest-agent paths) unless migration notes explicitly approve change.

#### Q35TopologyPlanner (import path)

All bus-policy decisions for a single Proxmox import pass are centralized in `Q35TopologyPlanner` (`src/import/proxmox/mapper/topology.rs`). This struct is instantiated once per import and is the **single source of truth** for bus names during mapping. No mapper helper may derive bus names independently from `RuntimeTarget`.

Planner contract:

| Decision | PortableLinux | ProxmoxParity |
|---|---|---|
| `readconfig` injected | `/usr/share/ezkvm/ezkvm-q35.cfg` | `/usr/share/qemu-server/pve-q35-4.0.cfg` |
| Machine string rewritten | No (kept as-is) | Yes (`+pve0` appended) |
| `legacy_root_bus()` | `pcie.0` | `pci.0` |
| `audio_controller_bus()` | `pci.2` | `pci.2` |
| `xhci_controller` default placement | `bus: pci.1, addr: 0x1b` | `bus: pci.1, addr: 0x1b` |
| hostpci without explicit bus | Auto-allocates `ich9-pcie-port-1..8` | Falls back to `pcie.0` |
| Root-port budget | 8 (`MAX_PORTABLE_ROOT_PORTS`) | N/A |

The 8-port budget matches the port definitions in `share/ezkvm-q35.cfg`. Changing either requires updating both.

**Safety fallback**: `normalize_legacy_root_bus` in `src/qemu/manager.rs` rewrites any surviving `pci.N` bus references to `pcie.0` at command-emit time for portable-linux + Q35 + non-pve machines. This is a last-resort normalization, not a replacement for correct planner output.

Q35 device placement policy:

| Device Class | Default Bus Type | Bridge Chain | Hotplug Model | Slot Stability Requirement |
| --- | --- | --- | --- | --- |
| PCIe NIC / GPU / NVMe / passthrough devices | PCIe | `pcie-root-port` -> endpoint | PCIe native hotplug | Required for imported VMs |
| Legacy PCI network/storage/audio cards | Legacy PCI | `pcie-pci-bridge` -> `pci-bridge` -> endpoint | ACPI/SHPC bridge hotplug | Required for imported VMs |
| Guest agent and related serial controllers | Legacy PCI unless explicit PCIe policy is required | Profile-defined, but consistent across imports | Depends on controller choice | Required for imported VMs |

Avoid these anti-patterns:

- Placing large numbers of legacy PCI devices directly on `pcie.0`.
- Using deep PCIe switch hierarchies without bus budget justification.
- Re-slotting imported devices without explicit migration guidance.
- Bypassing `Q35TopologyPlanner` to derive bus names from `RuntimeTarget` in mapper helpers.

Topology validation checklist for Q35 changes:

- IO window budget reviewed (bridge/port IO pressure considered).
- Bus number budget reviewed (0..255 domain usage planned).
- Hotplug behavior reviewed (native PCIe vs bridge-based semantics).
- Dry-run parity checked against captured Proxmox command lines.
- Imported guest-visible slot identities preserved or migration-noted.
- `MAX_PORTABLE_ROOT_PORTS` and `share/ezkvm-q35.cfg` port count kept in sync.

## 3. Layering / Packaging

### Goal
Group similar components into clear architectural layers to reduce dependency cycles and improve evolvability.

### Layer Model
1. Interface Layer:
- `cli`
- Responsibility: parse input, invoke use-cases, present results.

2. Application/Use-Case Layer:
- command handlers, runtime orchestration modules
- Responsibility: coordinate workflows across components.

3. Domain Layer:
- config schema, validation rules, command intent structures
- Responsibility: core business rules and invariants.

4. Infrastructure/Adapter Layer:
- process execution, filesystem I/O, network tooling commands, QMP transport
- Responsibility: interact with external systems.

### Dependency Direction
- Interface -> Application -> Domain -> Infrastructure abstractions.
- Infrastructure depends on domain data contracts, not on CLI details.
- Avoid cross-layer shortcuts that bypass validation or orchestrator logic.

### Packaging Guidelines
- Package by bounded context first (`config`, `qemu`, `network`, `storage`) rather than technical role.
- Inside a context, split by concern:
  - schema types
  - validation
  - loading/merge
  - execution/adapters
- Keep test packages aligned with production contexts (`tests/config`, `tests/integration`).

### Architectural Guardrails
- No cyclic module dependencies.
- Keep public API surface minimal; prefer `pub(crate)` by default.
- Keep adapter-specific details out of domain models where possible.
- Document any intentional architecture exceptions near the affected module.

## 4. Architecture Decision Records (ADRs)

Major architecture decisions are recorded in `doc/dev/adr/` for visibility and future reference.

### Current ADRs

- [ADR-0001: Base Selection](adr/ADR-0001-base-selection.md) - Why current codebase was chosen as foundation for incremental convergence
- [ADR-0002: Import Normalization Contract](adr/ADR-0002-import-normalization-contract.md) - How external configs (Proxmox) normalize to canonical schema
- [ADR-0003: Hooks Policy](adr/ADR-0003-hooks-policy.md) - VM lifecycle hooks design (pre/post start/stop)
- [ADR-0004: Trait Seam Policy](adr/ADR-0004-trait-seam-policy.md) - Where and how trait-based extensibility is allowed
- [ADR-0005: Q35 Topology Contract](adr/ADR-0005-q35-topology-contract.md) - Why ezkvm preserves Proxmox-aligned Q35 PCIe/PCI topology and slot stability for imports

Concrete seam definitions and examples live in `doc/dev/EXTENSIBILITY_SEAMS.md`.

### Adding New ADRs

When proposing an architecture decision:
1. Create a new file in `doc/dev/adr/ADR-NNNN-{title}.md`
2. Use the [template below](#adr-template)
3. Reference related ADRs
4. Link from this document
5. Discuss during architecture review before merging

### ADR Template

```markdown
# ADR-NNNN: {Title}

**Date:** YYYY-MM-DD  
**Status:** Proposed | Accepted | Deprecated | Superseded  
**Context:** Brief one-liner

## Question

What question does this decision answer?

## Decision

Clear statement of the decision.

## Rationale

Why was this decision made? What alternatives were considered?

## Consequences

What are the positive and negative consequences?

## Related ADRs

Links to related decisions.

## References

Links to docs, code, or external resources.
```

### Evolution Guidance
- For new capabilities, add or extend a context module before creating global utility modules.
- For larger features, create a thin orchestrator function and push detailed logic into focused helpers.
- Update this document when introducing new top-level components or changing layer boundaries.
