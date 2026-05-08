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
- Confirm explicit export-mode behavior is consistent and documented:
  - compact mode remains default for import output
  - canonical mode is available for fully explicit export
  - debug-canonical mode is available for diagnostics and parity investigations
- Document mode semantics clearly: profile-aware compaction is a compact-mode behavior and must not leak into canonical/debug-canonical output.
Dependencies: B-29
Acceptance Criteria:
- Verified that profile inference remains active across representative Windows/Linux/macOS/headless/parity fixtures, including `proxmox-base`, `proxmox-q35-uefi`, `proxmox-windows`, `linux-l26-common`, `macos-kvm`, `looking-glass`, `gpu-passthrough`, `hugepages`, `viommu`, `hidden-hypervisor`, `headless-vnc`, and `proxmox-parity-runtime` where applicable.
- Verified that profile-aware compaction omits redundant fields owned by profiles in compact mode only.
- Verified that compact output remains smaller than canonical output for representative complex fixtures (for wakiza/felucia `108.conf`, current dry-run output is approximately `88 -> 71` lines after recent schema/layout changes).
- CLI behavior documented for explicit mode selection: `--output-mode {canonical,compact,debug}` with compact as default.
- Verified that canonical and debug-canonical exports are available and do not apply profile-compaction elision rules.
Estimate: 1 day

### B-31 Expand profile inference coverage for Proxmox importer
Scope:
- Extend profile inference heuristics to cover additional Proxmox patterns beyond current baseline layers.
- Add inferred profiles for: nested virtualization (l2 profiles), AppleSMC + SMBIOS (macOS-kvm variant), mixed storage buses, specific Hyper-V variants.
- Document profile inference decision tree and maintainability model, including interaction with explicit export modes.
Dependencies: B-23
Acceptance Criteria:
- At least 5 new profile inference rules added and tested.
- Inference decision tree documented in `doc/user/config/profiles-and-merge.md`.
- Representative fixtures validate correct inference across rule set with snapshot tests in compact and canonical/debug-canonical export paths.
- No breaking changes to existing profile stack behavior or explicit output-mode semantics.
Estimate: 2.5 days

### B-32 Omit deterministic fields in import-output mode
Scope:
- Identify deterministic/derived fields (e.g., auto-assigned IDs, bus/address allocations, socket paths) that should be omitted from import output to reduce clutter.
- Extend `skip_serializing_if` policies in config schema to omit deterministic fields when serializing imported configs.
- Ensure QEMU command generation infers or auto-derives these fields from context when deserializing.
- Capture consistency finding: where feasible, prefer schema-level ownership (controller-owned devices) over post-parse cross-reference checks.
Dependencies: B-29
Acceptance Criteria:
- Deterministic field list documented (at least 8-10 fields identified).
- Serialization omits these fields; QEMU arg generation reconstructs them deterministically.
- Schema validation allows missing deterministic fields during import deserialization.
- Snapshot tests verify generated QEMU args remain identical with/without fields.
- Documented decision: schema-driven linkage is the preferred long-term approach for controller/device consistency.
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

### B-35 Update schema to attach drives to controllers which belong to devices
Scope:
- Allow for controllers to reside under devices, and drives under controllers, and make this the default for generated vm yaml files.
Example yaml:
```
devices:
   - controller: "pvscsi"
     drives:
      - { path: "/dev/vm1/vm-108-boot", type: "disk", boot_index: 100 }
```
Note that id's can be generated automatically when creating the qemu commandline, and interface type can be linked to the controller rather than the drive.
Allow the original devices.drives stansa as well, to maintain backwards compatibility, but also to allow for controller to be exposed in profile, and drives connected to that from vm config.

Note: This may already be partially implemented, but as of the time of writing, a generated yaml from proxmox config does not default to the suggested layout. After this task is finished, it should do so.

### B-36 Add central host capability schema for portable runtime
Scope:
- Add central config sections for host capability resolution used by portable runtime mode.
- Cover at minimum: runtime directory policy, firmware locator policy, network helper/backend policy, swtpm policy, optional Looking Glass capability.
- Keep schema ownership clear: host capability facts/policies only, not guest-semantic VM settings.
Dependencies: B-33, D-01
Acceptance Criteria:
- Central config schema supports host capability sections with validation and documented defaults.
- Portable runtime can read capability settings without requiring VM-local duplication.
- Documentation clearly distinguishes host capability config from profile/VM semantic config.
Estimate: 3 days

