---
phase: 08-vm-lifecycle
verified: 2026-07-28T13:50:28Z
status: passed
score: 6/6 must-haves verified
behavior_unverified: 0
gaps: []
closed_gaps:
  - truth: "`kill` satisfies VMGR-04 exactly: QMP `quit` with SIGTERM fallback if monitor path fails or qemu does not exit"
    closed_at: 2026-07-28T14:30:00Z
    resolution: "Rewrote src/lifecycle/kill.rs: QMP connect failure or a quit that does not result in exit within QUIT_GRACE_PERIOD now falls back to terminate_process(qemu_pid, false) (SIGTERM), matching VMGR-04's literal wording. SIGKILL is retained only as a bounded last-resort escalation after the SIGTERM fallback also times out (TERM_GRACE_PERIOD), preserving robustness without violating the requirement. Added FAKE_QEMU_IGNORE_QUIT to src/bin/fake_qemu.rs to simulate an unresponsive quit for testing. Added two new tests to tests/qmp_client.rs: kill_falls_back_to_sigterm_when_qmp_connect_fails (missing/stale QMP socket) and kill_falls_back_to_sigterm_when_quit_does_not_exit_in_time (quit acknowledged but ignored). Full suite now 148/148 passing (was 144/144), 0 regressions."
deferred: []
---

# Phase 8: VM Lifecycle Verification Report

**Phase Goal:** ezkvm can start, stop, and reset a running VM by managing `qemu-system-x86_64` and `swtpm` child processes, communicate with QEMU via its monitor socket, and launch the configured UI client after VM start.
**Verified:** 2026-07-28T13:50:28Z
**Status:** passed
**Re-verification:** Yes — VMGR-04 gap closed 2026-07-28T14:30:00Z

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `start` launches detached qemu, starts `swtpm` first when TPM is configured, and polls the swtpm socket before qemu spawn | ✓ VERIFIED | `src/lifecycle/process.rs` uses `setsid()` detached spawn; `src/lifecycle/start.rs:111-136` starts `swtpm` and calls `readiness::wait_for_socket(...)` before `qemu`; exercised by `tests/vm_lifecycle.rs:296-346` and `381-431` |
| 2 | After start, the mapped UI client is auto-launched after a fixed delay; launch failure does not kill VM | ✓ VERIFIED | `src/lifecycle/start.rs:163-174` sleeps 2s then best-effort spawns UI client; `src/lifecycle/ui_client.rs` maps Spice/VNC/Looking Glass; tested by `tests/ui_client_mapping.rs:206-262` |
| 3 | `stop` sends `system_powerdown` over QMP, blocks for qemu exit, and does not auto-escalate unless configured | ✓ VERIFIED | `src/lifecycle/qmp.rs:83-96` sends `system_powerdown`; `src/lifecycle/stop.rs:47-71` waits until PID exit and only escalates when `stop_escalation_timeout_secs` is set; tested by `tests/qmp_client.rs:363-379,447-494` |
| 4 | `kill` satisfies VMGR-04 exactly: QMP `quit` with SIGTERM fallback if monitor path fails or qemu does not exit | ✓ VERIFIED | `src/lifecycle/kill.rs` falls back to `terminate_process(qemu_pid, false)` (SIGTERM) both when QMP connect fails and when `quit` doesn't cause exit in time; SIGKILL is only a bounded last resort. Tested by `tests/qmp_client.rs::kill_falls_back_to_sigterm_when_qmp_connect_fails` and `::kill_falls_back_to_sigterm_when_quit_does_not_exit_in_time` |
| 5 | `reset` sends `system_reset` without waiting for a reply and leaves the VM running | ✓ VERIFIED | `src/lifecycle/qmp.rs:118-123` is fire-and-forget; `src/lifecycle/reset.rs:7-11` returns immediately; tested by `tests/qmp_client.rs:398-418` |
| 6 | QMP-over-Unix-socket control and lifecycle state security are implemented with host-agnostic test coverage | ✓ VERIFIED | QMP handshake/capabilities enforced in `src/lifecycle/qmp.rs:35-59`; state dir locked to `0700` in `src/lifecycle/process.rs`; YAML state + symlink defense in `src/lifecycle/vm_handle.rs:47-130`; tested by `tests/qmp_client.rs:497-534` and `tests/cli_security.rs:61-173` |

