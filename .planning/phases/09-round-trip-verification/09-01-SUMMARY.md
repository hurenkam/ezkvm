---
phase: 09-round-trip-verification
plan: 01
subsystem: tests
tags: [rust, integration-test, proxmox, yaml, qemu, round-trip]
requires:
  - phase: 06-yaml-runtime
    provides: [Runtime to ezkvm YAML round-trip pipeline]
  - phase: 07-qemu-cmdline
    provides: [QemuCommandLine emission, EfiDisk workaround pattern, RawArgs ordering assertions]
  - phase: 08.1-usb-scsi-schema-extension
    provides: [typed USB identities, scsihw-specific SCSI controller emission]
provides:
  - felucia/108 four-stage round-trip integration coverage in tests/round_trip_verification.rs
  - reusable load_corpus, runtime_for_cmdline, and make_ctx helpers for later Phase 9 plans
  - structural assertions for drive-before-device ordering, RawArgs verbatim placement, TPM presence, USB host passthrough, vfio-pci count, and pvscsi controller emission
affects: [phase-09-round-trip-verification, integration-tests]
tech-stack:
  added: []
  patterns:
    - corpus-backed four-stage integration round-trip test
    - EfiDisk exclusion immediately before QEMU emission to avoid known importer gap
    - RawArgs verbatim-at-end assertion using cmdline.devices().last()
key-files:
  created:
    - tests/round_trip_verification.rs
    - .planning/phases/09-round-trip-verification/09-01-SUMMARY.md
  modified: []
key-decisions:
  - "Reused the existing EfiDisk exclusion workaround only at the final QEMU stage so YAML still round-trips the imported EfiDisk."
  - "Asserted the exact TPM device string emitted today by handlers/root.rs (`tpm-tis`) and the exact Phase 8.1 pvscsi controller string emitted by handlers/pcie.rs."
  - "Kept the new test self-contained in tests/round_trip_verification.rs instead of introducing shared test-support modules."
patterns-established:
  - "Future Phase 09 corpus tests should extend tests/round_trip_verification.rs using load_corpus/runtime_for_cmdline/make_ctx helpers."
requirements-completed: [QEMU-04]
coverage:
  - id: D1
    description: "felucia/108 completes Proxmox import -> YAML round trip -> QEMU cmdline emission with no errors or panics."
    requirement: "QEMU-04"
    verification:
      - kind: integration
        ref: "tests/round_trip_verification.rs#felucia_108_full_round_trip_produces_valid_qemu_cmdline"
        status: pass
      - kind: other
        ref: "cargo test --test round_trip_verification felucia -- --nocapture"
        status: pass
      - kind: other
        ref: "cargo test"
        status: pass
    human_judgment: false
completed: 2026-07-29
status: complete
---

# Phase 9 Plan 01 Summary

**Added the felucia/108 end-to-end round-trip integration test covering Proxmox import, ezkvm YAML serialization/parsing, Runtime rebuild, and QEMU cmdline emission with structural cmdline assertions.**

## Performance

- **Completed:** 2026-07-29
- **Tasks:** 1
- **Files created:** 2

## Accomplishments
- Created `tests/round_trip_verification.rs` with reusable `load_corpus`, `runtime_for_cmdline`, and `make_ctx` helpers.
- Added `felucia_108_full_round_trip_produces_valid_qemu_cmdline` to prove the full four-stage pipeline succeeds on real corpus data.
- Asserted drive-before-device ordering for `scsi0` and `ide2`, RawArgs verbatim/exactly-once placement, 2 `vfio-pci` devices, 1 `usb-host` device with `hostbus=1` and `hostport=2.2`, TPM device presence, and the exact `pvscsi` controller line.

## Task Commits

No commits were created. Per the user's standing policy, no `git add`, `git commit`, `git push`, or any other git-mutating command was run.

## Files Created/Modified
- `tests/round_trip_verification.rs` - new Phase 9 tracer integration test for felucia/108.
- `.planning/phases/09-round-trip-verification/09-01-SUMMARY.md` - execution summary for this plan.

## Decisions Made
- Reused the Phase 7 `RootDeviceKind::EfiDisk` exclusion workaround only after the YAML round trip, matching Phase 9 research Pitfall 1.
- Used `cmdline.devices().last()` plus a bounded `rfind` to avoid RawArgs false positives from embedded `-device` substrings.
- Read the live emitter code before asserting device strings, so the test now checks `-device tpm-tis,tpmdev=tpmdev` and `-device pvscsi,id=scsihw0,bus=pci.0,addr=0x5` exactly as emitted.

## Deviations from Plan

None.

## Issues Encountered

- `cargo test` still reports a pre-existing warning in `tests/proxmox_import.rs` for an unused `std::sync::Arc` import. This did not affect test success and was left untouched because it is outside this plan's scope.

## User Setup Required

None.

## Next Phase Readiness

- `tests/round_trip_verification.rs` now has the helper pattern Plan 09-03 can extend for coruscant/501 and zbp-server-mh2/301.
- The working tree is ready for user review and manual commit if desired.
