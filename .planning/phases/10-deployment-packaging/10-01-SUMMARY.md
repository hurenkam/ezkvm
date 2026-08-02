---
phase: 10-deployment-packaging
plan: 01
subsystem: infra
tags: [debian, cargo-deb, musl, github-actions, qemu, packaging]
requires:
  - phase: 09-round-trip-verification
    provides: round-trip-verified lifecycle/config generation used as packaging baseline
provides:
  - Debian packaging metadata and maintainer scripts for ezkvm
  - Default packaged host config and tmpfiles.d runtime directory setup
  - Single-target package verification workflow and CI-safe VM fixtures
affects: [10-02, 10-03, deployment, packaging, ci]
tech-stack:
  added: [cargo-deb, GitHub Actions]
  patterns: [Cargo.toml-driven deb packaging, tmpfiles.d runtime directory provisioning, CI fixture-based lifecycle verification]
key-files:
  created:
    - debian/postinst
    - debian/postrm
    - debian/ezkvm.conf
    - debian/host.yaml.example
    - .github/workflows/package-verify.yml
    - .github/ci-fixtures/host.yaml
    - .github/ci-fixtures/ci-boot.yaml
    - .planning/phases/10-deployment-packaging/10-01-SUMMARY.md
  modified:
    - Cargo.toml
key-decisions:
  - "Used cargo-deb metadata in Cargo.toml with qemu-system-x86 and swtpm as Depends, and virt-viewer plus looking-glass-client as Recommends."
  - "Provisioned /run/ezkvm through a tmpfiles.d asset instead of any systemd unit, preserving the no-service constraint."
  - "Used a hand-authored, headless CI VM fixture for real-QEMU lifecycle verification to avoid unrelated passthrough/EFI/TPM constraints."
patterns-established:
  - "Pattern 1: express Debian packaging metadata declaratively in Cargo.toml via [package.metadata.deb]."
  - "Pattern 2: create ezkvm group and FHS directories in idempotent postinst/postrm maintainer scripts."
  - "Pattern 3: author CI package verification around mounted fixtures plus /dev/kvm passthrough in Docker."
requirements-completed: [DEPLOY-01, DEPLOY-02, DEPLOY-03, DEPLOY-04, QEMU-04]
coverage:
  - id: D1
    description: "cargo-deb packaging metadata, release profile tuning, and Debian maintainer scripts for ezkvm"
    requirement: "DEPLOY-01"
    verification:
      - kind: integration
        ref: "cargo deb --target x86_64-unknown-linux-musl"
        status: pass
      - kind: integration
        ref: "dpkg -I target/debian/ezkvm_*.deb | grep -E 'Depends: qemu-system-x86, swtpm'"
        status: pass
      - kind: integration
        ref: "dpkg -I target/debian/ezkvm_*.deb | grep -E 'Recommends: virt-viewer, looking-glass-client'"
        status: pass
      - kind: integration
        ref: "test \"$(find debian -iname '*.service' -o -iname '*.socket' -o -iname '*.timer' | wc -l)\" -eq 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "single-target GitHub Actions workflow and CI fixtures for real-QEMU package verification on debian:bookworm"
    requirement: "DEPLOY-04"
    verification:
      - kind: other
        ref: "python3 -c \"import yaml; [yaml.safe_load(open(f)) for f in ('.github/workflows/package-verify.yml', '.github/ci-fixtures/host.yaml', '.github/ci-fixtures/ci-boot.yaml')]\""
        status: pass
    human_judgment: true
    rationale: "Workflow execution requires GitHub Actions with Docker and /dev/kvm passthrough; local authoring sandbox cannot perform the real containerized lifecycle run."
duration: 6 min
completed: 2026-07-30
status: complete
---

# Phase 10: Deployment Packaging Summary

**Debian packaging metadata, maintainer scripts, and a debian:bookworm package-verification workflow now exist for ezkvm's first end-to-end deployment slice.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-30T13:04:54+02:00
- **Completed:** 2026-07-30T13:10:44+02:00
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Added `Cargo.toml` deb packaging metadata and release-profile tuning for `cargo-deb`.
- Created `debian/` maintainer scripts, tmpfiles.d config, and packaged default `host.yaml`.
- Authored `.github/workflows/package-verify.yml` plus CI-safe `host.yaml`/`ci-boot.yaml` fixtures.

## Task Commits

None — per execution instructions, no git commits were created.

## Files Created/Modified
- `Cargo.toml` - Added `[package.metadata.deb]` and `[profile.release]`.
- `debian/postinst` - Creates `ezkvm` group and `/var/lib/ezkvm`, tightens `/etc/ezkvm` permissions.
- `debian/postrm` - Purges `ezkvm` group and `/var/lib/ezkvm` on package purge.
- `debian/ezkvm.conf` - tmpfiles.d rule for `/run/ezkvm`.
- `debian/host.yaml.example` - Default packaged host configuration and operator notes.
- `.github/ci-fixtures/host.yaml` - CI host config with synthetic storage and dummy bridge resources.
- `.github/ci-fixtures/ci-boot.yaml` - Minimal headless VM fixture for lifecycle verification.
- `.github/workflows/package-verify.yml` - Build + verify workflow for `debian:bookworm`.

## Decisions Made
- Followed the plan's package split exactly: `qemu-system-x86`/`swtpm` as hard dependencies, display clients as recommends.
- Kept runtime directory creation in tmpfiles.d instead of adding any service/unit integration.
- Modeled the CI VM as a hand-authored q35 + virtio-scsi + virtio-net fixture to keep real-QEMU verification CI-safe.

## Deviations from Plan

None in delivered scope.

## Issues Encountered
- `cargo deb --target x86_64-unknown-linux-musl` initially failed before the musl Rust target finished installing; rerunning after `rustup target add` completed succeeded.
- `musl-tools` is not installed locally, but the musl package build still succeeded in this sandbox without needing the skipped `sudo apt-get install` fallback.
- `cargo-deb` emitted the `.deb` to `target/debian/` while packaging the musl target binary; verification used the actual output path produced by the tool.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Ready for Plan 10-02 matrix expansion and actual CI execution across the remaining distro targets.
- Local verification completed for package metadata, YAML syntax, and full `cargo build`/`cargo test` regression safety.
- Not locally verified: GitHub Actions runtime behavior, Docker container execution, `/dev/kvm` passthrough, package install-time `postinst`/`postrm` effects on a real Debian/Ubuntu system, or the workflow's real `start`/`status`/`stop`/`reset`/`kill` lifecycle sequence. Those require CI or a privileged target host and were intentionally left as environment-limited checks.

---
*Phase: 10-deployment-packaging*
*Completed: 2026-07-30*