### B-37 Define and implement runtime precedence contract
Scope:
- Implement deterministic precedence for effective runtime resolution.
- Required order: CLI flags > VM-local explicit config > profile defaults > central host defaults > built-in fallback.
- Ensure dry-run and run paths share identical precedence behavior.
Dependencies: B-36
Acceptance Criteria:
- Precedence order is enforced in code and documented.
- Unit/integration tests cover override scenarios and conflict resolution.
- No regression in existing Proxmox parity behavior when central host config is absent.
Estimate: 2 days

### B-38 Add portability preflight validation and error model
Scope:
- Add preflight validation for portable runtime requirements (required binaries, helper paths, firmware availability, permission-sensitive runtime directories).
- Add structured, actionable errors for missing required capabilities and explicit graceful degradation for optional capabilities.
- Ensure preflight output is deterministic in dry-run and run modes.
Dependencies: B-36, B-37
Acceptance Criteria:
- Portable mode fails fast with actionable diagnostics when required capabilities are missing.
- Optional capabilities (for example Looking Glass integration) degrade predictably without breaking VM start.
- Integration tests cover success, required-capability failure, and optional-capability downgrade paths.
Estimate: 3 days

### B-39 Separate parity-only defaults from portable semantics
Scope:
- Audit current Proxmox profiles and import defaults and classify each field as guest-semantic or host-runtime-specific.
- Split mixed profile ownership so Proxmox host literals (for example `qemu-server` runtime paths, `pve-bridge` scripts, Proxmox asset paths) are isolated into parity-only runtime profile layers.
- Keep guest-visible semantics unchanged and preserve existing Proxmox parity dry-run behavior in parity workflows.
Dependencies: B-38
Acceptance Criteria:
- Proxmox-only host literals are isolated from guest-semantic defaults in profile/runtime layering.
- Import output for parity workflows remains compatible with current parity fixtures.
- Documentation records ownership boundaries and migration notes for profile/runtime split.
Estimate: 3 days

### B-40 Add explicit runtime target selection for Proxmox import
Scope:
- Extend import CLI and import pipeline to support explicit runtime targets: `proxmox-parity` and `portable-linux`.
- `portable-linux` is the default; `proxmox-parity` requires explicit opt-in via flag.
- Keep canonical schema output unchanged while selecting target-specific runtime normalization/profile assignment behavior.
- Document runtime target semantics and operator guidance.
Dependencies: B-39
Acceptance Criteria:
- CLI default is `portable-linux`; `proxmox-parity` must be explicitly requested.
- Integration tests cover target selection and verify parity target preserves current behavior.
- User docs explain when to use parity vs portable targets.
Estimate: 2 days

### B-41 Implement portable-linux runtime normalization for host-only literals
Scope:
- For `portable-linux` target, normalize host-only Proxmox literals (runtime paths, helper paths, firmware asset paths, optional integration paths) into ezkvm-managed runtime capability resolution.
- Preserve guest-visible topology/ordering/semantics while allowing host-specific runtime realization.
- Network backend MVP supports bridge-helper and user-mode networking only; `passt` is deferred to a follow-on ticket.
- Add/extend tests to assert invariant guest semantics with host-specific runtime differences.
Dependencies: B-39, B-40
Acceptance Criteria:
- Portable target output/runtime no longer requires Proxmox host filesystem conventions.
- Parity fixtures remain byte-close under `proxmox-parity` target; portable mode asserts guest-topology invariance only, not literal host-path equivalence.
- Regression tests verify portable normalization behavior and guard against host-literal reintroduction.
Estimate: 4 days

## Phase 2: Host Capability Resolution (Portable Runtime Infrastructure)

### B-42 Extend central config schema with host capability sections
Scope:
- Add `host_capabilities` top-level section to central config with subsections for:
  - `runtime`: runtime directory root and socket/log placement policies
  - `firmware`: OVMF discovery, fallback paths, version handling
  - `tpm`: swtpm binary location, socket vs state-file placement, socket directory policy
  - `network`: bridge helper binary path, backend preference (bridge-helper vs user-mode)
  - `integrations.looking_glass`: optional Looking Glass client binary, shared-mem device policy
- Support precedence: CLI flags > explicit VM config > profile defaults > central config > built-in fallback
- Define validation gates for required vs optional capabilities per target mode
Dependencies: B-41
Acceptance Criteria:
- Central config schema updated with all host capability sections
- Each section includes version comments and migration guidance
- Schema validates required capabilities for portable-linux target
- Integration test covers schema parsing and precedence ordering
- Documentation includes operator examples for each Debian, Ubuntu, Arch variant
Estimate: 3 days

