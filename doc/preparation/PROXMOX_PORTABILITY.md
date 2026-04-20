# Proxmox Import Portability

## Purpose

This document evaluates how ezkvm should make imported Proxmox VMs runnable on non-Proxmox Linux hosts, specifically:

- Debian Trixie
- Ubuntu 26.04 LTS
- Arch Linux

The current Proxmox import path is already useful for runtime parity, but many imported VMs still depend on Proxmox host conventions and helper infrastructure. The question is not whether ezkvm should preserve Proxmox behavior. It should. The question is which parts of that behavior belong to the guest contract and which parts are only host implementation details that must be rewritten for portability.

## Problem Statement

Today, imported Proxmox VMs preserve more than guest-visible behavior. They also preserve several Proxmox-specific host runtime assumptions.

Examples already present in the repo include:

- Proxmox runtime socket and PID paths under `/var/run/qemu-server/<vmid>.*`
- Proxmox bridge scripts under `/usr/libexec/qemu-server/pve-bridge` and `/usr/libexec/qemu-server/pve-bridgedown`
- Proxmox boot splash asset `/usr/share/qemu-server/bootsplash.jpg`
- Proxmox Q35 readconfig asset `/usr/share/qemu-server/pve-q35-4.0.cfg`
- Looking Glass shared memory path `/dev/kvmfr0`
- Looking Glass client path `/usr/local/bin/looking-glass-client`

Some of these are required for strict Proxmox runtime parity. None of them are portable across normal Debian, Ubuntu, and Arch installations.

This creates a mismatch:

1. The imported YAML is canonical ezkvm config.
2. The canonical config should be runnable by ezkvm.
3. But the current imported result still assumes parts of a Proxmox host layout.

That is acceptable for parity testing and Proxmox-host reproduction. It is not acceptable as the long-term portable import story.

## Design Constraint

ADR-0002 remains correct and should not be weakened:

- imported configs still normalize into the current canonical schema
- runtime defaults still belong in profiles, not ad hoc mapper logic
- dry-run comparison against captured Proxmox commands remains the correctness check for the parity path

The portability work therefore should not introduce a second schema. It should split runtime concerns more cleanly.

## What Must Be Preserved

These properties are part of the guest-facing contract and should remain stable after import:

- machine family and firmware mode
- CPU model and important CPU feature flags
- PCIe topology and device ordering where guest behavior depends on it
- controller layout and disk attachment semantics
- network device model, MAC address, and boot ordering
- guest agent, TPM, SPICE, VNC, and serial behavior at the semantic level
- Windows-oriented Hyper-V feature sets where guest expectations depend on them

These properties affect what the guest sees. Replacing them casually would risk regressions.

## What Should Not Be Preserved Literally

These are host implementation details and should become ezkvm-managed abstractions:

- host runtime directory layout
- socket file paths
- PID file paths
- tap naming conventions when only the host cares about the exact name
- bridge helper script paths
- host-side helper binaries and package-specific executable paths
- firmware asset discovery paths
- optional host integrations such as Looking Glass shared-memory and client binary locations

The portable import goal is: preserve guest-visible semantics, not Proxmox filesystem trivia.

## Approaches

### Approach 1: Recreate Proxmox Host Semantics Everywhere

This approach keeps imported configs effectively unchanged and teaches ezkvm to reproduce the Proxmox host environment on non-Proxmox systems.

That means creating or emulating things such as:

- `/var/run/qemu-server/...`
- `tap<vmid>i<index>` naming
- Proxmox bridge script behavior
- Proxmox-specific helper assets and socket conventions

#### Pros

- Best match for current parity fixtures and captured QEMU commands
- Lowest immediate churn in mapper and profile logic
- Simplifies debugging against Proxmox reference output because host details stay nearly identical

#### Cons

- Bakes Proxmox host behavior into ezkvm permanently
- Requires ezkvm to imitate a distro-specific product layout that Debian, Ubuntu, and Arch do not ship
- Makes packaging and permissions harder, especially for bridge helpers, sockets, and runtime directories
- Turns portability into a compatibility shim problem rather than a clean runtime design
- Makes future non-Proxmox features harder because the runtime contract is anchored to Proxmox naming and paths

#### Assessment

Useful as a regression mode, but weak as the primary design direction.

### Approach 2: Rewrite Imports Into Pure Portable ezkvm Runtime Conventions

