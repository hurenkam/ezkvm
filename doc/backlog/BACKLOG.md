# Concrete Implementation Backlog

Date: 2026-04-15  
Project: ezkvm  
Source Plan: INCREMENTAL_CONVERGENCE.md

Backlog format:
- ID
- Title
- Scope
- Dependencies
- Acceptance Criteria
- Estimate

## Epic A: Foundation and Governance

### A-01 Create convergence ADR set
Scope:
- Create ADRs for: base selection, import normalization contract, hooks policy, trait seam policy.
Dependencies: none
Acceptance Criteria:
- 4 ADRs merged and linked from developer docs.
- Each ADR includes context, decision, alternatives, and consequences.
Estimate: 2 days

### A-02 Define target module ownership map
Scope:
- Document ownership boundaries for `cli`, `config`, `qemu`, `runtime`, `import`, `state`.
Dependencies: A-01
Acceptance Criteria:
- Ownership map committed and referenced in contribution guidelines.
- Review checklist includes layer-boundary verification.
Estimate: 1 day

### A-03 Create convergence tracking board
Scope:
- Create milestones and labels for Epics A-E.
Dependencies: none
Acceptance Criteria:
- All tickets created with dependencies and sequence tags.
Estimate: 0.5 day

## Epic B: Proxmox Import (from v1)

### B-01 Port Proxmox parser into dedicated module
Scope:
- Introduce `src/import/proxmox/` with parser and storage metadata parser.
Dependencies: A-01
Acceptance Criteria:
- Parser reads representative `.conf` samples and emits intermediate typed model.
- Unit tests cover at least 10 realistic config variants.
Estimate: 5 days

### B-02 Add canonical mapper (intermediate model -> current schema)
Scope:
- Build mapper to emit canonical VM config paths only.
Dependencies: B-01
Acceptance Criteria:
- Mapping output passes existing config deserialization.
- Golden tests for CPU, memory, storage, network, host PCI/USB, TPM, and display mapping.
Estimate: 6 days

### B-03 Add import CLI command
Scope:
- Add clap command: `import-proxmox <input> [--proxmox-storage <path>] [--output <path>] [--dry-run] [--strict]`.
Dependencies: B-02
Acceptance Criteria:
- Command available in `--help` output with examples.
- `--dry-run` prints canonical YAML and performs no file writes.
- Exit codes and error messages are deterministic.
Estimate: 3 days

### B-04 Add import validation/reporting
Scope:
- Structured warnings for unsupported/partially supported fields.
Dependencies: B-02
Acceptance Criteria:
- Import report contains warnings with source field references.
- `--strict` returns non-zero when warnings are present.
Estimate: 2 days

### B-05 Integration tests for import-to-start pipeline
Scope:
- End-to-end flow: Proxmox conf -> canonical YAML -> validate/start dry-run.
Dependencies: B-03, B-04
Acceptance Criteria:
- At least 5 fixture-based e2e tests green.
- Dry-run QEMU args match expected snapshots.
Estimate: 3 days

### B-06 Parse and map `efidisk0` for UEFI vars parity
Scope:
- Parse Proxmox `efidisk0` and map it to canonical UEFI vars handling for pflash unit=1 emission.
Dependencies: B-02
Acceptance Criteria:
- Imported configs with `efidisk0` emit both UEFI code and vars drives in dry-run args.
- Includes size/path mapping coverage in tests.
Estimate: 2 days

### B-07 Parse and map `audio0` for SPICE/HDA parity
Scope:
- Parse Proxmox `audio0` and map to canonical audio devices/audiodev wiring.
Dependencies: B-02
Acceptance Criteria:
- Imported configs with `audio0` emit expected `-audiodev` and HDA codec args.
- Unit and fixture tests cover at least one SPICE audio case.
Estimate: 2 days

### B-08 Parse and map `agent` field for guest-agent plumbing
Scope:
- Parse Proxmox `agent` and map to guest-agent socket and virtio-serial device config.
Dependencies: B-02
Acceptance Criteria:
- `agent: 1` imports produce guest-agent chardev/device args in dry-run.
- Disabled/absent agent remains non-emitting and tested.
Estimate: 2 days