### B-43 Implement runtime directory capability provider
Scope:
- Build host capability resolver layer that replaces hard-coded runtime path generation
- Runtime directory provider discovers/uses:
  - `XDG_RUNTIME_DIR` environment variable (preferred for portable)
  - central config `host_capabilities.runtime.root_directory` (deployment policy override)
  - built-in fallback: `~/.local/run/ezkvm`
- Derive and namespace socket/log locations under resolved runtime root:
  - pid files: `<runtime>/ezkvm/<vmname>.pid`
  - QMP sockets: `<runtime>/ezkvm/<vmname>.qmp`
  - guest-agent sockets: `<runtime>/ezkvm/<vmname>.qga`
  - TPM sockets: `<runtime>/ezkvm/<vmname>.swtpm` (or state dir if state-file policy)
  - serial sockets: `<runtime>/ezkvm/<vmname>.serial<port>`
  - VNC unix sockets: `<runtime>/ezkvm/<vmname>.vnc`
- Add `RuntimeCapabilityResolver` trait and central resolver impl
- Wire resolver into QemuManager and import mapper pipeline
- Add precedence evaluation: CLI `--run-dir` > explicit config > central config > env > built-in
Dependencies: B-42
Acceptance Criteria:
- RuntimeCapabilityResolver trait defined with clear interface
- Runtime directory provider correctly resolves paths in all precedence orders
- All normalization formerly done by mapper now delegated to resolver
- Unit tests cover precedence evaluation and path derivation
- Integration tests verify portable mode uses resolved runtime dir instead of /var/run/qemu-server
- No mapper hard-coding of /var/run paths remains
Estimate: 3 days

### B-44 Implement firmware locator capability provider
Scope:
- Firmware capability provider discovers OVMF, SeaBIOS, and other firmware assets
- Discovery strategy (in precedence order):
  - CLI `--ovmf-dir` flag (if provided)
  - Explicit VM config firmware overrides
  - Profile defaults (e.g., from proxmox-q35-uefi)
  - Central config `host_capabilities.firmware.search_paths`
  - Built-in platform defaults:
    - Debian: `/usr/share/ovmf`
    - Ubuntu: `/usr/share/OVMF`
    - Arch: `/usr/share/ovmf`
- Support per-firmware-type configuration (OVMF.fd, OVMF_CODE.fd, etc.)
- Validate firmware files exist and are readable at runtime
- Produce actionable error if required firmware not found
- Support firmware aliasing (e.g., `ovmf/q35` -> resolved path per distro)
Dependencies: B-43
Acceptance Criteria:
- FirmwareCapabilityProvider implemented and wired into boot args generation
- Discovery correctly prioritizes sources per precedence contract
- Unit tests cover all discovery paths and fallback order
- Integration tests verify firmware location resolution for Debian, Ubuntu, Arch
- Actionable error messages when firmware not found
- Portable mode no longer depends on Proxmox firmware asset paths
Estimate: 2 days

### B-45 Implement swtpm capability provider
Scope:
- swtpm capability provider handles TPM backend setup for portable mode
- Provider discovers/resolves:
  - swtpm binary location (CLI override > central config > `which swtpm` > error)
  - TPM socket vs state-file placement policy
  - Socket directory (resolved via runtime directory provider)
  - State directory for state-file policy (defaults to XDG_STATE_HOME or fallback)
- Emit conditional QEMU args based on discovered policy:
  - For socket mode: `-chardev socket,path=<resolved>,server=on,wait=off,id=tpm0`
  - For state-file mode: `-chardev tpmemu,id=tpm0` with state path handling
- Preflight validation: check swtpm binary exists and is executable
- Support for distro-specific swtpm package paths
Dependencies: B-44
Acceptance Criteria:
- swtpmCapabilityProvider implemented with socket and state-file modes
- swtpm binary discovery works via PATH and central config override
- TPM socket/state paths resolved correctly per runtime provider
- Unit tests cover binary discovery, path resolution, and mode selection
- Integration tests verify swtpm args generated correctly for portable mode
- Preflight validation produces actionable error if swtpm not found in portable mode
Estimate: 2 days

### B-46 Implement network backend helper capability provider
Scope:
- Network capability provider selects and resolves bridge-helper backend for portable mode
- Provider discovers/resolves:
  - Bridge helper binary (qemu-bridge-helper) from standard system paths
  - Central config override for custom helper path
  - User-mode fallback if bridge helper not available or disabled
  - Bridge helper configuration directory (usually /etc/qemu)
