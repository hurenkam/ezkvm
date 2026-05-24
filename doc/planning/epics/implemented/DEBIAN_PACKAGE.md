# Debian Package Plan (Debian Trixie + Ubuntu Resolute)

## Goal

Build and maintain one `ezkvm` Debian package per CPU architecture (for example `amd64`, `arm64`) that installs and runs on:

- Debian Trixie
- Ubuntu 26.04 (Resolute)

This plan prioritizes a single shared package contract, explicit dependency policy, and repeatable validation on both distributions.

## Non-Goals

- No distro-specific package forks by default.
- No split package model unless a shared baseline is proven impossible.
- No hidden runtime assumptions tied to only one distro image.

## Packaging Contract

1. One binary package per architecture.
2. Keep `Depends` limited to cross-distro baseline runtime requirements.
3. Put optional integrations into `Recommends` whenever possible.
4. Use alternative dependency expressions when package naming differs.
5. Keep filesystem layout stable:
  - `/usr/bin` for executables
  - `/etc/ezkvm/` for config and defaults
  - `/run/ezkvm` (or `/var/run/ezkvm`) for runtime sockets/state
  - `/var/lib/ezkvm` and `/var/log/ezkvm` only when behavior requires them
  - `/usr/share/ezkvm/ezkvm-q35.cfg` for portable q35 PCIe topology definitions (required for portable mode)
6. Preserve conffile-safe behavior for admin-edited config.

## Phase 0: Baseline Inventory

## Objective
Collect current runtime and packaging requirements before introducing `debian/` metadata.

## Tasks
- Inventory required binaries and helpers used by current runtime profiles.
- Identify mandatory runtime dependencies vs optional features.
- Confirm default config/runtime path contract from existing docs and code.
- Define initial architecture targets (`amd64` first, `arm64` if CI/hosts available).

## Deliverables
- Dependency inventory table (required/optional with rationale).
- Path contract checklist for packaged install.

## Exit Criteria
- Mandatory runtime set agreed.
- Optional feature set clearly separated.

## Dependency Inventory

Source of truth: `etc/ezkvm.yaml` and current profile corpus under `etc/profiles.d/`.

### Architecture Targets

| Architecture | Status |
|---|---|
| `amd64` | primary target |
| `arm64` | secondary; include when CI hosts available |

### Runtime: Required (Depends)

These packages are needed for any useful ezkvm invocation.

| Package | Provides | Rationale |
|---|---|---|
| `qemu-system-x86` | `/usr/bin/qemu-system-x86_64` | Core VM execution binary. All start operations require it. |
| `qemu-utils` | `qemu-img` | Disk image management. Required for image creation and inspection. |

Note: For arm64 targets, replace `qemu-system-x86` with `qemu-system-arm` as appropriate.

### Runtime: Optional (Recommends)

These packages enable specific features but are not needed for a minimal VM start.

| Package | Provides | Needed For | Notes |
|---|---|---|---|
| `ovmf` | OVMF firmware files under `/usr/share/OVMF/` | UEFI VMs and secure-boot guests | Required by most non-legacy workloads; recommend rather than hard-depend since BIOS-only VMs do not need it |
| `swtpm` | `/usr/bin/swtpm` | TPM-backed VMs (`tpmstate0` in Proxmox, `tpm` device in schema) | Optional; only needed when VM config includes a TPM device |
| `swtpm-tools` | `swtpm_setup`, `swtpm_cert` | TPM state and certificate initialization | Optional companion to `swtpm` |
| `virt-viewer` | `/usr/bin/remote-viewer` | SPICE/VNC display client (`remote_viewer` integration in `ezkvm.yaml`) | Client-side only; not required for VM runtime itself |
| `looking-glass-client` | `/usr/bin/looking-glass-client` | Looking Glass low-latency display sharing | Requires `/dev/kvmfr0` shared memory device; highly optional |
| `bridge-utils` | `brctl` | Bridged networking setup | Network bridge creation is admin responsibility, not ezkvm's |
| `iproute2` | `ip` | TAP interface lifecycle for tap-based networking | Most systems already have this installed |

### Alternative Dependency Expressions

If package naming diverges between Debian Trixie and Ubuntu 26.04, use alternatives syntax in `debian/control`. Known current state:

| Feature | Debian Trixie | Ubuntu 26.04 | Resolution |
|---|---|---|---|
| QEMU core | `qemu-system-x86` | `qemu-system-x86` | identical — no alternatives needed |
| UEFI firmware | `ovmf` | `ovmf` | identical — no alternatives needed |
| Software TPM | `swtpm` | `swtpm` | identical — no alternatives needed |

No alternatives expressions required at this time. Revalidate at K-03.

### Build Dependencies

| Package | Rationale |
|---|---|
| `debhelper-compat (= 13)` | Packaging helper compatibility level |
| `dh-cargo` | Rust/Cargo integration for `dh` |
| `cargo` | Rust build tool |
| `rustc` | Rust compiler |
| `pkg-config` | Native library detection during build |

## Path Contract Checklist

Source of truth: `etc/ezkvm.yaml` (`host_capabilities` and `locations` sections).