### B-09 Parse and map `args` passthrough (safe subset first)
Scope:
- Parse Proxmox `args` into canonical representations for supported subsets first (SPICE, vdagent, input, ivshmem), with warnings for unsupported tokens.
Dependencies: B-02, B-04
Acceptance Criteria:
- Supported subset from `args` is preserved in generated command.
- Unsupported tokens are reported in structured warnings.
Estimate: 3 days

### B-10 Preserve machine and CPU feature fidelity
Scope:
- Import machine options (for example `hpet=off`) and CPU feature list/Hyper-V flags when present.
Dependencies: B-02
Acceptance Criteria:
- Imported machine/cpu sections preserve source options when representable.
- Fixture snapshots assert these flags appear in dry-run output.
Estimate: 2 days

### B-11 Improve network backend fidelity for Proxmox bridge/tap
Scope:
- Map Proxmox networking to canonical backend fields that can emit tap/bridge options (ifname/script/downscript/vhost/queues) with deterministic behavior.
Dependencies: B-02
Acceptance Criteria:
- Imported net configs preserve bridge/tap behavior from representative Proxmox samples.
- Snapshot tests cover queue sizes and vhost behavior where present.
Estimate: 3 days

### B-12 Expand host PCI fidelity for multifunction devices
Scope:
- Improve host PCI mapping for multi-function GPU pairs (for example `.0` + `.1`) and preserve bus/addr/multifunction semantics.
Dependencies: B-02
Acceptance Criteria:
- Representative GPU passthrough samples import both functions when present.
- Snapshot tests verify expected vfio device emission order and attributes.
Estimate: 2 days

### B-13 Add wakiza parity fixture and regression test
Scope:
- Add dedicated `wakiza` import fixture pair (`108.conf` + expected args snapshot) to lock in Proxmox parity improvements.
Dependencies: B-06, B-07, B-08, B-09, B-10, B-11, B-12
Acceptance Criteria:
- Fixture test verifies key command fragments from Proxmox command are preserved.
- Fails on regressions in efidisk/audio/agent/args/network/machine/cpu/hostpci coverage.
Estimate: 2 days

### B-14 Improve CPU and Hyper-V fidelity for Windows guests
Scope:
- Add import mapping and/or policy support to preserve representative Hyper-V CPU feature bundles used by Proxmox Windows workloads.
- Keep default behavior portable while allowing higher Proxmox compatibility where representable.
Dependencies: B-10
Acceptance Criteria:
- Imported Windows guest configs can emit validated Hyper-V feature flags required by representative Proxmox examples.
- Tests cover both default portable output and compatibility-preserving output.
Estimate: 3 days

### B-15 Expand network device fidelity (virtio-net-pci placement and queues)
Scope:
- Extend network import/output mapping to preserve queue sizes and optional explicit PCI placement where present in Proxmox command behavior.
- Preserve deterministic defaults when source data is absent.
Dependencies: B-11
Acceptance Criteria:
- Imported representative bridge/tap configs preserve queue-size settings and placement details when available.
- Regression tests verify generated args include expected net device model/options.
Estimate: 3 days

### B-16 Refine firmware and secure-boot mapping from efidisk metadata
Scope:
- Improve OVMF/secure-boot related mapping using Proxmox `efidisk0` metadata (for example `ms-cert`, `pre-enrolled-keys`) where safely representable.
- Keep current default fallback behavior when metadata is incomplete or unsupported.
Dependencies: B-06
Acceptance Criteria:
- Imported configs with secure-boot-related efidisk metadata map to expected firmware behavior or emit explicit structured warnings when not representable.
- Tests cover metadata-driven firmware mapping and fallback paths.
Estimate: 2 days

