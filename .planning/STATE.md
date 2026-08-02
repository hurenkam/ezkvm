---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 10
current_phase_name: Deployment Packaging - Debian/Ubuntu
status: complete
stopped_at: Phase 10 complete and goal-backward verified (9/9 must-haves, 168/168 tests passing); all 11 phases done, but QEMU-04/DEPLOY-02/DEPLOY-04 remain pending an actual GitHub Actions run (CI workflow authored + locally validated, not yet executed — requires a push, outside this session's no-commit scope)
last_updated: "2026-07-30T02:00:00.000Z"
last_activity: 2026-07-30
last_activity_desc: Phase 10 executed in 2 waves (10-01 tracer: cargo-deb musl packaging + single-target CI; 10-02/10-03 parallel: 4-target CI matrix + size check, packaging/manual-verification docs), fixed a ci-boot.yaml resource-reference bug and a postinst missing-vm.d-directory gap found during independent verification, goal-backward verified — status passed
progress:
  total_phases: 11
  completed_phases: 11
  total_plans: 23
  completed_plans: 23
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2025-07-15)

**Core value:** Import Proxmox VM configurations into a typed Runtime model, save/load as ezkvm YAML, and generate valid QEMU commandlines — with full round-trip fidelity for real-world configs.
**Current focus:** Phase 08 — VM Lifecycle (complete) → Phase 08.1 — USB & SCSI Schema Extension (complete) → Phase 09 — Round-Trip Verification (complete) → Phase 10 — Deployment Packaging (complete, pending live CI run). **All 11 phases are executed and verified; 3 requirements (QEMU-04, DEPLOY-02, DEPLOY-04) await the packaging CI workflow's first real GitHub Actions run before they can be marked fully done.**

## Current Position