This approach treats Proxmox host conventions as translation input only. Imported configs would preserve guest-visible semantics, but all host runtime details would be rewritten to ezkvm-native conventions.

Examples:

- runtime files under an ezkvm-owned runtime directory instead of `/var/run/qemu-server`
- standard QEMU bridge helper or explicit ezkvm network setup instead of `pve-bridge`
- distro-discovered firmware and helper paths instead of Proxmox asset paths
- ezkvm-owned TPM and agent socket locations

#### Pros

- Cleanest long-term architecture
- Best fit with canonical-schema thinking
- Makes imported VMs behave like normal ezkvm VMs instead of special Proxmox artifacts
- Easier to support Debian, Ubuntu, and Arch without shipping fake Proxmox filesystem structure

#### Cons

- Higher near-term implementation cost
- Requires careful identification of which Proxmox behaviors are actually guest-visible and must remain unchanged
- Makes dry-run parity comparison harder if the only validation strategy is byte-for-byte command equivalence
- Risks accidental regressions if topology-preserving rules are not enforced carefully

#### Assessment

Architecturally strong, but too blunt if it replaces the current parity path outright.

### Approach 3: Distro-Specific Imported Profiles

This approach keeps Proxmox import mostly as-is, but swaps Proxmox-specific defaults for distro-specific profiles such as:

- `debian-portable-base`
- `ubuntu-portable-base`
- `arch-portable-base`

Each profile would provide helper paths, firmware paths, networking defaults, and host integration paths appropriate to that distro.

#### Pros

- Straightforward migration from the current profile-driven model
- Makes distro differences explicit
- Can solve path differences without overhauling import structure immediately

#### Cons

- Risks encoding packaging differences as primary business logic
- Still leaves the main design question unresolved: what is guest contract versus host runtime detail?
- Can become a matrix explosion when combined with Proxmox, Windows, UEFI, GPU passthrough, Looking Glass, and future features
- Debian and Ubuntu may share some paths now, but that is not a stable architectural boundary

#### Assessment

Useful as a delivery mechanism for host-specific defaults, but not sufficient as the core design.

### Approach 4: Dual Import Targets: Proxmox Parity And Portable Linux

This approach keeps two explicit runtime targets:

- `proxmox-parity`: preserve current strict runtime parity behavior for validation and reproduction
- `portable-linux`: preserve guest-visible semantics but rewrite host runtime details into ezkvm-managed abstractions

The mapper still produces canonical config. The difference is the assigned runtime profile set and the allowed normalization of host-only details.

#### Pros

- Preserves current Proxmox validation workflow
- Gives users a clear path for non-Proxmox execution
- Avoids forcing one output mode to satisfy two incompatible goals
- Makes tradeoffs explicit instead of hidden in conditionals

#### Cons

- Adds product surface area
- Requires the team to maintain two supported intents
- Needs clear documentation so users understand when parity mode is appropriate and when portable mode is appropriate

#### Assessment

Strong candidate, especially if the portable target is built on top of a clean runtime abstraction rather than distro-specific ad hoc replacements.

### Approach 5: Runtime Capability Layer Under Shared Portable Profiles

This approach introduces ezkvm-owned runtime capabilities that abstract the host details, for example:

- runtime directory provider
- network attachment provider
- firmware locator
- TPM socket/state provider
- display/helper locator
- optional Looking Glass capability

Profiles then depend on capabilities, not hard-coded Proxmox paths.

For example:

- a portable bridge-backed NIC profile would request a bridge capability
- the Debian, Ubuntu, and Arch host adapters would resolve helper path and policy details
- Proxmox parity mode would use a Proxmox capability implementation instead

#### Pros

- Clean separation between VM semantics and host realization
- Best fit for long-term portability beyond only three distros
- Lets parity mode and portable mode share most of the same higher-level config shape
- Contains distro differences in a narrow layer instead of spreading them through import logic

#### Cons

- Requires the most design work up front
- Needs careful scoping to avoid building an oversized abstraction system
- Some current YAML fields may need to become more declarative to benefit fully

#### Assessment

This is the strongest architectural foundation, but it should be introduced incrementally and paired with an explicit portable target.

## Status Summary

**Phase 1**: ✅ COMPLETE (2026-04-19)
- Profile ownership boundaries established
- Runtime target selection implemented (CLI flag, portable-linux default)
- Host-only literal isolation in parity-runtime profile
- Mapper normalization for portable target operational
- All 282+ tests passing