### B-17 Add SATA support
Scope:
- Add `SataControllerConfig` struct to schema and update `DriveConfig` to support `interface: sata`.
- Extend Proxmox mapper to parse `sata0`, `sata1`, `sata2` fields from configs.
- Add SATA controller and disk argument generation in QEMU command builder.
Dependencies: B-02
Acceptance Criteria:
- Imported Proxmox configs with sata-backed disks emit correct `-device ahci` controllers and drive attachment args.
- Unit and integration tests cover representative SATA disk and controller combinations.
- Schema validation enforces valid SATA IDs and bus/addr placement.
Estimate: 3 days

### B-18 Serial port configuration
Scope:
- Add `SerialDeviceConfig` struct to schema (parallel to DisplayConfig).
- Extend mapper to parse `serial0`, `serial1` fields from Proxmox configs.
- Support common serial backends: socket, file, chardev.
Dependencies: B-02
Acceptance Criteria:
- Imported Proxmox serial configs map to ezkvm SerialDeviceConfig with correct socket paths and backend types.
- QEMU command builder emits `-chardev` and `-device isa-serial` for each configured serial port.
- Fixtures include serial-port examples; tests validate round-trip accuracy.
Estimate: 2 days

### B-19 Support IOMMU/vIOMMU device definitions
Scope:
- Add `IommuConfig` struct to schema (e.g., containing `iommu_type`, `intremap`, `caching_mode`).
- Extend mapper to detect and represent Intel IOMMU (`-device intel-iommu`) or AMD-V IOMMU setup in Proxmox args.
- Allow explicit IOMMU placement and tuning in schema.
Dependencies: B-02
Acceptance Criteria:
- Imported configs with IOMMU args emit ezkvm IOMMU representation.
- QEMU command builder correctly generates `-device intel-iommu` with passthrough options when IOMMU is configured.
- Schema supports IOMMU enable/disable and option overrides.
- Tests include representative IOMMU passthrough scenarios.
Estimate: 2 days

### B-20 Full mapping of hugepages settings
Scope:
- Enhance hugepages mapping in schema and mapper to preserve complete Proxmox hugepage configuration (size, prealloc, mempath, NUMA binding).
- Extend memory config to support nuanced hugepages settings (e.g., per-socket hugepage pool specifications).
Dependencies: B-02
Acceptance Criteria:
- Imported Proxmox hugepages settings (including size, prealloc flags, mem-path, and host-nodes affinity) round-trip to ezkvm representation.
- QEMU command builder emits correct `-object memory-backend-file` args with all applicable options.
- Tests cover default hugepages, custom sizes (2M, 1G), and NUMA-bound hugepage allocation.
Estimate: 2 days

### B-21 Materialize hugepages profile layer end-to-end
Scope:
- Add and standardize canonical `hugepages` profile defaults in `etc/profiles.d` aligned with `system.memory.hugepages` schema.
- Extend importer profile inference to emit `hugepages` profile when source config/runtime indicates hugepages usage.
- Tighten compaction ownership for hugepages so profile-backed defaults are not redundantly re-emitted in VM-local config.
Dependencies: B-20
Acceptance Criteria:
- Imported hugepages-backed VMs include `hugepages` in inferred profile stacks.
- Profile-aware compaction removes redundant hugepages fields when profile defaults match effective values.
- Fixture tests verify inferred profile stack and resulting command parity.
Estimate: 2 days

### B-22 Add canonical VNC profile schema and `headless-vnc` layer
Scope:
- Extend profile-oriented canonical schema to represent VNC settings needed for layered profile materialization.
- Add `headless-vnc` profile definition and map importer assignment rules for VNC plus headless runtime mode.
Dependencies: B-02, B-05
Acceptance Criteria:
- Canonical schema supports VNC settings used by profile layering.
- `headless-vnc` profile can be emitted by importer for qualifying Proxmox workloads.
- Integration fixtures cover VNC headless profile inference and dry-run argument output.
Estimate: 3 days

