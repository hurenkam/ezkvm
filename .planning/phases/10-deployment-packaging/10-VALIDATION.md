---
phase: 10
slug: deployment-packaging
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-29
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Materialized from `10-RESEARCH.md`'s `## Validation Architecture` section.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust's built-in `#[test]`/`cargo test` for existing tests (unchanged); this phase ADDS a non-`cargo-test` layer — shell-driven Docker/GitHub-Actions verification, since real package install + real QEMU boot is not expressible as a plain Rust unit test |
| **Config file** | none new for the Rust side; new `.github/workflows/package-verify.yml` for the packaging/boot-verification side |
| **Quick run command** | `cargo deb --target x86_64-unknown-linux-musl` (build-only smoke test; confirms packaging metadata is well-formed without a full container run) |
| **Full suite command** | The GitHub Actions matrix job (4 containers × install + `ezkvm start`/`status`/`stop`/`kill`/`reset` cycle) |
| **Estimated runtime** | ~10-30s for the quick build smoke test; several minutes for the full 4-target container matrix |

---

## Sampling Rate

- **After every task commit:** `cargo deb --target x86_64-unknown-linux-musl` (fast — confirms packaging metadata parses and the binary still builds for the musl target; does not require Docker/KVM)
- **After every plan wave:** Full 4-target Docker/KVM matrix (the actual `package-verify.yml` workflow)
- **Before `/gsd-verify-work`:** All 4 targets green (install + full start/status/stop/kill/reset cycle); felucia/108's real-hardware boot remains an explicitly separate, manual, non-blocking verification note (cannot gate an automated phase-completion on hardware this project's CI will never have)
- **Max feedback latency:** several minutes (container matrix) — acceptable given this phase's nature (real package/OS verification, not unit tests)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 10-01 Task 1 | 10-01 | 1 | DEPLOY-01 | V4 (0750/0770 dirs) | `cargo-deb` musl packaging config + postinst/postrm creating `ezkvm` group and FHS dirs with restrictive permissions | build smoke test | `cargo deb --target x86_64-unknown-linux-musl` | ❌ W0 | ⬜ pending |
| 10-01 Task 2 | 10-01 | 1 | QEMU-04, DEPLOY-04 | N/A | Single-target (debian:bookworm) container proves real `ezkvm start/status/stop/kill/reset` against real QEMU via `/dev/kvm` passthrough, using CI-safe `coruscant/100.conf`-derived synthetic-disk fixture | integration (containerized) | `docker run --device /dev/kvm ... ezkvm start coruscant100` (in `package-verify.yml`) | ❌ W0 | ⬜ pending |
| 10-02 Task 1 | 10-02 | 2 | DEPLOY-02, DEPLOY-03 | V4 | CI matrix expanded to all 4 required targets (Debian 13/12, Ubuntu 26.04/24.04); package install + group/FHS-dir assertions per target | integration (containerized), per target | `docker run ... apt-get install -y /dist/*.deb && getent group ezkvm && test -d /etc/ezkvm && test -d /var/lib/ezkvm` | ❌ W0 | ⬜ pending |
| 10-02 Task 2 | 10-02 | 2 | DEPLOY-01 | N/A | `.deb` size sanity check; Debian revision-suffix convention documented in `Cargo.toml` | build smoke test | asserted within `package-verify.yml` | ❌ W0 | ⬜ pending |
| 10-03 Task 1 | 10-03 | 2 | N/A (documentation) | N/A | `doc/dev/PACKAGING.md` documents all 8 CONTEXT.md decisions (architecture reference) | manual-only | N/A — documentation, no automated check | N/A | N/A |
| 10-03 Task 2 | 10-03 | 2 | N/A (documentation) | N/A | `doc/dev/MANUAL-VERIFICATION.md` explicitly scopes felucia/108's real-hardware GPU/USB-passthrough boot as manual-only, non-CI | manual-only (`checkpoint:human-verify`) | N/A — requires physical hardware, cannot automate | N/A | N/A |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` — add `[package.metadata.deb]` section (no file exists yet for this)
- [ ] `debian/postinst`, `debian/postrm` — group creation/removal, directory setup (new)
- [ ] `debian/ezkvm.conf` (tmpfiles.d drop-in) and `debian/host.yaml.example` (default config) — new
- [ ] `.github/workflows/package-verify.yml` — new, no existing CI to extend
- [ ] CI-only synthetic disk-image fixture generation for `coruscant/100.conf` (script or workflow step producing sparse files / loop devices in place of the original LVM volumes) — new

*(Framework install: none needed — `cargo test` infra already exists from prior phases; the new layer here is entirely packaging/container tooling, not Rust test-framework tooling.)*

---

## Manual-Only Verifications

- **felucia/108's real-hardware GPU/USB-passthrough boot** — requires physical PCI GPU and physical USB devices; cannot be automated in any container/CI environment this project has access to. Documented explicitly in `doc/dev/MANUAL-VERIFICATION.md` (Plan 10-03) as a non-blocking, human-verify checkpoint — does NOT gate phase completion.

---

## Validation Sign-Off

- [x] All automatable tasks have `<automated>` verify commands — confirmed across 10-01/10-02 (10-03 is documentation-only, correctly has no automated verify, per its manual-only nature)
- [x] Sampling continuity: no more than 2 consecutive tasks without automated verify (10-03's 2 documentation tasks are the only non-automated ones, and they don't gate any other task)
- [x] Wave 0 covers all MISSING references — `[package.metadata.deb]`, `debian/postinst`/`postrm`, tmpfiles.d config, new CI workflow, synthetic disk fixture (Plan 10-01 Task 1/2)
- [x] No watch-mode flags
- [x] Feedback latency acceptable given phase nature (container matrix takes minutes, not the sub-15s bar used for pure-Rust phases — explicitly noted above)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved (planning-time Nyquist compliance confirmed; per-task Status/File Exists in the verification map above remain ⬜ pending until Plan execution)