| Path | Type | Created By | Notes |
|---|---|---|---|
| `/usr/bin/ezkvm` | binary | package install | Main executable |
| `/etc/ezkvm/ezkvm.yaml` | conffile | package install | Default global config; admin-editable; must survive upgrade |
| `/etc/ezkvm/vms.d/` | directory | package install | VM definition files; admin-managed |
| `/etc/ezkvm/profiles.d/` | directory | package install | Profile definition files; shipped defaults + admin-extendable |
| `/run/ezkvm/` | runtime dir | `tmpfiles.d` or service | Root runtime directory; equivalent to `/var/run/ezkvm/` via symlink on systemd hosts |
| `/run/ezkvm/pids/` | runtime dir | `tmpfiles.d` or service | PID files for running VMs |
| `/run/ezkvm/sockets/` | runtime dir | `tmpfiles.d` or service | Unix domain sockets |
| `/var/log/ezkvm/` | log dir | `tmpfiles.d` or service | Log output; optional if journald is used exclusively |
| `/var/lib/ezkvm/tpm/` | state dir | `tmpfiles.d` or service | TPM state persistence; only required when TPM device is configured |
| `/run/ezkvm/tpm/` | runtime dir | `tmpfiles.d` or service | TPM sockets; only required when TPM device is configured |

**Conffile policy**: files under `/etc/ezkvm/` must be listed in `debian/conffiles` or managed by `dh_installdocs`/`dh_installconffiles` so admin edits are preserved across upgrades.

**Runtime dir policy**: `/run/ezkvm/` and all subdirectories must be created via `debian/ezkvm.tmpfiles` at boot or by the service at start. They must NOT be present in the `.deb` as static directories.

## Phase 1: Debian Packaging Skeleton

## Objective
Create a minimal but policy-compliant `debian/` package layout.

## Tasks
- Add `debian/control` with:
  - `Build-Depends`: `debhelper-compat (= 13)`, `cargo`, `rustc`, `dh-cargo`, and required build tooling
  - `Depends`: baseline runtime set valid on both target distros
  - `Recommends`: optional helper packages
- Add `debian/rules` using `dh` + Rust build flow.
- Add `debian/changelog` initial packaging entry.
- Add `debian/install` mapping binary and default config/profile files.
- Add runtime directory policy (`tmpfiles` and/or service startup behavior).
- Ensure conffile-safe defaults for `/etc/ezkvm/*`.

## Deliverables
- First installable `.deb` artifact for one architecture.

## Exit Criteria
- Package builds successfully with `dpkg-buildpackage -us -uc -b`.
- Artifact contains expected file layout.

## Phase 2: Dependency Policy Hardening

## Objective
Guarantee dependency expressions remain cross-distro compatible.

## Tasks
- Resolve package-name differences with alternatives in `Depends` where needed.
- Move non-essential integrations out of `Depends` into `Recommends`.
- Document why each hard dependency is mandatory for core behavior.

## Deliverables
- Finalized dependency policy section in packaging notes.

## Exit Criteria
- Debian Trixie and Ubuntu Resolute both resolve dependencies without manual patching.

## Phase 3: Dual-Distro Validation Matrix

## Objective
Validate build, install, and runtime behavior on both targets.

## Tasks
- Build checks:
  - `dpkg-buildpackage -us -uc -b`
  - `lintian` on generated artifacts
- Install checks on Debian Trixie and Ubuntu Resolute:
  - package install success
  - dependency resolution success
- Runtime checks on both:
  - binary is runnable
  - required config/runtime dirs exist with expected ownership/modes
  - `ezkvm` dry-run/start readiness checks pass
- Upgrade checks:
  - admin-edited conffiles are preserved

## Validation Matrix Template

| Area | Debian Trixie | Ubuntu Resolute | Notes |
|---|---|---|---|
| Build (`dpkg-buildpackage`) | pending | pending | |
| Lint (`lintian`) | pending | pending | |
| Install (`dpkg -i`) | pending | pending | |
| Dependency resolution | pending | pending | |
| Runtime dirs and paths | pending | pending | |
| Dry-run/start readiness | pending | pending | |
| Conffile preservation on upgrade | pending | pending | |

## Exit Criteria
- Matrix complete with all required checks passing.

## Phase 4: CI and Release Gate

## Objective
Prevent regressions after initial packaging success.

## Tasks
- Add CI jobs for package build and lint.
- Add at least one install smoke test per distro image.
- Publish release gate requiring dual-distro validation evidence.
- Add a packaging review checklist for PRs touching dependencies/paths.

## Deliverables
- CI workflow steps for packaging validation.
- Release checklist used by maintainers.

## Exit Criteria
- Packaging PRs cannot merge without required evidence.

## Escalation Path (Only If Needed)

If one shared package contract becomes infeasible:

1. Document exact blocker (dependency, path, ABI, or service integration).
2. Propose minimal split strategy (`common` + distro shim).
3. Require explicit maintainer approval before implementing split packaging.

## Suggested Execution Order

1. Phase 0 inventory
2. Phase 1 packaging skeleton
3. Phase 2 dependency hardening
4. Phase 3 validation matrix completion
5. Phase 4 CI/release enforcement

## Owner Checklist

- Packaging owner assigned
- Target architectures confirmed
- Dual-distro test hosts or containers available
- Validation matrix updated per run
- Final release gate signed off

## Status

- Draft status: ready for implementation
- Dual-distro validation status: not run