### B-23 Add `viommu` and `hidden-hypervisor` tuning profile assignment
Scope:
- Add importer rules to infer `viommu` profile from machine options and/or mapped IOMMU settings.
- Add importer rules to infer `hidden-hypervisor` profile from effective CPU feature patterns (for example `kvm=off` and related hiding strategies).
- Materialize profile files and align compaction ownership for both tuning layers.
Dependencies: B-10, B-19
Acceptance Criteria:
- Imported configs that carry vIOMMU settings emit `viommu` profile.
- Imported configs that carry hypervisor-hiding CPU semantics emit `hidden-hypervisor` profile.
- Fixture tests validate inference, compaction behavior, and generated QEMU args.
Estimate: 2 days

### B-24 Tighten profile-aware compaction ownership boundaries
Scope:
- Refine ownership boundaries for list-shaped and tuning sections so profile-aware compaction can safely remove redundant VM-local fields.
- Keep compaction deterministic and semantics-preserving across profile stacks.
Dependencies: B-21
Acceptance Criteria:
- Compaction removes redundant profile-owned fields for hugepages, access-mode, and tuning layers without changing effective runtime config.
- Regression tests cover ownership-boundary edge cases and ensure stable YAML output.
Estimate: 3 days

### B-25 Expand profile-stack corpus and edge-case coverage
Scope:
- Add fixture coverage for nested virtualization, multiple display backends, mixed storage buses, and macOS-specific AppleSMC/SMBIOS type behavior.
- Expand profile-stack integration tests to cover new deferred-layer materializations.
Dependencies: B-24
Acceptance Criteria:
- Profile-stack test corpus includes representative edge fixtures for all newly materialized layers.
- Snapshot and integration tests pass deterministically and catch profile inference regressions.
Estimate: 3 days

### B-26 Decompose mapper orchestration and module boundaries
Scope:
- Introduce `src/import/proxmox/mapper/` submodules and keep `mapper.rs` as orchestration plus public entry points.
- Move helper parsing and domain-specific mapping logic behind focused internal modules.
Dependencies: B-25
Acceptance Criteria:
- `mapper.rs` contains orchestration and entry points only.
- Mapper domains are split into focused modules (system, storage, network, display, devices, helpers or equivalent).
- No behavioral regressions in existing Proxmox import fixtures and snapshots.
Estimate: 4 days

### B-27 Split profile-aware compaction by domain ownership
Scope:
- Refactor `profile_compact.rs` into domain-focused modules aligned to ownership paths (for example system/devices/network/storage).
- Keep compaction deterministic and profile-aware for current profile stack semantics.
Dependencies: B-26
Acceptance Criteria:
- Profile compaction logic is split into smaller focused files.
- Existing profile-stack integration tests stay green with unchanged effective output.
- Compaction ownership behavior remains stable for mixed profile stacks.
Estimate: 3 days

### B-28 Restrict Proxmox importer public surface to high-level API
Scope:
- Keep only `ImportRunOptions`, `run_import_from_files`, and `ImportError` as public importer API.
- Remove public re-exports of low-level parser/mapper entry points and update internal tests/imports accordingly.
Dependencies: B-26
Acceptance Criteria:
- No repository consumer outside importer internals depends on low-level parser/mapper exports.
- CLI and integration tests continue to use only high-level import API.
- Build and tests pass after export tightening.
Estimate: 1.5 days

### B-29 Add temporary over-size rationale and cleanup guardrails
Scope:
- Add module-level comments in still-oversized files with explicit rationale and cleanup references.
- Record line-count and public-surface targets for mapper/compaction cleanup closure.
Dependencies: B-26, B-27, B-28
Acceptance Criteria:
- Oversized modules include short rationale plus linked backlog IDs.
- Cleanup checklist captures target thresholds and remaining hotspots.
- Technical debt is explicitly tracked until files return to guideline range.
Estimate: 1 day