**Phase 2**: ✅ COMPLETE (2026-04-19)
- Central host capability schema implemented (B-42)
- Runtime capability providers implemented: directory, firmware, swtpm, network, Looking Glass (B-43–B-47)
- Capability precedence contract and validation implemented (B-48)
- Integration tests for host capability resolution added (B-49)
- Operator documentation for portable mode added (B-50)

**Phase 3**: ⏳ READY TO START
- Requires real-host distro validation matrix (Debian Trixie, Ubuntu 26.04, Arch Linux)

**Phase 4**: ⏳ PLANNED
- Distro-specific validation matrix for Debian Trixie, Ubuntu 26.04, Arch Linux

1. Keep a `proxmox-parity` import target for validation, regression testing, and users who want exact Proxmox-style runtime behavior. This target requires explicit opt-in; it is not the default.
2. Add a `portable-linux` import target as the default import mode for all non-parity workflows.
3. Build `portable-linux` on an ezkvm-owned runtime capability layer that replaces host-only Proxmox assumptions.
4. Preserve guest-visible topology and semantics across both targets.

This gives ezkvm a clean answer to the core conflict:

- strict parity is still supported
- portability no longer depends on recreating Proxmox

## Central Host Config As An Integration Layer

An important alternative was considered: capture most host-specific differences in a central host config.

### Is It A Replacement For The Chosen Approach?

No.

Using central host config alone is not a full replacement for the selected direction (dual import targets plus runtime capability abstraction). Central config can store host facts and preferences, but by itself it does not enforce the architectural boundary between:

- guest-visible semantics that must remain stable
- host realization details that should vary per environment

Without the capability boundary, central config tends to become a growing matrix of low-level path and package toggles, which recreates coupling in a different location.

### Is It A Good Idea To Integrate?

Yes.

Central host config is a strong complement when used as the backing data source for runtime capability resolution.

In this model:

- profiles express semantic intent
- central config expresses host environment facts and deployment policy
- runtime capability providers resolve concrete paths, binaries, and backend details

This keeps portability explicit and operator-friendly while preserving clean ownership boundaries.

### Impact Analysis

#### Positive impact

- Faster host adaptation across Debian, Ubuntu, and Arch without changing mapper logic.
- Cleaner operator overrides for bridge helper paths, firmware lookup, swtpm location, and runtime directory roots.
- Better packaging/deployment flexibility because host details are configured rather than hard-coded.

#### Risks if done incorrectly

- Config sprawl and unclear ownership if central config starts carrying guest semantics.
- Behavioral drift if host-level overrides silently alter effective VM behavior.
- Harder reproducibility if central config is mutable and not validated or versioned.
- Security and permissions risk if low-level host controls are exposed without strict validation.

#### Net assessment

- Positive when central config is limited to host capability resolution.
- Negative when central config is used as a direct substitute for portability architecture.

### Integration Pattern In The Current Approach

The chosen approach should stay intact and be extended as follows:

1. Keep dual targets:
	- `proxmox-parity` for strict parity workflows
	- `portable-linux` for host-portable execution
2. Add host capability sections to central config for:
	- runtime directory policy
	- network backend preference and helper path
	- firmware locator policy
	- swtpm binary/socket/state policy
	- optional Looking Glass capability
3. Define deterministic precedence:
	- CLI flags
	- VM-local explicit config
	- profile defaults
	- central host capability defaults
	- built-in fallback defaults
4. Keep profile ownership unchanged:
	- profiles keep semantic defaults
	- central config keeps host facts and deployment policy
	- runtime layer performs final host-specific resolution
5. Add validation and safety gates:
	- startup preflight for required binaries and paths
	- actionable errors for missing required capabilities in portable mode
	- graceful downgrade for optional integrations
6. Add test guarantees:
	- host config variants may change host-specific runtime args
	- guest-visible topology and semantics must remain invariant

This integration gives the project both portability and operational flexibility, without reintroducing Proxmox-specific coupling.

## Sequencing With Epic D (Trait-Based Extensibility)

Portability should be implemented before the full Epic D rollout, with one important exception: do D-01 first as a guardrail.

Recommended execution order:

1. **D-01 first** (identify extension seams and trait interfaces).
2. **Portability MVP next** (portable-linux target and profile/runtime split from this document).
3. **D-02 and D-03 after portability MVP** (compile-time registry and one concrete extension backed by the real portability implementation).
4. **D-04 docs last** (after real extension behavior is validated in code and tests).