Phase: 10 — Deployment Packaging - Debian/Ubuntu
Plan: 3/3 executed, verification passed
Status: Complete
Last activity: 2026-07-30 — Phase 10 executed in 2 waves: 10-01 (tracer — cargo-deb musl packaging metadata, postinst/postrm/tmpfiles.d, single-target debian:bookworm CI workflow proving real ezkvm start/status/stop/kill/reset against real QEMU), 10-02+10-03 in parallel (10-02 expanded the CI matrix to all 4 required targets + package-size sanity check; 10-03 wrote doc/dev/PACKAGING.md + doc/dev/MANUAL-VERIFICATION.md). Independently found and fixed a real bug in the CI fixture (ci-boot.yaml referenced resource IDs that belonged in its own host.resources, not the separate host.yaml's unused host_resources field) and a postinst gap (missing /etc/ezkvm/vm.d directory creation) before goal-backward verification passed (9/9 must-haves, 168/168 tests).

Progress: [██████████] 100%

## Performance Metrics

**Velocity:**

- Total plans completed: 23
- Average duration: —
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | historical | - | - |
| 02 | historical | - | - |
| 03 | 1 | - | - |
| 04 | 1 | - | - |
| 05 | 1 | - | - |
| 06 | 3 | - | - |
| 07 | 5 | - | - |
| 08 | 4 | - | - |
| 08.1 | 3 | - | - |
| 09 | 2 | - | - |
| 10 | 3 | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*

## Historical Phase Verification

- **Phase 01 — Foundation:** Complete. Typed conversion errors, `device_kind()` dispatch across
  device traits, and a mutex-free `RuntimeBuilder` are present; no `type Error = ()` or
  `downcast_ref()` calls remain under `src/`.

- **Phase 02 — Runtime Model:** Complete. `EfiDisk`, `TpmState`, `HostPci`, `Ivshmem`,
  `AudioDevice`, `SpiceDisplay`, and `RawArgs` are first-class Runtime types with builder and
  device-kind coverage in `tests/runtime_phase2.rs`.

## Accumulated Context

### Decisions

- **saphyr over serde_yaml**: serde_yaml is officially deprecated; saphyr is the maintained successor and is already used in the codebase. Do not switch.
- **args field is opaque**: Proxmox `args:` sub-key must be stored and emitted verbatim as `RawArgs(String)`. Never attempt to parse its contents.
- **Snapshot sections must be split first**: Proxmox `.conf` parser MUST detect and discard `[snapshot_name]` sections before processing any key-value pairs. Bleeding snapshot fields into the active config produces silently wrong VMs.
- **ProxmoxImporter as named struct**: Use `ProxmoxImporter` struct (not bare `TryFrom`) so context (storage pool map, host name) can be passed through conversion without tuple hackery.
- **thiserror from day one**: Both `proxmox.rs` and `qemu.rs` currently use `type Error = ()`. Fix in Phase 1 before any new conversion code is written.
- **device_kind() replaces downcast_ref()**: 54 existing `downcast_ref()` sites exist. Add `device_kind()` to all device traits in Phase 1; no new `downcast_ref()` after that.
- **drive-before-device ordering**: QEMU requires `-drive id=X` before `-device ...,drive=X`. Use a segment-based `QemuCommandLine` builder struct (Phase 7) not a flat `Vec<String>`.
- **storage.cfg is first-class input**: Pool references in `.conf` values (e.g., `vm1-pool:vm-108-boot`) cannot be resolved without `storage.cfg`. It is a required input to the importer, not optional.

### Patterns

- Conversion via `TryFrom` impls between layer types (Runtime ↔ Schema ↔ Proxmox ↔ QEMU)
- `Arc<dyn Trait>` for device storage in Runtime (existing pattern — keep)
- Indexed keys (`scsi0`, `hostpci0`, …) parsed by stripping trailing digits then dispatching on base name
- Sub-option values: split on first comma for positional, then `k=v` pairs; handle colons inside values by per-field tokenization (not generic split)

### Watch Out For

- **Colon in sub-option values**: MAC addresses (`BC:24:11`), PCI BDFs (`0000:03:00`), storage volumes (`pool:vm-108-boot`) all contain colons — generic `split(':')` will corrupt them. Use per-field tokenizers.
- **URL-encoded comments**: `##args%3A` in some `.conf` files must be detected and discarded as comments, not decoded as live config.
- **Multi-function GPU**: `hostpci0: 0000:03:00,pcie=1,x-vga=1` maps to two `vfio-pci` devices (`.0` audio + `.1` GPU). Model as `functions: Vec<u8>` in `HostPci`.
- **EfiDisk dual sizes**: `efidisk0` has a logical size field AND a block device size in bytes — both must be preserved.
- **RuntimeBuilder Mutex**: Current builder wraps device Vec in Mutex; remove it (builders are single-threaded, Mutex can poison on panic).
- **Phase 6 YAML boundary**: The Felucia test now exercises `Runtime → ConfigSchema → YAML → ConfigSchema → Runtime`, while dedicated tests cover SPICE and empty collections.

### Roadmap Evolution

- Phase 10 added: Deployment Packaging - Debian/Ubuntu — build installable packages (`.deb`) and verify real VM start/stop/reset lifecycle against actual host tooling on Debian 13 and Ubuntu 26.04. Deliberately deferred until package-creation time; Phase 8's tests remain host-agnostic/stubbed per user directive.
- Phase 8.1 inserted (URGENT) after Phase 8, before Phase 9: USB & SCSI Schema Extension. Phase 9 research surfaced two production bugs (USB vendor:product-ID passthrough panics; SCSI controller type silently collapsed to pvscsi). User rejected handler-level workarounds in favor of proper Runtime/ezkvm-YAML schema support for both, round-tripping losslessly. Added requirements RUNT-10 (USB identity variants) and RUNT-11 (SCSI controller type). **Phase 8.1 completed 2026-07-28**: `UsbHostIdentity` (bus-port/vendor:product-ID), `GenericScsiController`+`ScsiControllerType` (pvscsi/virtio-scsi-pci), and `VirtioScsiSingleDisk` (merged per-disk device, no internal bus, keyed on Proxmox scsiN index) all landed across Runtime/ezkvm-YAML schema/importer/emitter, independently verified goal-backward. RUNT-10/RUNT-11 marked complete. Phase 9's `09-02-PLAN.md` (the old USB handler workaround) was dropped as redundant/superseded; `09-01`/`09-03` replanned against the new baseline. RawArgs "smart recognition" (re-injecting supported items on export) remains deferred, out of scope, per user decision.
- **Phase 9 replanned and completed 2026-07-29**: dropped `09-02-PLAN.md` (its USB panic fix already shipped via Phase 8.1's typed `UsbHostIdentity`); corrected `09-01`/`09-03` to assert literal scsihw-controller-specific device strings (`pvscsi` for felucia, `virtio-scsi-pci` shared for coruscant, 4× per-instance `virtio-scsi-single`-flavored controllers for zbp-server-mh2) now that Phase 8.1 fully wires `scsihw` through the pipeline instead of discarding it. Executed in 2 waves: `tests/round_trip_verification.rs` created with 3 tests chaining all 4 pipeline stages (Proxmox parse → Runtime → YAML → Runtime → QEMU cmdline) for felucia/108, coruscant/501, zbp-server-mh2/301 — 168/168 tests passing, goal-backward verified (8/8 must-haves). QEMU-04 intentionally remains unchecked in REQUIREMENTS.md: its literal wording requires a VM that actually "starts in QEMU," which D-01 explicitly defers to Phase 10's real-boot verification — Phase 9 only proves structural/string-level correctness.
- **Phase 10 planned and completed 2026-07-30**: discussed 8 locked decisions (D-01–D-08: cargo-deb + musl, no systemd, dedicated `ezkvm` group not user/no libvirt, standard FHS layout, Docker+`/dev/kvm` CI test environment, no AppArmor/SELinux in v1, simple Cargo.toml-tracked versioning, single musl-static package across Debian 12/13 + Ubuntu 24.04/26.04); researched (cargo-deb v3.7.0 confirmed live, `qemu-system-x86` — not `qemu-system-x86_64` — is the real package name, `looking-glass-client` must be `Recommends` not `Depends` since it's absent from 3 of 4 target archives); planned 3 waves (10-01 tracer, 10-02+10-03 parallel). Executed: `[package.metadata.deb]`/`[profile.release]` added to `Cargo.toml`, `debian/postinst`/`postrm`/`ezkvm.conf`/`host.yaml.example` created, `.github/workflows/package-verify.yml` (4-target matrix, size sanity check) + `.github/ci-fixtures/host.yaml`/`ci-boot.yaml` (hand-authored CI-safe fixture sidestepping the pre-existing EfiDisk import bug), `doc/dev/PACKAGING.md` + `doc/dev/MANUAL-VERIFICATION.md` (felucia/108's real-hardware boot documented as manual-only, never CI-gating). Independently rebuilt the musl `.deb` and confirmed `Depends: qemu-system-x86, swtpm` / `Recommends: virt-viewer, looking-glass-client` via `dpkg -I`; found and fixed a real bug (CI fixture's device→resource references pointed at the wrong file) and a real gap (postinst never created `/etc/ezkvm/vm.d`) before goal-backward verification passed (9/9 must-haves). Full Docker+KVM CI execution and felucia/108's real-hardware boot were not runnable in this sandbox — an explicitly acknowledged, plan-scoped limitation, not a gap, but it does mean QEMU-04/DEPLOY-02/DEPLOY-04 stay pending until the workflow actually runs on GitHub Actions (requires the user to push the branch/commit). **This was the final currently-planned phase.**

## Next Steps

1. Push the current work to GitHub (or otherwise trigger `.github/workflows/package-verify.yml`) to get the packaging CI workflow's first real run — this is required to close out QEMU-04, DEPLOY-02, and DEPLOY-04 in `.planning/REQUIREMENTS.md` (currently marked pending pending that run; everything else this phase could verify locally has been independently confirmed).
2. Once the CI run passes, update `.planning/REQUIREMENTS.md`'s 3 pending rows to Complete.
3. Consider running `/gsd-audit-milestone` to audit completion against original intent, or `/gsd-complete-milestone` to archive and prepare for a new milestone/version, once the above is closed out.

---
*State initialized: 2025-07-15*
*Last updated: 2026-07-30 — Phase 10 executed and verified; milestone v1.0 complete*

## Session

**Last session:** 2026-07-30T02:00:00.000Z
**Stopped at:** Phase 10 complete and goal-backward verified; all 11 phases executed, but QEMU-04/DEPLOY-02/DEPLOY-04 remain pending the packaging CI workflow's first real GitHub Actions run
**Resume file:** .planning/phases/10-deployment-packaging/10-VERIFICATION.md
