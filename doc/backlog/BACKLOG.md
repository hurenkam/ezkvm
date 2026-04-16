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