The concrete D-01 seam definitions are documented in `doc/dev/EXTENSIBILITY_SEAMS.md`.

Rationale:

- Doing D-01 first creates explicit boundaries so portability work does not hard-wire accidental abstractions.
- Doing full Epic D first (especially D-02 and D-03) risks building trait seams around assumptions that portability implementation will later invalidate.
- The portability path in this document is the highest uncertainty area (runtime directories, helper resolution, network backend strategy, firmware discovery), so extension architecture should be informed by that concrete implementation.
- Once portability MVP exists, D-02 and D-03 become lower risk and easier to validate with an actual extension use case instead of synthetic examples.

In short: **guardrails first, portability first, extensibility completion second**.

## Why This Direction Is Better

It respects the real distinction in the current system.

The captured Proxmox QEMU command is useful as a reference for what the guest should experience, but not every literal path in that command is part of the portable contract.

In practice:

- the guest cares that a TPM exists, not that its socket lives under `/var/run/qemu-server`
- the guest cares that networking works with the same model and bridge semantics, not that the host called `/usr/libexec/qemu-server/pve-bridge`
- the guest cares that the machine type, firmware mode, PCI addresses, and CPU flags stay stable
- the host cares about helper paths, runtime directories, package layout, and permission model

That distinction should become explicit in ezkvm.

## Proposed Runtime Boundary

### Preserve In Import Output

- canonical device and controller graph
- stable IDs such as `net0`, `scsi0`, and controller identifiers
- PCI and boot-order decisions required for guest consistency
- guest-facing CPU, TPM, graphics, and agent semantics

### Replace During Portable Normalization

- Proxmox runtime paths under `/var/run/qemu-server`
- Proxmox asset paths under `/usr/share/qemu-server`
- Proxmox bridge helper scripts under `/usr/libexec/qemu-server`
- Proxmox-specific optional helper program paths

### Resolve At Host Runtime

- bridge helper strategy
- runtime directory root
- firmware file discovery
- swtpm socket and state placement
- VNC, QMP, and agent socket placement
- Looking Glass optional dependencies

## Concrete Changes Implied By This Design

### 1. Split Proxmox Profiles By Intent

Current profiles such as `proxmox-base`, `proxmox-q35-uefi`, and `proxmox-windows` mix together guest semantics and Proxmox host assumptions.

They should be split into:

- guest-semantic profiles that remain valid on any host
- Proxmox-parity runtime profiles that exist only for strict parity mode
- portable runtime profiles that use ezkvm capabilities instead of fixed Proxmox paths

Examples:

- keep Windows Hyper-V flags in a guest-semantic profile
- move Proxmox boot splash asset into a parity-only runtime profile
- move `pve-bridge` script paths into a parity-only networking runtime profile
- move Looking Glass host paths into a host-capability-backed optional profile

### 2. Introduce An ezkvm Runtime Directory Contract

Portable mode should not hard-code `/var/run/qemu-server`.

Instead, ezkvm should own a runtime directory contract, for example under an ezkvm-specific runtime root, with derived locations for:

- pid file
- qmp socket
- qga socket
- swtpm socket
- optional VNC unix socket
- serial sockets or PTY symlinks where needed

The exact root can be host-dependent. The important part is that the config model refers to ezkvm runtime intent, not Proxmox path literals.

### 3. Replace Proxmox Bridge Scripts With Portable Network Backends

Portable mode MVP supports these network strategies:

- user-mode networking for zero-privilege bring-up
- standard QEMU bridge helper for bridge-backed networking
- explicit tap creation managed by ezkvm when elevated setup is acceptable

`passt` (rootless higher-fidelity userspace networking) is deferred beyond MVP due to performance and multicast tradeoffs. It may be added as an optional backend in a follow-on ticket.

Debian, Ubuntu, and Arch all support standard QEMU bridge-helper style networking. That is a better portability base than `pve-bridge`.

### 4. Discover Firmware And Helper Assets Per Host

Portable mode should not depend on Proxmox asset packaging.

Instead, ezkvm should discover or configure:

- OVMF firmware files
- bridge helper executable location
- swtpm executable location
- optional Looking Glass client location

This can be supplied by host capability providers and overridden by user config when necessary.

### 5. Treat Looking Glass As Optional Host Integration

The current Looking Glass profile assumes `/dev/kvmfr0` and `/usr/local/bin/looking-glass-client`.