- Emit conditional network backend args:
  - For bridge helper: `-netdev bridge,br=<bridge>,helper=<helper-path>,id=net<N>`
  - For user-mode fallback: `-netdev user,id=net<N>,hostname=<vmname>`
- Preflight validation: check bridge helper exists and network socket writable
- Support for distro-specific bridge-helper package locations
- Document bridge setup requirements for operator (netctl, ip link, etc.)
Dependencies: B-45
Acceptance Criteria:
- NetworkCapabilityProvider implemented with bridge-helper and user-mode modes
- Bridge helper binary discovered via PATH and central config override
- Fallback to user-mode networking when bridge-helper unavailable
- Unit tests cover helper discovery, path resolution, and mode selection
- Integration tests verify network backend args for portable mode
- Preflight warns about bridge requirement and suggests setup when needed
- Documentation includes bridge setup steps for each distro
Estimate: 2 days

### B-47 Add optional Looking Glass capability provider
Scope:
- Looking Glass provider handles optional integration for portable mode
- Provider discovers/resolves:
  - Looking Glass client binary (user config > central config > PATH > disabled)
  - Shared-memory device policies (/dev/kvmfr0 vs fallback)
  - Optional integration: graceful downgrade if not available
- Behavior modes:
  - Explicit enable (user config): error if not found, actionable guidance
  - Auto-discover (profile or central config): use if found, silently skip if not
  - Disabled (default): no Looking Glass args emitted
- Emit conditional QEMU args when enabled:
  - `-device ivshmem-plain,memdev=ivshmem,size=32M` (if OK)
  - Update display sections for Looking Glass client compatibility
- Preflight validation: warn if Looking Glass enabled but binary not found
- Document Looking Glass setup per distro (package installation, device permissions)
Dependencies: B-46
Acceptance Criteria:
- LookingGlassCapabilityProvider implemented with enable/auto/disable modes
- Graceful downgrade when Looking Glass not available
- Client binary discovery via PATH and central config override
- Unit tests cover discovery, mode selection, and graceful downgrade
- Integration tests verify optional integration behavior
- Documentation includes Looking Glass setup for Debian, Ubuntu, Arch
- Existing parity-mode Looking Glass support not affected
Estimate: 2 days

### B-48 Implement capability precedence contract and validation
Scope:
- Build unified precedence evaluation engine for all capability providers
- Precedence order (highest to lowest):
  1. CLI flags (--run-dir, --ovmf-dir, --swtpm-binary, etc.)
  2. Explicit VM config overrides
  3. Profile defaults
  4. Central config host_capabilities sections
  5. Built-in platform defaults
- Validate and normalize capability resolution results
- Produce clear diagnostic output showing which source was used for each capability
- Add validation gates per mode:
  - portable-linux: required capabilities (runtime, swtpm, network) must resolve
  - proxmox-parity: skip capability resolution, use parity defaults
- Wire precedence engine into import and runtime pipelines
- Add capability resolution diagnostics to --dry-run output
Dependencies: B-47
Acceptance Criteria:
- CapabilityPrecedenceResolver trait and central impl provided
- Precedence evaluation correctly prioritizes all sources
- Validation gates work per target mode and produce actionable errors
- Unit tests cover all precedence combinations and edge cases
- Integration tests verify precedence in import and start pipelines
- --dry-run output includes capability resolution diagnostic 
- Documentation explains precedence and how to use each override point
Estimate: 3 days

### B-49 Add integration tests for host capability resolution
Scope:
- Build test matrix covering capability resolution across Debian Trixie, Ubuntu 26.04, Arch Linux
- Test scenarios:
  - Default capability discovery (no config, no CLI flags)
  - CLI flag overrides for each capability
  - Central config overrides (create test central.yaml variants)
  - Profile-based defaults interaction with capabilities
  - Precedence correctness (CLI wins > explicit > profile > central > built-in)
  - Optional capabilities (Looking Glass) graceful downgrade
  - Preflight validation for portable-linux target (required capabilities)
  - Parity target ignores capability resolution (uses parity profiles)