**Score:** 6/6 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/lifecycle/start.rs` | Start flow with qemu/swtpm/UI launch | ✓ EXISTS + SUBSTANTIVE | Implements validation, stale cleanup, swtpm readiness poll, qemu spawn, UI launch, YAML state persistence |
| `src/lifecycle/qmp.rs` | QMP handshake + commands | ✓ EXISTS + SUBSTANTIVE | Implements greeting read, `qmp_capabilities`, `system_powerdown`, `quit`, `system_reset` |
| `src/lifecycle/stop.rs` | Graceful stop flow | ✓ EXISTS + SUBSTANTIVE | Waits for qemu exit, optional escalation, cleanup of tracked processes and socket files |
| `src/lifecycle/kill.rs` | Force-stop flow | ✓ EXISTS + SUBSTANTIVE | Sends `quit`; falls back to SIGTERM on connect failure or non-exit; SIGKILL only as bounded last resort |
| `src/lifecycle/reset.rs` | Reset flow | ✓ EXISTS + SUBSTANTIVE | Fire-and-forget QMP reset |
| `tests/vm_lifecycle.rs` | Full lifecycle integration coverage | ✓ EXISTS + SUBSTANTIVE | Covers start/status, TPM ordering, orphan cleanup, full lifecycle round-trip |
| `tests/qmp_client.rs` | QMP client + lifecycle control tests | ✓ EXISTS + SUBSTANTIVE | Covers handshake, EOF tolerance, stop/kill/reset integration, escalation behavior |
| `tests/ui_client_mapping.rs` | UI mapping and launch coverage | ✓ EXISTS + SUBSTANTIVE | Covers Spice/VNC/Looking Glass mapping and warning-on-failure |
| `tests/cli_security.rs` | Traversal + symlink defense coverage | ✓ EXISTS + SUBSTANTIVE | Covers vm-name rejection, CLI error shape, symlinked state-file rejection |

**Artifacts:** 9/9 verified

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| `main.rs` | `start()` | `Verb::Start` dispatch | ✓ WIRED | `src/main.rs:61-65` |
| `main.rs` | `stop()` with override | `Verb::Stop` dispatch | ✓ WIRED | `src/main.rs:80-102` builds override host config for `--escalate-after` |
| `start()` | swtpm readiness | `wait_for_socket()` | ✓ WIRED | `src/lifecycle/start.rs:111-136` |
| `start()` | UI launch mapping | `resolve_ui_client()` | ✓ WIRED | `src/lifecycle/start.rs:140,163-174`; `src/lifecycle/ui_client.rs` |
| `stop()` / `kill()` / `reset()` | QMP socket | `QmpClient::connect(...)` | ✓ WIRED | `src/lifecycle/stop.rs:47`, `kill.rs:22`, `reset.rs:9` |
| `VmHandle` | YAML persistence | `crate::serde_yaml::{to_string,from_str}` | ✓ WIRED | `src/lifecycle/vm_handle.rs:54-65,84-95` |
| tests | stub binaries only | `env!("CARGO_BIN_EXE_fake_...")` | ✓ WIRED | `tests/vm_lifecycle.rs`, `tests/qmp_client.rs`, `tests/ui_client_mapping.rs`; fake binaries defined in `src/bin/fake_qemu.rs`, `fake_swtpm.rs`, `fake_ui_client.rs` |

**Wiring:** 7/7 connections verified

## Requirements Coverage

| Requirement | Status | Blocking Issue |
|-------------|--------|----------------|
| VMGR-01: ezkvm starts a VM by launching `qemu-system-x86_64` (and `swtpm` when TPM is configured) as child processes | ✓ SATISFIED | - |
| VMGR-02: After VM start, ezkvm launches the configured UI client | ✓ SATISFIED | - |
| VMGR-03: ezkvm gracefully shuts down a running VM via QEMU monitor `system_powerdown` | ✓ SATISFIED | - |
| VMGR-04: ezkvm force-stops a running VM via QEMU monitor `quit` (or SIGTERM fallback) | ✓ SATISFIED | - |
| VMGR-05: ezkvm resets a running VM via QEMU monitor `system_reset` | ✓ SATISFIED | - |

**Coverage:** 5/5 requirements satisfied

## Decision Spot-Check

| Decision | Status | Evidence |
|----------|--------|----------|
| D-07 YAML state file, not `.pid` files | ✓ VERIFIED | `VmHandle` serializes/deserializes with `crate::serde_yaml` in `src/lifecycle/vm_handle.rs:54-65,91-95` |
| D-09 stale detection is PID-liveness only | ✓ VERIFIED | `VmHandle::is_stale()` is `!process::is_pid_alive(self.qemu_pid)` in `src/lifecycle/vm_handle.rs:108-110` |
| D-10 already-running guard | ✓ VERIFIED | `src/lifecycle/start.rs:71-80`; tested by `tests/vm_lifecycle.rs:296-346,465-506` |
| D-14 swtpm poll-not-sleep | ✓ VERIFIED | `src/lifecycle/readiness.rs` plus `src/lifecycle/start.rs:126-133`; tested by `tests/vm_lifecycle.rs:225-248` |
| D-16 fire-and-forget/no supervisor | ✓ VERIFIED | `spawn_detached()` uses `setsid()` in `src/lifecycle/process.rs`; `start()` returns after child launch/state write |
| D-17 orphan swtpm cleanup on qemu spawn failure | ✓ VERIFIED | `src/lifecycle/start.rs:152-160`; tested by `tests/vm_lifecycle.rs:348-378,434-463` |
| D-18 `stop` blocks, no default auto-escalation | ✓ VERIFIED | `src/lifecycle/stop.rs:50-71`; tested by `tests/qmp_client.rs:447-475` |
| D-19 explicit stop escalation override | ✓ VERIFIED | `src/main.rs:80-102`; `src/lifecycle/stop.rs:50-68`; tested by `tests/qmp_client.rs:478-494` |
| D-20 UI-client cleanup on stop/kill | ✓ VERIFIED | `cleanup_after_exit()` terminates tracked UI PID in `src/lifecycle/stop.rs:101-126`; tested by `tests/qmp_client.rs:421-444` and `tests/vm_lifecycle.rs:381-431` |
| D-21 reset fire-and-forget | ✓ VERIFIED | `src/lifecycle/qmp.rs:118-123`, `src/lifecycle/reset.rs:7-11`; tested by `tests/qmp_client.rs:308-330,398-418` |

## Host-Agnostic Test Audit

Verified: no final Phase 8 test shells out to a real `qemu-system-x86_64`, `swtpm`, `looking-glass-client`, or `remote-viewer` binary.

- Stub binaries exist at `src/bin/fake_qemu.rs`, `src/bin/fake_swtpm.rs`, and `src/bin/fake_ui_client.rs`.
- Executed lifecycle tests use `env!("CARGO_BIN_EXE_fake_qemu")`, `env!("CARGO_BIN_EXE_fake_swtpm")`, and `env!("CARGO_BIN_EXE_fake_ui_client")` (`tests/vm_lifecycle.rs:304-307,389-392`; `tests/qmp_client.rs:123-142,501-508`; `tests/ui_client_mapping.rs:236-242`).
- Some fixtures still serialize placeholder real paths into `host.yaml` for non-spawned fields, but no test process invocation targets those binaries.

## Security Verification

| Threat / Requirement | Status | Evidence |
|----------------------|--------|----------|
| `<vm-name>` path traversal rejection | ✓ VERIFIED | `HostConfig::resolve_vm_name()` rejects dot-prefix, `/`, and `..` in `src/lifecycle/host_config.rs:104-110`; CLI and shared-function tests in `tests/cli_security.rs:61-139` |
| State/socket directory permission restriction | ✓ VERIFIED | `ensure_dir_secure()` sets `0700` in `src/lifecycle/process.rs`; checked in `tests/vm_lifecycle.rs:286-287` and `tests/qmp_client.rs:521-524` |
| Symlink-attack defensive check on state file | ✓ VERIFIED | `reject_symlink()` + `O_NOFOLLOW` in `src/lifecycle/vm_handle.rs:52-65,70-82,98-130`; tested in `tests/cli_security.rs:141-173` |

## Anti-Patterns Found

None remaining. The prior blocker (SIGKILL-only fallback in `src/lifecycle/kill.rs`) was fixed; see Closed Gaps in frontmatter.

**Anti-patterns:** 0 found

## Human Verification Required

None — all checked items were verifiable programmatically or by code inspection.

## Gaps Summary

None. All previously identified gaps are closed — see `closed_gaps` in frontmatter.

## Verification Metadata

**Verification approach:** Goal-backward (derived from ROADMAP Phase 8 goal and success criteria)
**Must-haves source:** ROADMAP Phase 8 goal/success criteria + REQUIREMENTS VMGR-01..05 + CONTEXT locked decisions
**Automated checks:** `cargo build` passed; `cargo test` passed with 148 passed, 0 failed (was 144/144 prior to gap closure)
**Human checks required:** 0
**Total verification time:** ~22 minutes (initial) + gap closure fix/test pass

---
*Verified: 2026-07-28T13:50:28Z*
*Verifier: gsd-verifier subagent (initial pass); gap closed directly in follow-up*
