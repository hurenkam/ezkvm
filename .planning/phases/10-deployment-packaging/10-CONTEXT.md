# Phase 10: Deployment Packaging - Debian/Ubuntu - Context

**Gathered:** 2026-07-29
**Status:** Ready for research/planning

<domain>

## Phase Boundary

This phase builds an installable Debian package (`.deb`) for ezkvm and verifies real VM
start/stop/reset lifecycle behavior against actual target-OS environments — Debian 13 and
Ubuntu 26.04 — using real `qemu-system-x86_64`/`swtpm`/display-client binaries. This is the
deliberate exception to Phase 8's "tests must not depend on host-installed tooling" rule; Phase 8's
tests remain host-agnostic/stubbed, this phase is where real-binary, package/OS-dependent
verification belongs.

</domain>

<decisions>

## Implementation Decisions

### Packaging tooling & structure
- **D-01:** Left open for research — the agent should investigate `cargo-deb` (Rust-native,
  generates a `.deb` from `Cargo.toml` metadata + a `[package.metadata.deb]` section, minimal
  maintenance) vs a hand-rolled `debian/` directory (`dpkg-buildpackage`, full manual control over
  `control`/`rules`/`postinst`/`postrm` scripts) and propose the better fit during
  `/gsd-plan-phase 10`'s research step, given this phase's other locked decisions (D-02 dedicated
  group creation via `postinst`, D-04 FHS layout, D-05 test environment). — **Reversibility:**
  reversible — packaging tool choice doesn't affect runtime behavior or file layout, only the build
  mechanism producing the `.deb`.

### Systemd integration model
- **D-02:** No systemd integration in this phase. ezkvm remains plain-CLI: the user runs
  `ezkvm start`/`stop`/`reset` manually (or wires up their own tooling/automation) — the package
  does NOT install or manage any systemd unit (templated per-VM service or otherwise). This
  reinterprets ROADMAP's success criteria #2/#3 ("`ezkvm start`/`stop`/`reset` work against a real
  QEMU VM") as CLI-invocation verification, not systemd-service verification. — **Reversibility:**
  reversible — a future phase can add systemd unit templates on top of the CLI without breaking
  this phase's package structure or file layout.

### Privilege / permission model
- **D-03:** ezkvm does NOT run as root. The `.deb` package's `postinst` script creates a dedicated
  `ezkvm` **group** (not a dedicated system user/service account — whoever invokes the CLI uses
  their own login user, added to the `ezkvm` group for KVM/device-node access). No dependency on
  `libvirt` or the `libvirt` group is introduced — `/dev/kvm` access is granted directly via the new
  `ezkvm` group (e.g. udev rule or device-node group ownership), not by piggybacking on Debian's
  existing `kvm`/`libvirt` group conventions. Document in package README/postinst output that users
  must be added to the `ezkvm` group to run VMs. — **Reversibility:** reversible for the group
  creation/removal itself (handled in `postinst`/`postrm`), but any external device/udev rules
  referencing the group name should be named carefully since renaming later requires a package
  update.

### Filesystem layout (FHS)
- **D-04:** Standard FHS locations: `/etc/ezkvm/` for configuration (ezkvm YAML files, per-VM
  configs), `/var/lib/ezkvm/` for state and any disk-adjacent metadata, `/run/ezkvm/` for runtime
  sockets (e.g. QEMU monitor sockets, swtpm sockets) — not a single `/opt/ezkvm` tree. — **Reversibility:**
  one-way-ish — changing this later means a breaking migration for any existing installs, though
  this is the very first packaged release so the cost is currently zero; lock it now to avoid churn.

### Test environment for the two target distros
- **D-05:** Left open for research — the agent should investigate containers (Docker/systemd-nspawn)
  vs real/nested VMs for exercising real QEMU start/stop/reset against Debian 13 and Ubuntu 26.04
  reproducibly (e.g. in CI), including whether KVM nesting/passthrough works acceptably inside
  containers on the available CI runners, and propose the better fit during
  `/gsd-plan-phase 10`'s research step. — **Reversibility:** reversible — this is a testing-infra
  choice, doesn't affect the shipped package's behavior.

### Cross-distro/cross-version compatibility
- **D-08:** A single `.deb` package (not separate builds per distro/version) targets ALL FOUR of
  Debian 13 (trixie), Debian 12 (bookworm), Ubuntu 26.04, and Ubuntu 24.04 — Debian 13/Ubuntu 26.04
  are the current focus, but Debian 12/Ubuntu 24.04 support is also required, not just nice-to-have.
  To achieve this with one package, the ezkvm binary is statically linked against **musl** (e.g.
  `x86_64-unknown-linux-musl` target) rather than glibc, eliminating glibc symbol-versioning
  sensitivity entirely — the same `.deb` installs and runs identically across all four target
  OS/version combinations. Phase 10's test matrix (see D-05) must verify successful install +
  `ezkvm start`/`stop`/`reset` against real QEMU on all four targets, not just the two "current
  focus" ones. — **Reversibility:** reversible — switching the build target (musl vs glibc) later
  is a build-pipeline change, not a shipped-file-layout or API change; existing installs aren't
  broken by re-choosing this later, though it would require a new package version.

### AppArmor/SELinux profile
- **D-06:** No AppArmor/SELinux profile is shipped in this phase (v1). The package is documented as
  running "unconfined" for now — profile authoring is explicitly deferred to a future phase.
  — **Reversibility:** reversible — a future phase can add a profile without changing this phase's
  package structure.

### Versioning / changelog convention
- **D-07:** The package version tracks `Cargo.toml`'s `version` field directly (currently `0.1.0`).
  No separate Debian `debian/changelog`-with-`dch` versioning process is introduced in this phase —
  keep it simple for v1. — **Reversibility:** reversible — a full changelog convention can be added
  later without breaking existing installs (Debian versioning is additive/monotonic by convention).

### the agent's Discretion
- Exact `postinst`/`postrm` script contents (group creation/removal, directory creation with
  correct ownership/permissions for `/etc/ezkvm`, `/var/lib/ezkvm`, `/run/ezkvm`) — follow standard
  Debian packaging conventions for this.
- Whether `/run/ezkvm` is created via `postinst` directly or via a `systemd-tmpfiles.d` drop-in
  (acceptable either way since D-02 excludes systemd *service* integration, not incidental use of
  systemd-tmpfiles for directory-creation-on-boot, which is a common, systemd-service-independent
  Debian convention).
- Exact CI mechanism/tooling for D-05's chosen test environment once research recommends one.

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

- `.planning/PROJECT.md` — "VM lifecycle management" requirement (start VM (qemu + swtpm), launch UI
  client, graceful shutdown, force-stop, reset via QEMU monitor) that this phase verifies for real
- `.planning/phases/08-vm-lifecycle/08-CONTEXT.md` and `08-VALIDATION.md` — establishes the
  host-agnostic/stubbed testing discipline that Phase 8 uses and that this phase is the deliberate
  exception to (real binaries here, not stubs)
- `.planning/ROADMAP.md` §"Phase 10: Deployment Packaging - Debian/Ubuntu" — goal, success criteria,
  notes on deliberate deferral

</canonical_refs>

<deferred_ideas>

## Deferred Ideas (Out of Scope for This Phase)

- Systemd unit templates for VM lifecycle management (D-02) — a future phase, if desired
- AppArmor/SELinux profile authoring (D-06) — a future phase
- Full Debian changelog/`dch`-based versioning convention (D-07) — a future phase, if the project's
  release cadence later warrants it

</deferred_ideas>