### B-30 Audit profile-first compaction implementation status
Scope:
- Verify that mapper profiling inference and profile-aware compaction are working correctly in import output.
- Confirm that profile-overlay form is the default and only export mode.
- Document current behavior: profiles are inferred automatically and compaction is mandatory (no canonical export mode currently available).
- If canonical output is needed, defer to separate backlog item.
Dependencies: B-29
Acceptance Criteria:
- Verified that profiles are inferred from Proxmox config (15 layers: proxmox-q35-uefi, windows-common, windows-11, linux-l26-common, macos-kvm, looking-glass, remote-viewer-spice, gpu-passthrough, hugepages, viommu, hidden-hypervisor, headless-vnc, headless-serial, storage-virtio-scsi-single, storage-virtio-scsi-pci).
- Verified that profile-aware compaction omits redundant fields owned by profiles (always enabled, no opt-out).
- Verified that compact output sizes match expectations (5-6x reduction vs. canonical for complex fixtures like wakiza: 212→34 lines).
- CLI behavior documented: `--no-compact` only affects flow-style, not profile compaction.
- Decision made: canonical output mode is optional and can be deferred if not needed or added as separate backlog item if required.
Estimate: 1 day

### B-31 Expand profile inference coverage for Proxmox importer
Scope:
- Extend profile inference heuristics to cover additional Proxmox patterns beyond current 11 layers.
- Add inferred profiles for: nested virtualization (l2 profiles), AppleSMC + SMBIOS (macOS-kvm variant), mixed storage buses, specific Hyper-V variants.
- Document profile inference decision tree and maintainability model.
Dependencies: B-23
Acceptance Criteria:
- At least 5 new profile inference rules added and tested.
- Inference decision tree documented in CONFIGURATION.md profiles section.
- Representative fixtures validate correct inference across rule set with snapshot tests.
- No breaking changes to existing profile stack behavior.
Estimate: 2.5 days

### B-32 Omit deterministic fields in import-output mode
Scope:
- Identify deterministic/derived fields (e.g., auto-assigned IDs, bus/address allocations, socket paths) that should be omitted from import output to reduce clutter.
- Extend `skip_serializing_if` policies in config schema to omit deterministic fields when serializing imported configs.
- Ensure QEMU command generation infers or auto-derives these fields from context when deserializing.
Dependencies: B-29
Acceptance Criteria:
- Deterministic field list documented (at least 8-10 fields identified).
- Serialization omits these fields; QEMU arg generation reconstructs them deterministically.
- Schema validation allows missing deterministic fields during import deserialization.
- Snapshot tests verify generated QEMU args remain identical with/without deterministic fields.
Estimate: 2 days

### B-33 Introduce explicit export modes (canonical, compact, debug-canonical)
Scope:
- Add explicit output mode parameter to ImportRunOptions supporting three modes: canonical (full explicit), compact (profile-overlay), debug-canonical (canonical + deterministic fields + inline comments).
- CLI support via `--output-mode {canonical,compact,debug}` flag (default: compact).
- Validate mode combinations with dry-run behavior.
Dependencies: B-30, B-32
Acceptance Criteria:
- Three output modes working end-to-end from import pipeline.
- Mode selection visible in `--help` with use-case examples.
- Integration tests cover mode switching, snapshot accuracy, and deterministic output.
- Debug mode includes source mapping comments (e.g., "# from Proxmox ostype: win11").
Estimate: 2 days

### B-34 Extend compaction policies for repeated field omission
Scope:
- Enhance profile-aware compaction to omit values that repeat across multiple fields within same section (e.g., same bus/addr pattern across controllers).
- Add compaction heuristics for sparse sections where only a few fields differ from baseline profile.
- Document expanded compaction ownership boundaries in compact.rs and merge.rs.
Dependencies: B-29
Acceptance Criteria:
- Compaction identifies and omits repeated field values when profile base or sibling fields provide redundancy.
- Compaction ownership boundaries explicitly documented with examples.
- Regression tests verify profile-stack composition still inverts compaction correctly.
- Snapshot tests show compacted form 10-15% smaller for dense multi-device fixtures.
Estimate: 2 days

