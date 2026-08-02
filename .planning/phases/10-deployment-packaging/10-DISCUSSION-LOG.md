# Phase 10: Deployment Packaging - Discussion Log

**Date:** 2026-07-29

## Round 1: Gray area selection

Presented 7 gray areas (packaging tooling, systemd integration, privilege model, filesystem
layout, test environment, AppArmor/SELinux, versioning). User chose to discuss all of them,
one at a time.

## Round 2: Packaging tooling & structure

Q: cargo-deb vs hand-rolled `debian/` directory?
A: "Not sure — investigate during research and propose." → **D-01: left open for research.**

## Round 3: Systemd integration model

Q: Templated per-VM systemd service vs plain CLI-only?
A: "Plain CLI only, no systemd integration." → **D-02: no systemd integration.**

## Round 4: Privilege/permission model

Q: Dedicated non-root user + kvm/libvirt group vs root?
A: "Dedicated non-root user + kvm/libvirt group membership."

Follow-up Q: Given D-02 (no systemd, manual CLI), should the package still create a dedicated
service account, or just document required group membership?
A: "Like [document group membership], but do not require dependency on libvirt. If it is more
convenient, then introduce a specific ezkvm group for this purpose, which can be created as part
of installing the debian package." → **D-03: dedicated `ezkvm` group created via postinst, no
libvirt dependency, no dedicated user account — whoever runs ezkvm uses their own login user,
added to the ezkvm group.**

## Round 5: Filesystem layout

Q: Standard FHS locations vs single /opt/ezkvm tree?
A: "Standard FHS locations (/etc/ezkvm, /var/lib/ezkvm, /run/ezkvm)." → **D-04: locked.**

## Round 6: Test environment for the two target distros

Q: Containers vs real/nested VMs for exercising real QEMU against Debian 13/Ubuntu 26.04?
A: "Not sure — investigate during research and propose." → **D-05: left open for research.**

## Round 7: AppArmor/SELinux profile

Q: Ship a profile now, or defer?
A: "Ship no profile for v1, explicitly document 'unconfined'." → **D-06: deferred.**

## Round 8: Versioning/changelog convention

Q: Simple Cargo.toml-tracked version vs full Debian changelog/dch convention?
A: "Simple: package version tracks Cargo.toml version, no separate changelog process for now."
→ **D-07: locked.**

## Round 9: Cross-distro/cross-version compatibility (raised by user after initial CONTEXT.md draft)

User clarified: current focus is Debian 13/Ubuntu 26.04, but separate packages per distro/version
are "acceptable, but not preferred" — Debian 12/Ubuntu 24.04 support is also wanted. Asked the
agent to propose the standard approach.

Recommendation given: static musl-linked binary in a single `.deb`, eliminating glibc
symbol-versioning sensitivity, installing/running identically across all four targets.

A: User selected the recommended static-musl single-package approach.
→ **D-08: single `.deb`, musl-static binary, verified against Debian 12/13 + Ubuntu 24.04/26.04.**

## Outcome

8 decisions locked/scoped (D-01 through D-08, 2 explicitly left open for research: D-01 packaging
tool choice, D-05 test environment choice). Written to `10-CONTEXT.md`. No scope creep — systemd
integration and security-profile authoring explicitly captured as Deferred Ideas rather than
pulled into this phase.