Those are not reasonable portable defaults.

Portable mode should either:

- require explicit host capability confirmation before enabling Looking Glass, or
- downgrade imported Looking Glass-related settings into a disabled-but-preserved optional integration that the user can re-enable on the target host

## Distro Notes

### Debian Trixie

- Good target for a standard portable path because packaged QEMU behavior is close to upstream expectations
- Standard QEMU bridge-helper flow is a better base than trying to recreate Proxmox helpers
- Firmware and swtpm packaging should be treated as discoverable host capabilities, not fixed import defaults

### Ubuntu 26.04 LTS

- Similar portable strategy to Debian is appropriate
- Ubuntu manpages document the same standard QEMU networking primitives and TPM emulator model needed for portable mode
- Long-term support makes it a good reference target for stable portable behavior

### Arch Linux

- Good stress test for package/path variability because users are more likely to assemble their virtualization stack explicitly
- This makes Arch especially valuable for validating that helper paths and firmware assets are capability-driven, not hard-coded
- If the design works on Arch without Proxmox shims, it is likely correctly abstracted

## Recommended Delivery Plan

### Phase 1: Separate Parity-Only Defaults From Portable Semantics ✅ COMPLETE

**Status**: Completed 2026-04-19

**Achievements**:
- ✅ Audit complete: Proxmox profiles audited and classified (B-39)
- ✅ Profile split: `proxmox-parity-runtime.yaml` isolates host-runtime-specific paths
- ✅ Canonical output preserved: Existing parity tests remain byte-close
- ✅ Runtime target selection: CLI and import pipeline support explicit target selection (B-40)
- ✅ Portable normalization: Mapper normalizes host-only literals for `portable-linux` target (B-41)

**Implementation details**:
- Proxmox-specific paths isolated into `proxmox-parity-runtime.yaml`:
  - boot splash: `/usr/share/qemu-server/bootsplash.jpg`
  - network scripts: `/usr/libexec/qemu-server/pve-bridge`, `pve-bridgedown`
- Runtime target selection via `--runtime-target` CLI flag (default: `portable-linux`)
- Mapper functions thread `RuntimeTarget` to conditionally emit host paths:
  - **ProxmoxParity**: Preserves Proxmox paths for parity validation
  - **PortableLinux**: Normalizes to ezkvm-managed paths below

### Phase 2: Add Portable Linux Runtime Target and Capability Resolution ✅ COMPLETE

**Status**: Completed 2026-04-19 (B-42 through B-50)

**Delivered scope**:
- central host capability schema and provider plumbing for portable runtime paths
- precedence contract implementation and source-aware diagnostics:
	- CLI override > explicit VM config > profile defaults > central host defaults > built-in fallback
- portable-mode preflight validation with actionable error reporting
- import and start path integration for capability-backed runtime resolution
- expanded integration coverage for capability matrix and target-specific behavior
- operator documentation for portable import and host-capability configuration

**Behavior**:
- parity fixtures remain byte-close under `proxmox-parity` target
- portable mode enforces guest-topology invariance and host-literal normalization
- regressions guard against reintroduction of Proxmox-only runtime paths

### Phase 3: Validate On Real Target Distros

- Debian Trixie test matrix
- Ubuntu 26.04 LTS test matrix
- Arch Linux test matrix
- capability-matrix runs with host-config permutations
- operator-path validation for preflight and diagnostics UX

### Phase 4: Deferred Enhancements

- optional networking backend expansion (`passt`)

Validation should check both:

- guest-visible behavior remains correct
- host runtime does not require Proxmox filesystem layout or helper binaries

## Implementation Checklist Mapped To Backlog

The checklist below maps the central-host-config integration work to existing backlog items where possible. Gaps are explicitly identified as new backlog candidates.

### A. Guardrails and Architectural Seams

- Define capability seams and ownership boundaries before adding host-config resolution logic.
	- Backlog: D-01 (trait interfaces and extension seams).

### B. Runtime Target and CLI Surface

- Add and document portable-linux mode selection and any required CLI flags.
	- Backlog: B-03 (import CLI command), B-33 (explicit output/runtime modes).

### C. Network Capability Resolution

- Integrate host-config-driven network backend selection (bridge helper path, tap strategy, user-mode fallback). `passt` support is deferred beyond MVP.
	- Backlog: B-11 (network fidelity).
	- Gap: Add a new backlog item for host capability resolution policy and precedence for network backends.

