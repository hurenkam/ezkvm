---
phase: 08-vm-lifecycle
plan: 04
subsystem: testing
tags: [rust, qmp, lifecycle, security, swtpm, spice]
requires:
  - phase: 08-02
    provides: [TPM readiness polling, UI-client launch/tracking, fake_swtpm, fake_ui_client]
  - phase: 08-03
    provides: [QMP stop/kill/reset, UI-client cleanup, fake_qemu integration coverage]
provides:
  - Full lifecycle integration coverage for start -> stop with TPM and UI client
  - Cross-verb path traversal regression coverage across lifecycle verbs
  - QMP/state-dir security regression coverage plus symlink-safe vm handle persistence
affects: [vm-lifecycle, qmp, security, testing]
tech-stack:
  added: []
  patterns: [host-agnostic lifecycle integration testing, symlink-safe state file handling]
key-files:
  created: [.planning/phases/08-vm-lifecycle/08-04-SUMMARY.md]
  modified: [src/lifecycle/vm_handle.rs, src/lifecycle/stop.rs, src/lifecycle/kill.rs, tests/vm_lifecycle.rs, tests/qmp_client.rs, tests/cli_security.rs, tests/ui_client_mapping.rs]
key-decisions:
  - "Reused fake_qemu/fake_swtpm/fake_ui_client for all wave-3 coverage; no host binaries required."
  - "Hardened VmHandle read/write/remove against symlinked state paths to close the remaining state-file attack noted in research."
patterns-established:
  - "Lifecycle integration tests use short deterministic artifact paths so Unix socket path limits do not cause flakiness."
  - "Orphan cleanup assertions identify stub processes by cmdline fragment tied to the test socket path rather than global process counts."
requirements-completed: [VMGR-01, VMGR-02, VMGR-03, VMGR-04, VMGR-05]
coverage:
  - id: D1
    description: "Full lifecycle start with TPM + Spice UI client and stop cleanup works end-to-end, including persisted tracked PIDs and state-file removal."
    requirement: "VMGR-01"
    verification:
      - kind: integration
        ref: "tests/vm_lifecycle.rs#test_full_lifecycle_start_and_stop"
        status: pass
      - kind: other
        ref: "cargo test --test vm_lifecycle -- --test-threads=1"
        status: pass
    human_judgment: false
  - id: D2
    description: "Already-running and qemu-spawn-failure orphan-cleanup paths hold with the fully populated TPM + UI-client host configuration."
    requirement: "VMGR-03"
    verification:
      - kind: integration
        ref: "tests/vm_lifecycle.rs#test_start_already_running_fails_with_full_lifecycle"
        status: pass
      - kind: integration
        ref: "tests/vm_lifecycle.rs#test_qemu_failure_kills_orphaned_swtpm_with_full_config"
        status: pass
    human_judgment: false
  - id: D3
    description: "Lifecycle security invariants hold across the assembled phase: secure state/socket directory permissions, cross-verb traversal rejection, and symlink-safe state-file handling."
    verification:
      - kind: integration
        ref: "tests/qmp_client.rs#start_creates_qmp_socket_in_secure_state_dir_under_full_config"
        status: pass
      - kind: unit
        ref: "tests/cli_security.rs#resolve_vm_name_rejects_path_traversal_identically_for_all_lifecycle_verbs"
        status: pass
      - kind: unit
        ref: "tests/cli_security.rs#vm_handle_rejects_symlinked_state_files"
        status: pass
    human_judgment: false
duration: pending
completed: 2026-07-28
status: complete
---

# Phase 8 Plan 08-04 Summary

**VM lifecycle now has end-to-end TPM/UI-client integration coverage plus final security regression checks for traversal, socket directories, and state-file symlink defense.**

## Performance

- **Duration:** pending
- **Started:** 2026-07-28T15:32:59+02:00
- **Completed:** 2026-07-28
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Added full start -> stop lifecycle integration coverage that proves qemu, swtpm, and UI client are all tracked and cleaned up together.
- Added full-config regression tests for already-running and orphaned-swtpm cleanup paths.
- Added final security coverage for shared vm-name traversal validation, secure QMP/state directory behavior, and symlink-safe VmHandle persistence.

## Task Commits

No commits were created because this repository has a standing zero-auto-commit policy and this execution honored it.

## Files Created/Modified
- `src/lifecycle/vm_handle.rs` - hardens state-file read/write/remove against symlink attacks with `O_NOFOLLOW` and explicit symlink rejection.
- `src/lifecycle/stop.rs` - cleans up tracked swtpm/socket artifacts after qemu exit as part of the full lifecycle path.
- `src/lifecycle/kill.rs` - mirrors stop cleanup for swtpm/socket artifacts on force-stop.
- `tests/vm_lifecycle.rs` - adds wave-3 lifecycle integration coverage and stabilizes orphan assertions.
- `tests/qmp_client.rs` - adds secure state-dir/QMP socket regression coverage.
- `tests/cli_security.rs` - adds cross-verb traversal and symlink-defense regression coverage.
- `tests/ui_client_mapping.rs` - shortens test artifact paths to keep Unix socket paths stable.
- `.planning/phases/08-vm-lifecycle/08-04-SUMMARY.md` - documents the completed plan.

## Decisions Made
- Hardened `VmHandle` itself instead of only testing for the symlink attack, because the research doc explicitly called out the defensive check and the lifecycle state file drives process termination decisions.
- Shortened test artifact directory names across lifecycle-related tests to stay below Unix socket path limits and eliminate environment-dependent flakiness.

## Deviations from Plan

### Auto-fixed Issues

**1. Security hardening required by integrated verification**
- **Found during:** Task 2
- **Issue:** The state file path had no symlink defense even though the phase research called it out as a remaining threat.
- **Fix:** Added explicit symlink rejection plus `O_NOFOLLOW`-based open behavior in `VmHandle` read/write paths and covered it with automated tests.
- **Files modified:** `src/lifecycle/vm_handle.rs`, `tests/cli_security.rs`
- **Verification:** `cargo test --test cli_security`

**2. Integration cleanup gap surfaced by full round-trip testing**
- **Found during:** Task 1
- **Issue:** `stop`/`kill` removed the handle but did not explicitly clean up tracked swtpm/socket artifacts after qemu exit, which the new end-to-end tests expected.
- **Fix:** Extended lifecycle cleanup to terminate tracked swtpm and remove qmp/tpm socket files after successful stop/kill.
- **Files modified:** `src/lifecycle/stop.rs`, `src/lifecycle/kill.rs`, `tests/vm_lifecycle.rs`, `tests/qmp_client.rs`
- **Verification:** `cargo test --test vm_lifecycle -- --test-threads=1`, `cargo test --test qmp_client`

---

**Total deviations:** 2 auto-fixed
**Impact on plan:** Both changes were directly required to satisfy the plan's acceptance criteria and threat model; no unrelated scope was added.

## Issues Encountered
- Initial lifecycle tests were flaky because long test-artifact paths exceeded Unix-domain socket length limits for swtpm sockets. Shortening artifact directory names fixed this without altering production behavior.
- Orphan-cleanup assertions based on global fake_swtpm PID sets were brittle when unrelated stub processes existed; matching by socket-path cmdline fragment made the checks deterministic.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Phase 8 is ready for `/gsd-verify-work` with full lifecycle and security coverage in place.
- `STATE.md` and `ROADMAP.md` were not modified; all changes remain unstaged/uncommitted for the orchestrator/user to review.

---
*Phase: 08-vm-lifecycle*
*Completed: 2026-07-28*
