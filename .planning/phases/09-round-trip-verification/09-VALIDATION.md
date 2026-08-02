---
phase: 09
slug: round-trip-verification
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-28
---

# Phase 09 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Materialized from `09-RESEARCH.md`'s `## Validation Architecture` section during the
> post-Phase-8.1 replan (see `09-REPLAN-NOTES.md`). Only 09-01 and 09-03 remain in scope;
> 09-02 was dropped as fully superseded by Phase 8.1's landed `UsbHostIdentity` fix.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` via `cargo test` (integration tests under `tests/`) |
| **Config file** | none — no external test config; Cargo's own test harness |
| **Quick run command** | `cargo test --test round_trip_verification` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** `cargo test --test round_trip_verification`
- **After every plan wave:** `cargo test` (full suite — currently 165 tests, per Phase 8.1's verified baseline)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 09-01 Task 1 | 09-01 | 1 | QEMU-04 | N/A | felucia/108 full 4-stage round trip completes with zero errors/panics; drive-before-device ordering holds; RawArgs verbatim once; `pvscsi`-specific controller string asserted (post-8.1) | integration | `cargo test --test round_trip_verification felucia -- --nocapture` | ❌ W0 | ⬜ pending |
| 09-03 Task 1 | 09-03 | 2 | QEMU-04 | N/A | coruscant/501 full 4-stage round trip completes with zero errors/panics, incl. 3 hostpci devices, 3 scsi disks, multi-line ivshmem RawArgs; `virtio-scsi-pci`-specific controller string asserted (post-8.1) | integration | `cargo test --test round_trip_verification coruscant -- --nocapture` | ❌ W0 | ⬜ pending |
| 09-03 Task 2 | 09-03 | 2 | QEMU-04 | N/A | zbp-server-mh2/301 full 4-stage round trip completes with zero errors/panics, incl. 6 usb devices (4 bus-port + 2 vendor:product-ID via `UsbHostIdentity`); 4 per-instance `virtio-scsi-single` controller strings asserted (post-8.1) | integration | `cargo test --test round_trip_verification zbp -- --nocapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/round_trip_verification.rs` — new file covering QEMU-04 across all three corpus configs; created by Plan 09-01 Task 1, extended by Plan 09-03 Tasks 1–2
- [ ] `load_corpus()` / `runtime_for_cmdline()` / `make_ctx()` helpers — created by Plan 09-01, reused verbatim by Plan 09-03
- [x] Production-code prerequisite (USB vendor:product-ID panic fix) — **already satisfied**, landed via Phase 8.1's `UsbHostIdentity` enum (not a Wave 0 gap for this phase anymore; the original 09-02 plan that would have fixed it in-phase has been dropped as redundant)
- No new shared fixtures/conftest-equivalent needed beyond the above — per-file `load_corpus()`-style helpers follow the existing established pattern (see `tests/qemu_cmdline.rs`, `tests/yaml_round_trip.rs`)

---

## Manual-Only Verifications

*None — all phase behaviors have automated verification (structural spot-checks per `09-CONTEXT.md` D-04; no UI, no external I/O, no real QEMU/swtpm process spawned per D-01/D-02).*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands — confirmed across both remaining plans (09-01, 09-03)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (only 3 tasks total across both plans)
- [x] Wave 0 covers all MISSING references — `tests/round_trip_verification.rs` + helpers (Plan 09-01 Task 1); production-code prerequisite already satisfied by Phase 8.1
- [x] No watch-mode flags
- [x] Feedback latency < 15s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved (planning-time Nyquist compliance confirmed; per-task Status/File Exists in the verification map above remain ⬜ pending until Plan execution)
