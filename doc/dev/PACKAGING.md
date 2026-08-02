# Packaging

## Scope

This document records the packaging decisions for Phase 10's Debian/Ubuntu `.deb` work. It complements, rather than duplicates, the inline operator notes in `debian/host.yaml.example`, which remains the canonical place for the shipped sample config comments.

## Packaging tool and release model

- The package is built with `cargo-deb` (D-01).
- v1 ships as a plain CLI package with no systemd unit or service integration (D-02).
- The package version tracks `Cargo.toml` directly (D-07); the Debian revision starts at cargo-deb's default `-1`.

## Single musl-static package across all four targets

Phase 10 targets one `.deb` for:

- Debian 12 (bookworm)
- Debian 13 (trixie)
- Ubuntu 24.04
- Ubuntu 26.04

To make one package viable across all four targets, ezkvm is built for `x86_64-unknown-linux-musl` and packaged as a statically linked binary (D-08). The point is not performance; the point is removing glibc symbol-versioning drift between distro releases so the same `.deb` installs and runs consistently everywhere in the target matrix.

## Filesystem layout and access control

Phase 10 follows standard FHS locations, not `/opt/ezkvm` (D-04):

| Path | Purpose |
|---|---|
| `/etc/ezkvm/` | host config plus per-VM YAML definitions |
| `/var/lib/ezkvm/` | persistent ezkvm-managed state/metadata |
| `/run/ezkvm/` | runtime sockets and ephemeral process state |

`/run/ezkvm` is created via `usr/lib/tmpfiles.d/ezkvm.conf`, while `postinst` prepares the persistent directories and group ownership expected by the CLI install.

### `ezkvm` group model

ezkvm does **not** run as root and does **not** depend on libvirt or membership in a `libvirt` group (D-03). Instead:

- `postinst` creates a dedicated `ezkvm` group.
- login users who should manage VMs must be added to that group.
- `/etc/ezkvm` and `/var/lib/ezkvm` are owned to allow controlled shared access for that group.
- the group grant is security-relevant: membership should be treated as permission to manage local virtualization resources such as `/dev/kvm` and any device-node/udev access wired to the `ezkvm` group.

For the operator-facing wording shipped with the package, see the header comments in `debian/host.yaml.example`.

## Runtime package dependencies

### Hard `Depends`

These are required for the packaged CLI to do real work:

- `qemu-system-x86`
- `swtpm`

Use `qemu-system-x86` in Debian metadata, **not** `qemu-system-x86_64`. The executable is `/usr/bin/qemu-system-x86_64`, but the Debian/Ubuntu package name is `qemu-system-x86`; using the binary name as the package name breaks dependency resolution.

### `Recommends`

These are optional display-client packages:

- `virt-viewer`
- `looking-glass-client`

`looking-glass-client` is `Recommends`-only, not a hard dependency, because it is absent from three of the four target archives identified in the Phase 10 research: Debian 12, Debian 13, and Ubuntu 26.04. Users who want Looking Glass on those targets should install it from upstream release tarballs published at <https://looking-glass.io/>.

## Security profile stance in v1

No AppArmor or SELinux profile ships in v1 (D-06). The package currently runs **unconfined**. That is an explicit temporary choice, not an accidental omission: confinement policy authoring is deferred to a future phase.

## Debian revision-suffix convention

Package versioning follows `Cargo.toml` plus cargo-deb's normal Debian revision handling (D-07):

- normal releases ship as `<cargo-version>-1`
- packaging-only re-releases without a Rust version bump should increment the Debian revision to `-2`, `-3`, and so on

Use `debian/host.yaml.example` as the authoritative shipped reminder for this convention so the sample config and docs do not drift.

## Test environment (CI verification)

The package + real-QEMU lifecycle (`start`/`status`/`stop`/`kill`/`reset`) is verified in
`.github/workflows/package-verify.yml` using Docker containers with `--device /dev/kvm`
passthrough, one container per target distro/version (D-05). GitHub-hosted runners expose
`/dev/kvm` but require a CI-only `chmod 666 /dev/kvm` permission relaxation — this is documented
in the workflow itself and must never be recommended to real end users, who should rely on
`ezkvm`-group membership instead (see above).

## Distro-specific quirks

| Target(s) | Quirk | Status |
|---|---|---|
| All four targets | None encountered as of initial packaging (2026-07-30); update this table if Plan 10-02's 4-target matrix surfaces a real per-target difference. | Placeholder |
