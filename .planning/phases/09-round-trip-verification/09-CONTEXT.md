# Phase 9: Round-Trip Verification - Context

**Gathered:** 2026-07-28
**Status:** Ready for planning

<domain>

## Phase Boundary

This phase proves that the complete pipeline — `.conf` + `storage.cfg` → Runtime → ezkvm YAML → Runtime → QEMU cmdline — works end-to-end, without errors, across multiple real-world corpus configs (not just felucia/108.conf). It does **not** boot a real VM or invoke a real `qemu-system-x86_64` binary — that verification is explicitly Phase 10's job, against real Debian 13/Ubuntu 26.04 targets. Phase 9 stays entirely host-agnostic (no real QEMU/swtpm/UI-client binaries), consistent with the testing discipline established in Phase 8.

</domain>

<decisions>

## Implementation Decisions

### Scope: string/structural verification only, no real QEMU execution
- **D-01:** Phase 9 does NOT spawn a real `qemu-system-x86_64` process to check "reaches the QEMU monitor prompt." ROADMAP's success criterion #3 ("A VM launched with the generated cmdline reaches the QEMU monitor prompt without fatal startup errors") is reinterpreted as: the generated cmdline is well-formed and passes structural assertions (see D-04) — actual boot-readiness against a real QEMU binary is deferred entirely to Phase 10. — **Reversibility:** reversible — this is a scoping choice for what Phase 9 asserts, not a data format or API decision; Phase 10 can still exercise the same generated cmdlines against real QEMU without any rework here.
- **D-02:** No new stub/fake binaries are needed for this phase (Phase 8's `fake_qemu`/`fake_swtpm`/`fake_ui_client` stay untouched and unused here) — Phase 9 tests operate purely on the generated `Vec<String>`/cmdline model, never spawning any process.

### Corpus selection for the "two additional configs"
- **D-03:** The two additional integration-test targets (beyond felucia/108.conf) are **coruscant/501.conf** and **zbp-server-mh2/301.conf** — chosen because both ship companion `lspci`/`lspci_t`/`lspci_v` (and, for zbp-server-mh2/301, `lsusb`) output alongside the `.conf`, giving a real hardware cross-check for hostpci/USB device mapping, and because coruscant exercises different SCSI-controller/NUMA paths per the ROADMAP's own risk callout.

### Verification method: structural spot-checks, not exact-diff
- **D-04:** None of the three target configs (felucia/108, coruscant/501, zbp-server-mh2/301) are verified via an exact string diff against any hand-authored reference `.qemu.cmd` file. Instead, each is verified via structural spot-checks: correct device count, correct device types present (cross-referenced against the `.conf`'s device keys and, where available, the corpus's own `lspci`/`lsusb` output), key flags/ordering rules holding (drive-before-device, raw `args` appended verbatim), and zero errors/panics through the full round-trip (`.conf`+`storage.cfg` → Runtime → YAML → Runtime → QEMU cmdline).
- **D-05:** `input/felucia/108.ezkvm.qemu.cmd` is **stale** (generated for an older version of ezkvm) and MUST NOT be used as a diff target or correctness oracle in this phase. Treat it as historical/informational only — do not regenerate or "fix" it as part of this phase's own deliverables; it is out of scope. This directly supersedes ROADMAP's original success criterion #2 ("matches 108.ezkvm.qemu.cmd reference output") — the planner should treat that criterion as replaced by D-04's structural spot-check approach for all three configs, uniformly (felucia included).

### the agent's Discretion
- Exact assertion granularity within "structural spot-checks" (e.g., whether to assert exact `-device` argument counts vs. just presence/absence of each device-kind's flag) — the agent should follow whatever level of rigor is already established by Phase 7's `qemu.rs` unit tests, applied per-corpus-file here.
- Whether coruscant/501 and zbp-server-mh2/301's `lspci`/`lsusb` files are parsed programmatically for assertions or just used as human-reference documentation while writing the tests — either is acceptable as long as the resulting assertions are correct.

### Baseline update: Phase 8.1 landed (added during replan, 2026-07-29)

Phase 8.1 ("USB & SCSI Schema Extension") was inserted before this phase and has now shipped
(verified, 7/7 must-haves, 165/165 tests passing). It changes two assumptions this phase's
plans were originally written against — both are corrected here, not relitigated:

- **USB host identity is now a typed `UsbHostIdentity` enum** (`BusPort{bus,port}` /
  `VendorProduct{vendor_id,product_id}`), parsed once at the Proxmox import boundary
  (`src/config/proxmox/importer.rs`) and matched exhaustively (no panic path) in
  `src/config/qemu/handlers/usb.rs::emit_usb_device`. The vendor:product-ID passthrough panic
  that this phase's original planning found in research (real corpus data:
  `zbp-server-mh2/301.conf`'s `usb4`/`usb5`) is **already fixed**, and fixed more thoroughly
  than originally planned (a proper typed enum threaded through Runtime/schema/importer/emitter,
  not a `resource.contains(':')` string-split hack in the emitter). The plan that existed solely
  to apply that string-split hack (`09-02-PLAN.md`) is dropped as redundant — see
  `09-REPLAN-NOTES.md`.
- **`scsihw` (pvscsi / virtio-scsi-pci / virtio-scsi-single) is now fully consumed**, not parsed
  and discarded. `GenericScsiController { controller_type }` covers the shared-bus pvscsi/
  virtio-scsi-pci case; `VirtioScsiSingleDisk` (one per Proxmox `scsiN` index, no shared bus)
  covers virtio-scsi-single. The QEMU emitter (`handlers/pcie.rs`) emits the corresponding
  `-device pvscsi`/`-device virtio-scsi-pci` controller line per corpus file's actual `scsihw`
  value. This phase's tests SHOULD now assert the scsihw-specific device string per corpus file
  (felucia=pvscsi, coruscant=virtio-scsi-pci, zbp-server-mh2=virtio-scsi-single via per-disk
  `virtio-scsi-pci` controllers) — the previous instruction to avoid such an assertion no longer
  applies.

None of D-01 through D-05 (scope, corpus selection, verification method) are affected or
revisited by this update — only the two stale technical assumptions above are corrected.

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Corpus / fixture files
- `input/felucia/108.conf`, `input/felucia/storage.cfg` — primary corpus target (existing, already used by Phase 6/7 tests)
- `input/felucia/108.ezkvm.qemu.cmd` — **STALE, do not use** (see D-05); historical only
- `input/coruscant/501.conf`, `input/coruscant/storage.cfg` — second corpus target (D-03); companion `input/coruscant/501.lspci.txt`, `501.lspci_t.txt`, `501.lspci_v.txt` for hostpci cross-check
- `input/zbp-server-mh2/301.conf`, `input/zbp-server-mh2/storage.cfg` — third corpus target (D-03); companion `input/zbp-server-mh2/301.lspci`, `301.lspci_t.txt`, `301.lspci_v.txt`, `301.lsusb` for hostpci/USB cross-check

### Prior phase decisions relevant here
- `.planning/phases/08-vm-lifecycle/08-CONTEXT.md` — establishes the host-agnostic testing discipline (no real qemu/swtpm/UI-client binaries in tests) that this phase continues (D-01, D-02)
- `.planning/phases/07-qemu-cmdline/07-CONTEXT.md` and `.planning/phases/07-qemu-cmdline/07-VERIFICATION.md` — the existing `QemuCommandLine::try_from((Runtime, QemuContext))` emitter and its drive-before-device/raw-args-verbatim guarantees, which this phase's structural spot-checks build on rather than re-deriving
- `.planning/ROADMAP.md` §"Phase 10: Deployment Packaging - Debian/Ubuntu" — explicitly the deferred home for real-QEMU-binary boot verification (D-01)

### Existing conversion/emission pipeline
- `src/config/proxmox.rs` / `ProxmoxImporter` — `.conf`+`storage.cfg` → Runtime
- `src/config/ezkvm/` — Runtime ↔ ezkvm YAML (`ConfigSchema`)
- `src/config/qemu.rs`, `src/config/qemu/handlers/*.rs` — Runtime → QEMU cmdline (`QemuCommandLine`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ProxmoxImporter`, `EzkvmConfigSchema`, `QemuCommandLine::try_from` — all three pipeline stages already exist and are independently tested (Phases 3/4/5/6/7); this phase only needs to chain them across the three chosen corpus files and assert structural invariants at the end.
- Existing per-phase integration test patterns (e.g., `tests/vm_lifecycle.rs`'s TestDir/fixture conventions) are a reasonable style reference for structuring the new round-trip integration test file, even though this phase spawns no processes.

### Established Patterns
- `TryFrom` chain: `Proxmox → Runtime → ConfigSchema(YAML) → Runtime → QemuCommandLine` — the same four-stage conversion chain already exercised piecemeal in Phases 3-7; this phase is the first to chain all four stages together per corpus file in one test.

### Integration Points
- New integration test file(s) will live under `tests/` (exact name TBD by planner, e.g. `tests/round_trip_verification.rs`), reading corpus files from `input/<corpus>/`.

</code_context>

<specifics>

## Specific Ideas

- The old `input/felucia/108.ezkvm.qemu.cmd` reference file's staleness was flagged directly by the user during discussion — do not resurrect it as a test oracle or attempt to "fix" it to match; it's simply out of scope for this phase.

</specifics>

<deferred>

## Deferred Ideas

- **Real-QEMU boot verification** (spawning an actual `qemu-system-x86_64` process and confirming it reaches the monitor prompt) — belongs entirely to Phase 10, which is explicitly scoped for real-host-tooling verification on Debian 13/Ubuntu 26.04.
- **Regenerating a fresh, current `108.ezkvm.qemu.cmd` reference file** for human-eyeball documentation purposes — user did not request this; noted only as a "could do later" idea, not adopted.

None — discussion stayed within phase scope otherwise.

</deferred>

---

*Phase: 9-Round-Trip-Verification*
*Context gathered: 2026-07-28*