- Test both import-time capability selection and runtime-time discovery
- Verify portable mode no longer generates Proxmox-specific paths
- Host-specific test fixtures (e.g., mock firmware dirs, mock swtpm binary)
Dependencies: B-48
Acceptance Criteria:
- Integration test suite covers all precedence and mode combinations
- Tests run on matrix of Debian, Ubuntu, Arch (or emulated equivalents)
- Portable mode tests verify no Proxmox paths in generated args
- Parity mode tests verify unchanged behavior from B-41
- Capability resolution diagnostics validated for correctness
- All tests pass on actual host systems with real capability discovery
- Test documentation explains matrix setup and how to run locally
Estimate: 4 days

### B-50 Document portable mode operator guidance
Scope:
- Build comprehensive operator guide for portable-linux mode covering:
  - When to use portable-linux vs proxmox-parity targets
  - Host requirements per distro (QEMU, swtpm, bridge-helper, firmware packages)
  - Network setup guides for bridge-helper (netctl, ip link, udev rules)
  - Central config examples for common distro setups
  - CLI flag override examples for custom deployments
  - Preflight validation error messages and how to resolve them
  - Looking Glass optional setup
  - Troubleshooting capability discovery (enable diagnostics, check paths, etc.)
  - Migration guide: from parity-only imports to portable imports
- Add operational examples for multi-VM deployments with shared central config
- Update user docs with portable-linux as new default target
- Add architecture diagram showing capability precedence flow
- Create quick-start guides (copy-paste friendly) for each distro
- Cross-reference central config schema documentation
Dependencies: B-49
Acceptance Criteria:
- Operator guide published in doc/user/import-proxmox.md or equivalent
- All capability providers documented with examples
- Network bridge setup steps clear and tested per distro
- Troubleshooting guide provides common errors and solutions
- Quick-start guides verified on actual systems
- Central config examples included for each distro variant
- Migration guide helps users transition from parity-only workflows
Estimate: 2 days

## Phase 3: Real-Host Distro Validation

### B-51 Define Phase 3 distro validation matrix and success criteria
Scope:
- Convert the portability Phase 3 plan into an executable matrix covering Debian Trixie, Ubuntu 26.04 LTS, and Arch Linux.
- Define required versus optional scenarios, required artifacts, and pass/fail gates.
- Lock the representative fixture set for portable validation: Linux headless/networked, Windows UEFI+TPM, and one capability-heavy mixed-device fixture.
Dependencies: B-50
Acceptance Criteria:
- Matrix document defines distro rows, scenario columns, and required artifacts.
- Success criteria explicitly distinguish portability-contract failures from optional integration gaps.
- Required portable validation scope excludes hardware-bound non-gating features such as GPU passthrough.
- Matrix is referenced from portability docs and test guidance.
Estimate: 1 day

### B-52 Build reusable Phase 3 validation harness
Scope:
- Add a reusable harness or scripted workflow for running import, dry-run, preflight, and smoke-boot checks across distro-specific environments.
- Standardize artifact capture: imported YAML, dry-run args, capability diagnostics, preflight output, discovered helper/firmware paths, package versions.
- Keep the harness portable between nested-KVM test VMs and real hosts.
Dependencies: B-51
Acceptance Criteria:
- One command or documented workflow can execute the Phase 3 matrix on a prepared distro host.
- Artifact capture is consistent across Debian, Ubuntu, and Arch runs.
- Harness supports both required portable checks and optional integration toggles without changing core fixtures.
- Test guidance documents host prerequisites for nested virtualization versus direct-host execution.
Estimate: 2 days

### B-53 Execute Debian Trixie portable-runtime validation matrix
Scope:
- Run the full required Phase 3 matrix on Debian Trixie as the reference portable host.
- Validate bridge-helper, firmware discovery, swtpm, runtime directory resolution, portable preflight UX, and smoke boots for representative fixtures.
- Record Debian-specific package/path assumptions and any operator setup steps.
Dependencies: B-52
Acceptance Criteria:
- Debian Trixie results include complete captured artifacts for all required scenarios.
- No required portable path depends on Proxmox filesystem conventions.
- Any failures are categorized as code defect, packaging assumption, or operator-doc gap.
- Debian-specific setup notes are captured for operator documentation updates.
Estimate: 2 days

### B-54 Execute Ubuntu 26.04 portable-runtime validation matrix
Scope:
- Run the same required Phase 3 matrix on Ubuntu 26.04 LTS.
- Focus on firmware path and package differences versus Debian while preserving identical fixture expectations.
- Record Ubuntu-specific package/path assumptions and operator setup deltas.
Dependencies: B-52
Acceptance Criteria:
- Ubuntu 26.04 results include complete captured artifacts for all required scenarios.
- Required capability resolution and smoke-boot behavior match Phase 3 acceptance gates.
- Ubuntu-specific deltas versus Debian are documented without weakening the portability contract.
- Any failures are categorized and linked to actionable follow-up work.
Estimate: 2 days

