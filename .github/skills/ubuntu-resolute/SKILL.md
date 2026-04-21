---
name: ubuntu-resolute
description: 'Use when working on Ubuntu 26.04 (Resolute Raccoon) host setup, QEMU/KVM integration, distro-specific paths, or building installable ezkvm .deb packages with correct dependencies, default configs, and runtime directories.'
argument-hint: 'What Ubuntu 26.04 task should this skill perform?'
---

# ubuntu-resolute

Scope: Workspace skill for Ubuntu 26.04 (Resolute Raccoon) operational and packaging workflows for ezkvm.

## What This Skill Produces

- Ubuntu 26.04-oriented implementation guidance for ezkvm host setup and operations.
- QEMU/KVM path and runtime mapping that matches Ubuntu conventions.
- A repeatable Debian packaging workflow for producing installable ezkvm `.deb` artifacts.
- Validation checklists for dependencies, file placement, and runtime readiness.

## Description

This skill captures Ubuntu 26.04 host and packaging expertise for ezkvm, including QEMU/KVM integration, distro-specific filesystem and runtime conventions, and installable Debian package workflows. Use it when work depends on Ubuntu-specific behavior for setup, packaging, or runtime validation.

## Use When

- configuring ezkvm on Ubuntu 26.04 systems
- mapping distro-specific file paths and package names
- debugging QEMU/KVM startup issues on Ubuntu hosts
- documenting Ubuntu install/runtime prerequisites
- building or reviewing installable ezkvm `.deb` packages
- ensuring defaults in `/etc`, `/usr`, and `/var` are packaged and created correctly

## Ubuntu 26.04 Baseline Assumptions

1. Host package management uses `apt` and `dpkg`.
2. Service and tmpfiles integration uses `systemd` (`systemctl`, `tmpfiles.d`, optional `sysusers.d`).
3. KVM support depends on host CPU virtualization and `kvm` kernel modules.
4. qemu package naming follows Debian/Ubuntu style (for example `qemu-system-x86`, `qemu-utils`).
5. Runtime ownership and permissions should prefer dedicated users/groups only if the service model requires it.

## Architecture And Package Type Decisions

1. Confirm target architecture (`amd64`, `arm64`, `i386`, `riscv64`, or other supported target).
2. Decide package type: architecture-specific binary vs architecture-independent package.
3. Validate ABI and feature constraints that may differ across CPU families.
4. Ensure artifact naming and metadata correctly reflect architecture intent.

## Debian Artifact Context

- Binary package flow: `.deb` installable artifact.
- Source package flow: `.dsc` and related source artifacts for distribution workflows.
- Repository publication needs package metadata consistency and archive layout compatibility.
- Choose source-only vs binary outputs based on release and CI strategy.

## General Configuration Workflow

1. Confirm host and kernel capability
   - Check virtualization support (`vmx` or `svm`) and loaded modules (`kvm`, `kvm_intel`, `kvm_amd`).
   - Verify `/dev/kvm` exists and user/group access model is defined.

2. Install base dependencies
   - Runtime: `qemu-system-x86`, `qemu-utils`, `ovmf`, `bridge-utils`, `iproute2`, `dnsmasq-base` (if used by networking mode), and any graphics/input dependencies your profile requires.
   - Packaging/build: `build-essential`, `cargo`, `rustc`, `debhelper`, `dh-cargo`, `dpkg-dev`, `devscripts`, `fakeroot`, `lintian`.

3. Map ezkvm file hierarchy on Ubuntu
   - Main config: `/etc/ezkvm/ezkvm.yaml`
   - VM definitions: `/etc/ezkvm/vms.d/`
   - Profiles: `/etc/ezkvm/profiles.d/`
   - Runtime state and sockets: `/var/run/ezkvm/` (or `/run/ezkvm/`)
   - Persistent runtime data/logs: `/var/lib/ezkvm/` and/or `/var/log/ezkvm/` when applicable

4. Validate config behavior
   - Ensure package installs default config files without overwriting admin-modified files (`conffiles` behavior).
   - Ensure runtime directories are created at boot/start with correct owner/mode.

## QEMU On Ubuntu 26.04

### Core Binaries and Utilities

- `qemu-system-x86_64`: primary VM launcher for x86 guests.
- `qemu-img`: disk image management.
- `qemu-nbd`: optional block export and tooling.
- `virtiofsd`: optional shared filesystem support, package name may vary by release.

### Common Ubuntu Paths

- Firmware images:
  - `/usr/share/OVMF/OVMF_CODE.fd`
  - `/usr/share/OVMF/OVMF_VARS.fd`
- QEMU helper and share data:
  - `/usr/share/qemu/`
- QEMU bridge helper config:
  - `/etc/qemu/bridge.conf`
- Global/local libvirt context (if libvirt is involved):
  - `/etc/libvirt/`
  - `/var/lib/libvirt/`
- KVM device:
  - `/dev/kvm`

### Runtime and Process Inspection

1. Process details: `ps`, `systemctl status`, and ezkvm logs.
2. Open files/sockets: `lsof -p <pid>` or `ss -xl` for unix sockets.
3. Effective QEMU arguments: inspect ezkvm dry-run output and compare with running process command line.
4. Permissions: verify access to `/dev/kvm`, tap devices, bridge interfaces, and firmware files.

### Network Integration Notes