### B-35 Add --canonical flag to emit full explicit schema without profile compaction
Scope:
- Add `--canonical` flag (or `--output-mode canonical`) to the `import-proxmox` CLI command.
- When set, skip `compact_profile_owned_fields` and emit the full serialized schema directly.
- Add `output_mode` field to `ImportRunOptions` (compact | canonical) with compact as default.
- Preserve the `profiles` list field in canonical output for informational purposes.
Dependencies: B-30
Acceptance Criteria:
- CLI accepts `--canonical` flag; help text explains canonical vs. compact trade-off.
- Canonical output contains all fields (including profile-owned defaults) without omissions.
- Compact mode remains unchanged and is the default when `--canonical` is absent.
- Integration test verifies that canonical and compact outputs produce identical QEMU args in dry-run.
- `--canonical` and `--no-compact` flags are composable without conflict.
Estimate: 1.5 days

## Epic C: Flexible Lifecycle Hooks (from v1)

### C-01 Define hook contract and execution policy
Scope:
- Hook points: `pre_start`, `post_start`, `pre_stop`, `post_stop`.
- Define timeout, retries (if any), fail-open/fail-closed behavior.
Dependencies: A-01
Acceptance Criteria:
- Contract documented and approved in ADR.
- Logging and error surface format agreed.
Estimate: 2 days

### C-02 Implement hook runner service
Scope:
- Central runner in runtime layer with structured events.
Dependencies: C-01
Acceptance Criteria:
- Supports sync execution and timeout enforcement.
- Logs include hook name, VM name, duration, outcome.
Estimate: 4 days

### C-03 Add schema support for hook definitions
Scope:
- Add hook config under canonical options path (or agreed path).
Dependencies: C-01
Acceptance Criteria:
- YAML parsing supports hook definitions.
- Invalid hook definitions rejected with actionable errors.
Estimate: 2 days

### C-04 Wire hooks into start/stop flow
Scope:
- Integrate runner into lifecycle orchestration in runtime start/stop handlers.
Dependencies: C-02, C-03
Acceptance Criteria:
- Hooks execute in correct order and policy.
- Dry-run behavior explicitly defined and tested.
Estimate: 3 days

### C-05 Hook tests (unit + integration)
Scope:
- Success, timeout, failure, and policy behavior tests.
Dependencies: C-04
Acceptance Criteria:
- Coverage includes all hook stages and failure modes.
Estimate: 3 days

## Epic D: Trait-Based Extensibility (from v1, adapted)

### D-01 Identify extension seams and trait interfaces
Scope:
- Define traits for import mappers and optional runtime extensions.
Dependencies: A-02
Acceptance Criteria:
- Trait boundaries documented with examples.
- No layer inversion introduced.
Estimate: 2 days

### D-02 Implement compile-time extension registry
Scope:
- Register extension implementations without runtime plugin loading.
Dependencies: D-01
Acceptance Criteria:
- Default registry compiles with no behavior change when unused.
- Extension execution path covered by tests.
Estimate: 3 days

### D-03 Port one concrete extension from v1 patterns
Scope:
- Use a real extension use-case (for example Proxmox mapping strategy variant).
Dependencies: D-02
Acceptance Criteria:
- Demonstrates extension lifecycle and fallback behavior.
Estimate: 2 days

### D-04 Extensibility docs and examples
Scope:
- Add developer guide: how to add an extension safely.
Dependencies: D-03
Acceptance Criteria:
- Includes sample implementation and test template.
Estimate: 1 day

## Epic E: Integration Hardening and Release Readiness

### E-01 Regression suite expansion
Scope:
- Extend integration tests around profiles + import + hooks.
Dependencies: B-05, C-05, D-03
Acceptance Criteria:
- CI has deterministic, parallel-safe test execution.
- Snapshot updates are intentional and reviewed.
Estimate: 3 days

### E-02 Performance and stability checks
Scope:
- Measure startup overhead from hooks and import path.
Dependencies: E-01
Acceptance Criteria:
- Baseline metrics recorded.
- Overhead thresholds documented and met.
Estimate: 2 days

### E-03 Beta release gating
Scope:
- Feature flags and release notes.
Dependencies: E-01, E-02
Acceptance Criteria:
- Feature flags documented.
- Rollback plan validated.
Estimate: 1 day

