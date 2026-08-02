---
phase: 09-round-trip-verification
verified: 2026-07-29T15:41:00Z
status: passed
score: 8/8 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: No — initial verification (post-replan)
---

# Phase 9: Round-Trip Verification — Verification Report

**Phase Goal:** The complete felucia/108.conf pipeline — `.conf` + `storage.cfg` → Runtime → ezkvm
YAML → Runtime → QEMU cmdline — produces a QEMU commandline that starts a working VM, verified by
integration tests across multiple corpus files. (Re-scoped per `09-CONTEXT.md` D-01/D-04: "starts a
working VM" is verified via structural spot-checks, not a real QEMU boot — real boot is explicitly
deferred to Phase 10.)

**Verified:** 2026-07-29T15:41:00Z
**Status:** passed
**Re-verification:** No — initial verification, executed against the replanned 2-plan baseline
(09-01 + 09-03, 09-02 dropped as redundant per `09-REPLAN-NOTES.md`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | felucia/108 imports, round-trips through ezkvm YAML, and generates a QEMU cmdline with zero errors/panics | ✓ VERIFIED | `tests/round_trip_verification.rs:61-161`; independently re-run: `cargo test --test round_trip_verification felucia -- --nocapture` → `ok` |
| 2 | felucia cmdline places every `-drive` before its referencing `-device` (scsi0, ide2) | ✓ VERIFIED | Lines 89-90, `assert_precedes(&output, "id=drive-scsi0", "drive=drive-scsi0")` / same for ide2; test passes |
| 3 | felucia cmdline contains the RawArgs `args:` blob verbatim, exactly once, after every real emitted device | ✓ VERIFIED | Lines 92-114, uses `cmdline.devices().last()` + bounded `rfind` (avoids the naive-whole-output false-positive trap called out in RESEARCH.md Pitfall 4); test passes |
| 4 | felucia cmdline contains the `pvscsi`-specific shared SCSI controller device string (Phase 8.1 capability) | ✓ VERIFIED | Line 156: `output.matches("-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5").count() == 1`; cross-checked against `src/config/qemu/handlers/pcie.rs::emit_scsi_controller` (`ScsiControllerType::PvScsi => "pvscsi"`) — assertion matches actual emitter logic, not a coincidental string match |
| 5 | coruscant/501 imports, round-trips, and generates a cmdline with zero errors/panics — including 3 hostpci devices, 3 scsi disks on a shared `virtio-scsi-pci` controller, and multi-line ivshmem RawArgs | ✓ VERIFIED | `tests/round_trip_verification.rs:164-258`; `cargo test --test round_trip_verification coruscant -- --nocapture` → `ok`; asserts `vfio-pci` count == 3, `virtio-scsi-pci,id=scsihw0,...` == 1, 3 `bus=scsihw0.0` attachments, RawArgs verbatim-once-after-devices, no fabricated `net0` |
| 6 | zbp-server-mh2/301 imports, round-trips, and generates a cmdline with zero errors/panics — including 6 usb devices (4 bus-port + 2 vendor:product-ID) and 4 per-instance `virtio-scsi-single` controllers | ✓ VERIFIED | `tests/round_trip_verification.rs:260-342`; `cargo test --test round_trip_verification zbp -- --nocapture` → `ok`; asserts `usb-host` count == 6, `hostbus=` count == 4, literal `vendorid=0x0451`/`productid=0x16a0` and `vendorid=0x0403`/`productid=0x6001`, 4× `-device virtio-scsi-pci,id=scsihw{N}` with matching iothreads and drive-before-device ordering |
| 7 | Each corpus test asserts the scsihw-controller-specific device string actually emitted for that corpus's declared `scsihw` value (not generic presence-only) | ✓ VERIFIED | felucia → literal `pvscsi` (Truth 4); coruscant → literal `virtio-scsi-pci,id=scsihw0,bus=pci.0,addr=0x5` (line 199); zbp-server-mh2 → 4× literal `virtio-scsi-pci,id=scsihw{N}` (line 314+320) — cross-checked against `pcie.rs::emit_scsi_controller`/`emit_virtio_scsi_single` source, and against `usb.rs::emit_usb_device`'s exact `vendorid=`/`productid=`/`hostbus=`/`hostport=` flag formatting — assertions match the real emitter code, not guesses |
| 8 | No test references the stale `input/felucia/108.ezkvm.qemu.cmd` file (D-05) | ✓ VERIFIED | `grep -n "108.ezkvm.qemu.cmd" tests/round_trip_verification.rs` → no matches (exit 1) |

**Score:** 8/8 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `tests/round_trip_verification.rs` | New integration test file, 3 tests, chaining all 4 pipeline stages per corpus | ✓ VERIFIED | File exists, 342 lines, contains exactly 3 `#[test]` functions (`felucia_108_...`, `coruscant_501_...`, `zbp_server_mh2_301_...`), plus `load_corpus`/`runtime_for_cmdline`/`make_ctx`/`assert_precedes` helpers, all wired and exercised |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `load_corpus()` | `ProxmoxImporter::new(...).into_runtime()` | direct call | ✓ WIRED | All 3 tests call this chain; confirmed passing |
| Runtime (post-import) | `EzkvmConfigSchema::try_from` → `.to_styled_compact_yaml()` → `EzkvmConfigSchema::from_str` → `Runtime::try_from` | YAML round-trip stage | ✓ WIRED | All 3 tests exercise this chain; non-empty YAML asserted |
| `runtime_for_cmdline()` (EfiDisk-excluded) | `QemuCommandLine::try_from((runtime, make_ctx(...)))` | direct call | ✓ WIRED | All 3 tests exercise this chain; cmdline stringified and asserted |
| felucia's `scsihw: pvscsi` | `ScsiControllerType::PvScsi` → `emit_scsi_controller` → literal `pvscsi` string | `src/config/qemu/handlers/pcie.rs:79` | ✓ WIRED | Source inspected directly; matches test assertion exactly |
| coruscant's `scsihw: virtio-scsi-pci` | `GenericScsiController` → `emit_scsi_controller` → literal `virtio-scsi-pci` string | `src/config/qemu/handlers/pcie.rs:80` | ✓ WIRED | Source inspected directly; matches test assertion exactly |
| zbp-server-mh2's `scsihw: virtio-scsi-single` | 4× `VirtioScsiSingleDisk` → `emit_virtio_scsi_single` → per-instance `virtio-scsi-pci,id=scsihw{N}` | `src/config/qemu/handlers/pcie.rs:117-126` | ✓ WIRED | Source inspected directly; matches test assertion exactly |
| zbp-server-mh2's usb4/usb5 (`host=0451:16a0`, `host=0403:6001`) | `UsbHostIdentity::VendorProduct` → `emit_usb_device` → literal `vendorid=0x.../productid=0x...` | `src/config/qemu/handlers/usb.rs:26-33` | ✓ WIRED | Source inspected directly; exhaustive match, no panic path; matches test assertion exactly |

### Behavioral Spot-Checks (independently executed by verifier, not copied from SUMMARY)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| New round-trip test file passes | `cargo test --test round_trip_verification -- --nocapture` | `3 passed; 0 failed` (felucia, coruscant, zbp-server-mh2) | ✓ PASS |
| Full suite has no regressions | `cargo test` (full run) | 15 test binaries, all `ok`; summed count = 168 passed, 0 failed | ✓ PASS |
| Stale reference file not touched | `grep -n "108.ezkvm.qemu.cmd" tests/round_trip_verification.rs` | no match (exit 1) | ✓ PASS |
| Git state clean of executor/verifier commits | `git status --porcelain`, `git diff --stat HEAD` | 0 tracked-file changes; only new untracked files (`tests/round_trip_verification.rs`, 2 SUMMARY.md); `git diff --stat HEAD` empty | ✓ PASS |

**Independently re-derived counts (not copied from SUMMARY.md):**
- `tests/round_trip_verification.rs` alone: **3 passed, 0 failed** (matches both plans' claims)
- Full workspace `cargo test`: binaries and their counts — `cli_security` 5, `proxmox_import` 7, `qemu_cmdline` 5, `qmp_client` 15, `round_trip_verification` 3, `runtime_phase2` 5, `ui_client_mapping` 6, `vm_lifecycle` 12, `yaml_round_trip` 8, unit-test binary 102, plus 4 zero-test binaries and 1 doc-test binary (0 tests) = **168 passed, 0 failed** total. This matches 09-03-SUMMARY.md's claimed "168 passing tests, 0 failures" — independently confirmed, not merely trusted.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| QEMU-04 | 09-01, 09-03 | "Generated commandline for felucia/108.conf produces a VM that starts in QEMU" | ✓ SATISFIED (per D-01 reinterpretation) | Structural round-trip fully proven for all 3 corpus files; **note:** `.planning/REQUIREMENTS.md` line 42 still shows `- [ ] **QEMU-04**` (unchecked) with its original literal wording ("produces a VM that starts in QEMU"). This is **consistent, not a gap**: `09-CONTEXT.md` D-01 explicitly defers real-QEMU-process boot verification to Phase 10, and ROADMAP's Phase 9 success criterion #3 says the same. The requirement's literal "starts in QEMU" clause is intentionally left unchecked pending Phase 10; only the structural/round-trip portion is this phase's job, and that portion is now fully covered. See "Findings" below for a documentation-hygiene recommendation. |

### Anti-Patterns Found

None. Scanned `tests/round_trip_verification.rs` for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`/empty-stub patterns — no matches. All assertions are concrete literal-string/count checks tied to real emitter output, not hardcoded-empty stubs. No `src/` files were touched by this phase (confirmed via `git diff --stat HEAD` — empty).

### Data-Flow Trace

N/A — this phase's artifact is a test file, not a UI/data-rendering component. Level 4 trace not applicable; Level 1-3 (exists/substantive/wired) fully covered above, and the "data" in question (emitted device strings) was traced to its actual source (`pcie.rs`/`usb.rs` emitter code) in the Key Link Verification section, which serves the same purpose.

### Human Verification Required

None. All must-haves are structural/string assertions independently re-run and confirmed by the
verifier; no visual, real-time, or external-service behavior is in scope for this phase (per D-01/D-02,
no real QEMU/swtpm process is spawned).

### Gaps Summary

No gaps. All 8 derived must-have truths verified against live code (not SUMMARY narrative):
tests exist, are substantive (real literal-string assertions cross-checked against the actual
emitter source, not guesses), are wired into a real 4-stage pipeline call chain, execute
successfully both in isolation (`--test round_trip_verification`) and as part of the full 168-test
suite, and correctly avoid the stale `108.ezkvm.qemu.cmd` reference file per D-05. Git state is
clean of any commits — 09-01/09-03 executors made no commits (per their own "standing policy" note,
independently confirmed via `git status`/`git diff --stat HEAD`), and this verification made none
either (only read-only git commands were run).

**One documentation-hygiene finding (non-blocking, informational):** `.planning/REQUIREMENTS.md`
line 42's QEMU-04 entry retains its pre-D-01 literal wording ("produces a VM that starts in QEMU")
and remains unchecked (`[ ]`). This is factually correct given D-01's deferral of real-boot
verification to Phase 10, but a future documentation pass (e.g., during Phase 10 planning or
completion) should either reword QEMU-04 to explicitly split "structural round-trip" (satisfied,
Phase 9) from "real QEMU boot" (Phase 10) sub-clauses, or add an inline note pointing to this
VERIFICATION.md, so a future reader doesn't mistake the unchecked box for an unaddressed gap in
Phase 9. This does not block Phase 9 from being considered complete — it is purely a
requirements-ledger clarity suggestion for whoever picks up Phase 10.

---

_Verified: 2026-07-29T15:41:00Z_
_Verifier: the agent (gsd-verifier)_