- For tap/bridge networking, ensure bridge devices exist and are allowed by policy.
- If bridge helper is used, whitelist bridges in `/etc/qemu/bridge.conf`.
- Validate nftables/iptables policy if guest connectivity fails despite successful boot.

## Building Installable ezkvm Packages For Ubuntu

## Packaging Goal

Produce a `.deb` that installs:
- executable binaries under `/usr/bin`
- default configuration under `/etc/ezkvm/`
- runtime directory policy for `/run/ezkvm` (or `/var/run/ezkvm`)
- optional persistent state directories under `/var/lib/ezkvm`
- optional log directory under `/var/log/ezkvm`

## Recommended Debian Packaging Layout

Inside repository packaging metadata (`debian/`):

- `debian/control`: Build-Depends and runtime Depends.
- `debian/rules`: Build logic (typically `dh` + Rust integration via `dh-cargo`).
- `debian/changelog`: versioned release entries.
- `debian/install`: explicit file placement rules.
- `debian/conffiles` (if required): ensure editable defaults remain admin-safe.
- `debian/ezkvm.tmpfiles`: runtime directory creation policy.
- `debian/ezkvm.service` (optional): systemd unit when shipping daemon/service mode.

## Dependency Model (Typical)

1. Build dependencies
   - `debhelper-compat (= 13)`
   - `dh-cargo`
   - `cargo`, `rustc`
   - `pkg-config`
   - any native libraries required by enabled features

2. Runtime dependencies
   - `qemu-system-x86`
   - `qemu-utils`
   - `ovmf`
   - networking helper packages when required by runtime mode

3. Recommendation
   - Keep runtime `Depends` minimal and explicit.
   - Put optional helpers in `Recommends` when functionality is non-core.

## Build Procedure

1. Clean build prerequisites
   - Install build tools and dependencies.
   - Ensure working tree has packaging metadata and version bump.
   - Prefer clean-room builds (`sbuild`, `pbuilder`, or containerized build root) when validating release readiness.

2. Build package
   - `dpkg-buildpackage -us -uc -b`
   - or `debuild -us -uc -b`

3. Validate artifact
   - `lintian ../ezkvm_*.changes`
   - `dpkg-deb -c ../ezkvm_<version>_<arch>.deb`
   - Verify expected paths and file modes.

4. Install and verify
   - `sudo dpkg -i ../ezkvm_<version>_<arch>.deb`
   - `sudo apt-get -f install` if dependency fix-up is needed
   - Confirm binary availability, config files, and runtime dirs
   - Run ezkvm dry-run/start workflow to confirm QEMU launch readiness

## Debian Policy And Reproducibility Checklist

- File placement follows Debian/FHS expectations.
- `debian/control` expresses `Build-Depends` and runtime `Depends` clearly.
- Maintainer scripts and service integration avoid hidden side effects.
- Optional functionality is modeled in `Recommends` when appropriate.
- Build outcomes are repeatable in clean environments.

## Runtime Directories And Defaults Checklist

- `/etc/ezkvm/ezkvm.yaml` exists after install.
- `/etc/ezkvm/vms.d/` and `/etc/ezkvm/profiles.d/` exist with sensible default permissions.
- `/run/ezkvm` (or `/var/run/ezkvm`) is created automatically through systemd tmpfiles or service startup.
- If state/log folders are required, `/var/lib/ezkvm` and `/var/log/ezkvm` are created and writable by the service user.
- Package upgrades preserve admin-edited config files.

## Decision Points

- Native package vs CI-built artifact: choose based on release process and reproducibility needs.
- Single-binary package vs split packages: choose based on service, docs, and optional components.
- Runtime directory strategy: tmpfiles-managed, service-managed, or both.
- Dependency strictness: hard `Depends` vs `Recommends` for optional integration features.

## Distro Delta Notes

- Ubuntu 26.04 and Debian Trixie packaging workflows are closely related, but package availability and naming may diverge.
- Validate distro-specific helper package names before locking dependency lists.
- Reconfirm runtime path assumptions for QEMU helpers and integration files per distro image.

## Quality Criteria

- Package builds cleanly on Ubuntu 26.04 without manual patching.
- Installed files match expected FHS locations.
- Default config is present and editable as conffile-safe.
- Runtime directories are automatically available with correct ownership/mode.
- QEMU launch path works with installed dependencies and firmware.
- Upgrade/install/remove operations do not leave broken ownership or missing critical directories.

## Completion Gate

- package builds in a clean environment
- package lint/metadata checks pass
- package installs with expected dependency resolution
- ezkvm runtime launches with installed defaults
- docs/examples match Ubuntu 26.04 behavior and assumptions

## Example Prompts

- `/ubuntu-resolute create a Ubuntu 26.04 packaging checklist for ezkvm with exact Depends and Build-Depends.`
- `/ubuntu-resolute map my current ezkvm runtime paths to Ubuntu FHS-compliant package install paths.`
- `/ubuntu-resolute troubleshoot why QEMU boots manually but fails through ezkvm service on Ubuntu 26.04.`
- `/ubuntu-resolute review this debian/control and debian/install for missing runtime defaults.`

## Follow-up Customizations To Consider

- Add a packaging prompt template: `.github/prompts/ubuntu-resolute-package.prompt.md`.
- Add an Ubuntu instruction file with path defaults: `.github/instructions/ubuntu-resolute.instructions.md`.
- Add scripted packaging checks under `.github/skills/ubuntu-resolute/scripts/` for repeatable validation.
