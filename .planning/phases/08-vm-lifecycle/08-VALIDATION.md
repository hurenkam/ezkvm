---
phase: 8
slug: vm-lifecycle
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-07-29
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust's built-in `cargo test` (existing convention — no `#[test]` framework crate beyond `std`) |
| **Config file** | none — see Wave 0 gaps below |
| **Quick run command** | `cargo test --test <new_test_file> -- --test-threads=1` (process/socket tests should be serialized to avoid port/socket-path collisions) |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds (existing 108-test suite plus new integration tests spawning this phase's `fake_qemu`/`fake_swtpm`/`fake_ui_client` stub binaries — see the Test Independence note below) |

---

## Test Independence (Host-Agnostic Testing)

This phase's entire test suite is deliberately host-agnostic — no test spawns a real `qemu-system-x86_64`, `swtpm`, `remote-viewer`, or `looking-glass-client` binary, and no test assumes a particular Linux distribution's tooling is installed. Instead:

- `src/bin/fake_qemu.rs` (created in `08-01`) stands in for `qemu-system-x86_64`, including a minimal QMP responder for `08-03`'s stop/kill/reset tests.
- `src/bin/fake_swtpm.rs` (created in `08-02`) stands in for `swtpm socket --ctrl type=unixio,path=...`.
- `src/bin/fake_ui_client.rs` (created in `08-02`) stands in for `remote-viewer`/`looking-glass-client`.

All three are pure-Rust Cargo `[[bin]]` targets referenced by tests via `env!("CARGO_BIN_EXE_<name>")` — compiled by the same toolchain as `ezkvm` itself, so the full suite passes identically on Debian, Ubuntu, Fedora, or any other distribution the Rust toolchain targets. `08-03`'s QMP client tests additionally use a test-local `UnixListener`-based fake QMP server, which was already host-independent.

**Real-binary, real-distro verification is out of scope for Phase 8.** Actually installing and exercising `qemu-system-x86_64`, `swtpm`, and `looking-glass-client` against a real Debian 13 / Ubuntu 26.04 target is deferred to **Phase 10 — "Deployment Packaging - Debian/Ubuntu"**, which runs at actual package-creation time. This is a deliberate deferral, not an oversight.

---

## Sampling Rate

- **After every task commit:** Run `cargo test --test <file> -- <test_name>`
- **After every plan wave:** Run `cargo test` (full suite)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 08-01-T1 | 08-01 | 1 | VMGR-01 | T-8-01, T-8-03 | `ezkvm start` launches qemu (TPM-less minimal path) as a detached process and persists a `VmHandle` | integration (tracer) | `cargo test --test vm_lifecycle -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 08-01-T2 | 08-01 | 1 | — | T-8-01 | `<vm-name>` path-traversal rejected before filesystem join, across `start` | unit | `cargo test --test cli_security` | ❌ W0 | ⬜ pending |
| 08-02-T1 | 08-02 | 2 | VMGR-01 | T-8-04, T-8-05 | swtpm readiness poll blocks qemu spawn until socket connectable, times out cleanly; already-running guard (D-10); orphaned-swtpm cleanup (D-17) | integration | `cargo test --test vm_lifecycle -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 08-02-T2 | 08-02 | 2 | VMGR-02 | — | UI client launch mapping picks the right binary per `DisplaySchema` variant (D-11/D-12); launch failure logs a warning, VM keeps running (D-15) | unit + integration | `cargo test --test ui_client_mapping` | ❌ W0 | ⬜ pending |
| 08-03-T1 | 08-03 | 2 | — | T-8-06 | QMP capabilities handshake required before other commands succeed (Pitfall 1); `quit` tolerates EOF-without-response (Pitfall 4); `system_reset` is fire-and-forget (D-21) | unit | `cargo test --test qmp_client` | ❌ W0 | ⬜ pending |
| 08-03-T2 | 08-03 | 2 | VMGR-03, VMGR-04, VMGR-05 | T-8-06, T-8-07 | `stop` sends `system_powerdown`, blocks until qemu PID exits (D-18), opt-in escalation to `kill` (D-19); `kill` sends `quit`; `stop`/`kill` success terminates tracked UI client PID (D-20) | integration | `cargo test --test qmp_client -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 08-04-T1 | 08-04 | 3 | VMGR-01, VMGR-02, VMGR-03 | — | Full lifecycle round trip: start (TPM + UI client) -> stop (UI-client + swtpm cleanup) combining 08-01/08-02/08-03 | integration | `cargo test --test vm_lifecycle -- --test-threads=1` | ❌ W0 | ⬜ pending |
| 08-04-T2 | 08-04 | 3 | — | T-8-01, T-8-03, T-8-06 | QMP socket directory remains `0700` under full combined load; all five verbs reject path-traversal identically | unit + integration | `cargo test --test cli_security && cargo test --test qmp_client` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky. Task/Plan/Wave IDs above reflect the concrete PLAN.md files authored on 2026-07-28 (`08-01-PLAN.md` through `08-04-PLAN.md`). `File Exists` remains `❌ W0` until the executor creates `tests/vm_lifecycle.rs`, `tests/qmp_client.rs`, `tests/ui_client_mapping.rs`, and `tests/cli_security.rs` during execution — planning does not create test files, only specifies them in each PLAN.md's `<behavior>`/`<verify>` blocks.*

---

## Wave 0 Requirements

- [ ] `tests/vm_lifecycle.rs` — new integration test file covering VMGR-01/02/03/04's start/stop/kill/orphan-cleanup behaviors against this phase's `fake_qemu`/`fake_swtpm` stub binaries (see Test Independence note above) — no real `qemu-system-x86_64`/`swtpm` installation required
- [ ] `tests/qmp_client.rs` — new unit/integration test file for the QMP client's handshake, `system_powerdown`/`quit`/`system_reset` framing, and EOF-tolerance behavior (Pitfall 4), tested against a minimal fake QMP server (a test-local `UnixListener`)
- [ ] `tests/ui_client_mapping.rs` — new unit test file for the pure `DisplaySchema` (+ `Ivshmem` Runtime device, for Looking Glass) → binary/args mapping function
- [ ] Framework install: none — `cargo test` is already the project's test runner; only new test *files* are needed

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|--------------------|
| — | — | None — all Phase 8 verifications are automated (see Test Independence note above) | — |

*Looking Glass / UI-client launch is fully covered by automated tests in `08-02` using the `fake_ui_client` stub (D-15's "warn and continue" path is exercised by pointing `remote_viewer_path`/`looking_glass_client_path` at a nonexistent binary, and the happy path by pointing it at `fake_ui_client`). Real `looking-glass-client` binary behavior against a real Debian/Ubuntu host is deferred to Phase 10.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