### B-55 Execute Arch Linux portable-runtime validation matrix and publish runbooks
Scope:
- Run the required Phase 3 matrix on Arch Linux as the path-variability stress case.
- Prioritize capability discovery, preflight clarity, and user-mode networking fallback before bridge-helper sign-off.
- Consolidate Debian, Ubuntu, and Arch results into operator runbooks and support guidance.
Dependencies: B-53, B-54
Acceptance Criteria:
- Arch results include complete captured artifacts for all required scenarios.
- Operator runbooks document distro-specific package/setup differences and expected diagnostics.
- Phase 3 summary explicitly states which capabilities are required, optional, and non-gating.
- Portable-mode support claim for Debian, Ubuntu, and Arch is backed by captured validation evidence.
Estimate: 3 days

### B-56 Synthesize portable Q35 root ports dynamically per VM
Scope:
- Replace fixed portable readconfig root-port fanout with per-VM root-port synthesis based on effective device placement.
- Emit only the number of `ich9-pcie-port-*` bridges required by the resolved VM topology while preserving deterministic bus naming.
- Keep Proxmox-parity behavior unchanged and avoid regressions in existing imported hostpci bus assignments.
Dependencies: B-55
Acceptance Criteria:
- Portable Q35 output defines only required root ports instead of a static predeclared set.
- Imported Windows passthrough fixtures no longer expose unused portable root ports in guest device manager.
- Dry-run command snapshots remain deterministic for repeated runs with identical config.
- Import and command-builder tests cover root-port synthesis and fallback behavior when required port count changes.
Estimate: 3 days

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

## Epic K: Cross-Distro Debian Packaging (Debian Trixie + Ubuntu Resolute)

### K-01 Build packaging baseline inventory and contract
Scope:
- Inventory required versus optional runtime dependencies for packaged ezkvm.
- Confirm filesystem/runtime path contract for packaged installs (`/usr/bin`, `/etc/ezkvm`, `/run/ezkvm`, optional `/var/lib/ezkvm`, `/var/log/ezkvm`).
- Lock one-package-per-architecture policy for Debian Trixie and Ubuntu Resolute.
Dependencies: B-54
Acceptance Criteria:
- Dependency inventory and path contract documented in preparation docs.
- Required vs optional dependency split is explicit and reviewable.
- One-package-per-architecture contract is recorded as packaging policy.
Estimate: 1 day

### K-02 Create Debian packaging skeleton for ezkvm
Scope:
- Add initial `debian/` metadata (`control`, `rules`, `changelog`, `install`, runtime directory policy) to build installable artifacts.
- Ensure conffile-safe behavior for admin-edited files under `/etc/ezkvm`.
Dependencies: K-01
Acceptance Criteria:
- `dpkg-buildpackage -us -uc -b` produces installable package artifacts.
- Installed file layout matches packaging contract.
- Config defaults are preserved across reinstall/upgrade scenarios.
Estimate: 3 days

### K-03 Harden dependency policy for cross-distro installability
Scope:
- Keep hard `Depends` on shared Debian/Ubuntu baseline only.
- Move optional integrations to `Recommends` where practical.
- Use alternative dependency expressions for naming differences between distros.
Dependencies: K-02
Acceptance Criteria:
- Package dependencies resolve on Debian Trixie and Ubuntu Resolute without distro-specific package forks.
- Any unavoidable deltas are documented with rationale.
- Dependency policy is documented for maintainers.
Estimate: 2 days

### K-04 Execute dual-distro package validation matrix
Scope:
- Validate build, lint, install, dependency resolution, runtime readiness, and conffile preservation on Debian Trixie and Ubuntu Resolute.
- Capture repeatable validation evidence and operator notes.
Dependencies: K-03
Acceptance Criteria:
- Validation matrix completed for both distros with captured artifacts.
- `lintian` and install/runtime checks pass for required scenarios.
- Required runtime readiness checks pass with packaged defaults.
Estimate: 2 days

### K-05 Add packaging CI and release gate enforcement
Scope:
- Add CI checks for package build/lint and install smoke validation.
- Add release gating requiring dual-distro packaging evidence before merge/release.
Dependencies: K-04
Acceptance Criteria:
- CI enforces packaging checks on relevant changes.
- Release checklist includes dual-distro packaging gate.
- Packaging regressions are blocked before release.
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