### D. Profile Ownership and Compaction Safety

- Ensure central host config does not take over guest-semantic ownership from profiles.
- Keep compaction deterministic while introducing host-resolved runtime details.
	- Backlog: B-24 (compaction ownership boundaries), B-30 (profile-first compaction audit).

### E. Central Host Config Schema and Precedence

- Add central config schema sections for host capabilities: runtime directories, firmware locator, swtpm policy, helper paths, optional integrations.
- Enforce precedence: CLI > VM explicit > profile defaults > central host defaults > built-in fallback.
	- Gap: Add new backlog items for host-capability schema and precedence contract.

### F. Validation and Preflight

- Add preflight checks that validate required binaries and paths in portable mode.
- Produce actionable errors for missing required capabilities and graceful downgrade for optional integrations.
	- Gap: Add new backlog item for portability preflight validation and error model.

### G. Regression and Matrix Testing

- Add test matrix for Debian Trixie, Ubuntu 26.04 LTS, and Arch using host-config variants.
- Assert invariant guest-visible topology/semantics with host-specific runtime differences only.
	- Backlog: E-01 (regression expansion).

### H. Sequencing

- Execute in this order:
	1. D-01
	2. portability MVP phases 1-2
	3. host capability resolution plus central host config integration (this checklist)
	4. validation matrix and regression hardening

This keeps the selected approach intact while making central host config a first-class integration mechanism rather than a replacement architecture.

## Implementation Progress (as of 2026-04-19)

### Phase 1 Complete: Profile & Runtime Target Split

**What was done:**
- Split Proxmox profiles into guest-semantic and host-runtime-specific layers (B-39)
  - `proxmox-base.yaml`: guest-visible device/boot semantics (stable across targets)
  - `proxmox-parity-runtime.yaml`: Proxmox-only host paths (parity target only)
- Implemented explicit runtime target selection (B-40):
  - CLI flag `--runtime-target` with portable-linux as default
  - Import pipeline threads RuntimeTarget through mapper calls
  - ProxmoxParity requires explicit opt-in
- Implemented host-literal normalization for portable-linux (B-41):
  - Network backend: bridge (kernel helper) instead of tap + script
  - Runtime paths: ezkvm-managed instead of /var/run/qemu-server
  - TPM socket: /var/run/ezkvm instead of Proxmox path
  - Agent socket: /var/run/ezkvm instead of Proxmox path
  - Serial sockets: /var/run/ezkvm instead of Proxmox path
  - PID file: resolved at runtime via XDG_RUNTIME_DIR instead of hardcoded

**Test coverage:**
- All 282+ library and integration tests passing
- Parity fixtures remain byte-close under ProxmoxParity target
- Portable mode regressions verify guest topology invariance
- New parity-specific tests assert tap/ifname/script preservation

### Phase 2 Complete: Central Host Config Schema and Capability Resolution

**What was completed:**
- Central config schema sections for host capabilities were integrated for portable runtime resolution.
- Precedence contract was implemented and surfaced in diagnostics:
	- CLI override > explicit VM config > profile defaults > central config > built-in
- Validation gates for required capabilities in portable mode were added to runtime preflight.
- Host capability resolver integration was wired across runtime, TPM, networking, firmware, and optional Looking Glass handling.
- Integration tests and user/operator documentation were expanded for capability resolution behavior.

**What remains after Phase 2:**
1. Execute full real-host distro validation matrix (Debian, Ubuntu, Arch) and capture operator runbooks (Phase 3).
2. Evaluate deferred optional enhancements such as `passt` networking backend support.

Note: B-30 (profile-first compaction audit) and B-31 (profile inference coverage expansion) are both Done.

## Non-Goals

This work should not:

- create a second import schema
- remove the existing Proxmox parity workflow
- relax topology preservation where guest behavior depends on it
- hide host-specific requirements inside undocumented mapper defaults

## Final Decision

ezkvm should not try to make imported Proxmox VMs portable by recreating Proxmox everywhere.

Instead, it should keep strict Proxmox parity as one explicit target, and add a portable Linux target that preserves guest-visible semantics while replacing host-only Proxmox runtime details with ezkvm-owned abstractions and host capability resolution.

That gives the project the correct long-term boundary:

- Proxmox remains an import source and a validation reference
- ezkvm becomes the runtime owner
- Debian, Ubuntu, and Arch become normal supported hosts rather than partial Proxmox impersonators