---

# Later-Stage Backlog (Phase 3)

## Epic F: Fail-Fast Validation

### F-01 Expand validator coverage matrix
Acceptance Criteria:
- Validator covers schema, semantic constraints, and cross-field consistency.
Estimate: 4 days

### F-02 Improve error context granularity
Acceptance Criteria:
- Errors include path context and remediation hint.
Estimate: 2 days

## Epic G: Environment Variable Substitution

### G-01 Add substitution preprocessor in config load pipeline
Acceptance Criteria:
- Supports `${VAR}` and optional default policy if adopted.
Estimate: 2 days

### G-02 Add unresolved variable policy and tests
Acceptance Criteria:
- Deterministic fail behavior when required variables are missing.
Estimate: 1.5 days

## Epic H: Storage/Device Management Subcommands

### H-01 Storage subcommands baseline
Acceptance Criteria:
- Create/info/list/resize/snapshot commands implemented and tested.
Estimate: 5 days

### H-02 Device subcommands baseline
Acceptance Criteria:
- USB/PCI listing and selected hotplug operations available.
Estimate: 4 days

## Epic I: Resource Pooling Improvements

### I-01 Resource reservation model design
Acceptance Criteria:
- Reservation, lease, and conflict model documented.
Estimate: 2 days

### I-02 Implement reservation state backend
Acceptance Criteria:
- Persistent reservation records with stale lock recovery.
Estimate: 4 days

### I-03 Integrate reservation checks into start flow
Acceptance Criteria:
- Conflicts block start with actionable messages.
Estimate: 2 days

## Epic J: Comprehensive Documentation

### J-01 User docs: Getting Started
Acceptance Criteria:
- New user can launch a sample VM end-to-end from this guide.
Estimate: 2 days

### J-02 User docs: Feature Documentation
Acceptance Criteria:
- One page per major feature area with examples.
Estimate: 4 days

### J-03 User docs: Reference Guide
Acceptance Criteria:
- Canonical schema reference complete and versioned.
Estimate: 3 days

### J-04 Developer docs: How to Contribute
Acceptance Criteria:
- Includes workflow, review checklist, and testing expectations.
Estimate: 2 days

### J-05 Developer docs: Architecture and Design
Acceptance Criteria:
- Layer diagrams and extension seam descriptions included.
Estimate: 2 days

### J-06 Developer docs: Coding Guidelines
Acceptance Criteria:
- Error handling, module size limits, and testing style documented.
Estimate: 2 days

---

# Suggested Sprint Sequence

Sprint 1:
- A-01, A-02, A-03, B-01 (start)

Sprint 2:
- B-01 (finish), B-02

Sprint 3:
- B-03, B-04, B-05

Sprint 4:
- C-01, C-02, C-03

Sprint 5:
- C-04, C-05, D-01

Sprint 6:
- D-02, D-03, D-04, E-01

Sprint 7:
- E-02, E-03, F-01 (start)

Sprint 8+:
- Remaining Phase 3 Epics F-J by priority

---

# Definition of Done (cross-cutting)

1. Code compiles and all relevant tests pass.
2. New behavior has unit and integration test coverage.
3. Layer boundaries are respected (architecture review checklist).
4. User-facing behavior documented where applicable.
5. CLI help text and examples updated for new commands.
6. Release notes updated for externally visible changes.

# Risk Register (initial)

1. Schema drift risk between imported Proxmox model and canonical schema.
   - Mitigation: golden mapping tests + strict mapper contract.
2. Hook non-determinism risk.
   - Mitigation: timeout policy, explicit ordering, structured logs.
3. Trait seams causing architectural leakage.
   - Mitigation: extension interfaces restricted to approved seams.
4. Feature velocity reducing stability.
   - Mitigation: phased rollout with integration hardening gate.

# Immediate Next Actions

1. Create tickets for Epics A-E in tracking board.
2. Start A-01 ADR set and A-02 ownership map.
3. Start B-01 parser port in a feature branch.
