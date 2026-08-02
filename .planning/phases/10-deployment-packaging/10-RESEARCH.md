# Phase 10: Deployment Packaging - Debian/Ubuntu - Research

**Researched:** 2026-07-30
**Domain:** Debian packaging of a Rust CLI (musl static binary) + real-QEMU-boot CI verification across 4 distro/version targets
**Confidence:** HIGH (D-01/D-05 tooling claims verified directly against cargo-deb's own README/systemd.md and live Debian/Ubuntu package archives via `curl`; a handful of CI-environment specifics remain MEDIUM/ASSUMED — see Assumptions Log)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-02:** No systemd integration in this phase. ezkvm remains plain-CLI: the user runs
  `ezkvm start`/`stop`/`reset` manually — the package does NOT install or manage any systemd unit
  (templated per-VM service or otherwise).
- **D-03:** ezkvm does NOT run as root. The `.deb`'s `postinst` creates a dedicated `ezkvm`
  **group** (not a service user/account) — whoever invokes the CLI uses their own login user,
  added to the `ezkvm` group for `/dev/kvm` access. No dependency on `libvirt` or its group.
- **D-04:** Standard FHS locations: `/etc/ezkvm/` (config), `/var/lib/ezkvm/` (state),
  `/run/ezkvm/` (runtime sockets) — not a single `/opt/ezkvm` tree.
- **D-06:** No AppArmor/SELinux profile shipped in v1 — documented as "unconfined".
- **D-07:** Package version tracks `Cargo.toml`'s `version` field directly (currently `0.1.0`).
  No `debian/changelog`/`dch` versioning process in this phase.
- **D-08:** A single `.deb` package (not per-distro builds) targets ALL FOUR of Debian 13
  (trixie), Debian 12 (bookworm), Ubuntu 26.04 (resolute), and Ubuntu 24.04 (noble). Achieved via
  a statically musl-linked binary (`x86_64-unknown-linux-musl`) to eliminate glibc
  symbol-versioning sensitivity entirely.

### the agent's Discretion
- Exact `postinst`/`postrm` script contents (group creation/removal, directory creation with
  correct ownership/permissions for `/etc/ezkvm`, `/var/lib/ezkvm`, `/run/ezkvm`) — follow standard
  Debian packaging conventions.
- Whether `/run/ezkvm` is created via `postinst` directly or via a `systemd-tmpfiles.d` drop-in
  (acceptable either way — D-02 excludes systemd *service* integration, not incidental use of
  systemd-tmpfiles for directory-creation-on-boot).
- Exact CI mechanism/tooling for D-05's chosen test environment once research recommends one.

### Deferred Ideas (OUT OF SCOPE)
- Systemd unit templates for VM lifecycle management (D-02) — a future phase, if desired.
- AppArmor/SELinux profile authoring (D-06) — a future phase.
- Full Debian changelog/`dch`-based versioning convention (D-07) — a future phase, if warranted.

### Open decisions this research resolves
- **D-01 (packaging tooling):** Recommendation below — **use `cargo-deb`.**
- **D-05 (test environment):** Recommendation below — **Docker containers with `/dev/kvm`
  passthrough**, one container image per target distro/version, with a documented CI-host
  precondition and a TCG (software emulation) fallback path.
</user_constraints>

<phase_requirements>
## Phase Requirements

Phase 10's `REQUIREMENTS.md` entries are marked `TBD (to be assigned during /gsd-plan-phase)`.
The one *existing* requirement this phase completes is:

| ID | Description | Research Support |
|----|-------------|------------------|
| QEMU-04 | Generated commandline for felucia/108.conf produces a VM that starts in QEMU | felucia/108.conf (`wakiza`) is **unsuitable** for this phase's automated/CI boot test — see Pitfall 5. Research recommends satisfying QEMU-04's literal wording with a real, host-agnostic-hardware-free corpus file (`coruscant/100.conf`) run inside the packaging test environment, and treating felucia/108's PCI/USB-passthrough-dependent boot as a documented manual/human-verify step only, since no CI runner can provide the physical GPU/USB device felucia's VM passes through. |

New requirement IDs the planner should mint under a `DEPLOY-*` prefix (not yet in
`REQUIREMENTS.md` — proposed for the planner to formalize):

| Proposed ID | Description | Research Support |
|-------------|-------------|------------------|
| DEPLOY-01 | A single musl-static `.deb` builds via `cargo-deb --target x86_64-unknown-linux-musl` | Standard Stack, Code Examples below |
| DEPLOY-02 | Package installs cleanly on Debian 13/12 and Ubuntu 26.04/24.04, creating the `ezkvm` group and FHS directories via `postinst` | Package Legitimacy Audit, Architecture Patterns |
| DEPLOY-03 | `qemu-system-x86_64`, `swtpm` declared as hard `Depends`; `virt-viewer`/`looking-glass-client` declared as `Recommends` (not hard `Depends`) | Pitfall 4 — looking-glass-client absent from 3 of 4 targets |
| DEPLOY-04 | Real `ezkvm start`/`stop`/`kill`/`reset` verified against real `qemu-system-x86_64`+`swtpm` inside a container with KVM passthrough, for a corpus VM with no host-specific hardware dependency | Validation Architecture, Pitfall 5 |
</phase_requirements>

## Summary

This phase has two genuinely open questions (D-01, D-05); everything else is locked. Both are
now resolved with direct, verifiable evidence rather than guesswork.

**D-01 — packaging tooling:** `cargo-deb` (crate `cargo-deb`, currently v3.7.0 on crates.io
[VERIFIED: crates.io registry]) is the right tool. Its own README, fetched directly in this
research session, confirms all three things this phase needs: (a) cross-compilation is a
first-class, documented feature via `cargo deb --target=<triple> [--no-build]`, and the README
*explicitly recommends* "a completely static binary for MUSL" as the way to support Debian
releases older than the build host — i.e. cargo-deb's own maintainers point at exactly D-08's
architecture; (b) `maintainer-scripts` is a directory-valued config key pointing at hand-written
`preinst`/`postinst`/`prerm`/`postrm` scripts, which is exactly the mechanism D-03's group-creation
script needs; (c) `assets` entries can map arbitrary source files to arbitrary destination paths
with explicit permission strings, covering D-04's `/etc/ezkvm` and `/var/lib/ezkvm` directories
directly, and a plain (non-systemd-units) asset entry can install a `systemd-tmpfiles.d` drop-in
file for `/run/ezkvm` without touching cargo-deb's separate, heavier `[package.metadata.deb.systemd-units]`
feature (that feature is for actual systemd *services* — using it here would be scope creep
against D-02; a tmpfiles.d config is not a service and needs none of that machinery). A hand-rolled
`debian/` + `dpkg-buildpackage` approach would give equivalent end results but requires
maintaining a `debian/rules`/`debian/control` pair by hand with no `Cargo.toml`-driven single
source of truth for the version (fights D-07) — `cargo-deb` is the clear win here.

**D-05 — test environment:** Docker containers with `--device /dev/kvm` passthrough, one
container per target (`debian:trixie`, `debian:bookworm`, `ubuntu:26.04`/`resolute`,
`ubuntu:24.04`/`noble` base images), is the right approach — confirmed practical via GitHub's own
`actions/runner-images` issue tracker (fetched live in this session): standard GitHub-hosted Linux
x86_64 runners **do** expose `/dev/kvm` (issue #14062 is titled "Please support KVM on **ARM**
runners", implying x86_64 already has it; issue #8542 "runner user is not in the kvm group" and
#8670 confirm `/dev/kvm` exists on the runner but permissions need an explicit `chmod`/group-add
step in the workflow before container passthrough will work — a well-known, scripted fix, not a
blocker). This project has **no existing `.github/workflows/` directory** — CI must be created
from scratch in this phase, there's nothing to extend. Full nested VMs (one per target) are the
correct *fallback* if any CI provider used later lacks host KVM (self-hosted runner, alternate CI
vendor) — document as Plan B, don't build both paths speculatively.

The codebase itself is already `/etc/ezkvm` + `/run/ezkvm`-FHS-aware by default (`main.rs`'s
`--config-dir` defaults to `/etc/ezkvm`; `HostConfig::load`'s `state_dir` defaults to
`/run/ezkvm` when absent from `host.yaml`) — packaging work does not require source changes to
achieve D-04's layout, only shipping a default `/etc/ezkvm/host.yaml` with the correct absolute
tool paths and adding `/var/lib/ezkvm` (currently unused by any code path — reserve it for future
state, ship the empty directory per D-04). No dependency in `Cargo.lock` (46 crates total) links
against C libraries or requires network/DNS/TLS at build time — musl cross-compilation has no
known blockers for this codebase.

**Primary recommendation:** Use `cargo-deb` with `[package.metadata.deb]` in `Cargo.toml`,
building against `x86_64-unknown-linux-musl` (`rustup target add x86_64-unknown-linux-musl` +
`apt install musl-tools`, then `cargo deb --target=x86_64-unknown-linux-musl`). Verify
install + `ezkvm start`/`stop`/`kill`/`reset` against real QEMU inside 4 Docker containers
(`--device /dev/kvm`) driven by a new GitHub Actions workflow, using `coruscant/100.conf` (a
plain Linux VM with no PCI/USB passthrough) as the CI-suitable real-VM fixture — NOT
`felucia/108.conf`, which requires a physical GPU and USB device that cannot exist in CI.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Binary build (musl static link) | Build/CI | — | `cargo build --target x86_64-unknown-linux-musl --release`; produces the single portable artifact D-08 requires |
| `.deb` assembly (control file, assets, scripts) | Build/CI (`cargo-deb`) | — | Packaging metadata lives in `Cargo.toml`'s `[package.metadata.deb]`, not a separate build system |
| Privilege/group setup (`ezkvm` group, `/dev/kvm` access) | OS / Package install-time (`postinst`) | — | D-03 — happens once, at `dpkg -i` time, not at CLI runtime |
| FHS directory creation (`/etc`, `/var/lib`, `/run`) | OS / Package install-time (`postinst` + `assets` + tmpfiles.d) | — | D-04 — static dirs via assets, `/run` (tmpfs, wiped on reboot) via tmpfiles.d |
| VM lifecycle (start/stop/kill/reset) | CLI / Backend (`src/lifecycle/*`) | — | Already fully implemented (Phase 8); unchanged by this phase |
| Real QEMU/swtpm process execution | Host OS (installed `qemu-system-x86_64`/`swtpm` binaries) | — | Declared as package `Depends`, not vendored; this is the real-binary exception Phase 8 deliberately deferred |
| UI client launch (SPICE/VNC/Looking Glass) | Host OS (installed `virt-viewer`/`looking-glass-client`) | — | Declared as `Recommends` (optional; display type is per-VM config, and looking-glass-client isn't available on all 4 targets — see Pitfall 4) |
| Real-boot CI verification | CI (containers w/ KVM passthrough) | Full VM fallback | New GitHub Actions workflow; deliberate exception to Phase 8's host-agnostic-test rule |

## Standard Stack

### Core
| Library/Tool | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `cargo-deb` | 3.7.0 [VERIFIED: crates.io registry, confirmed via direct `curl` to `crates.io/api/v1/crates/cargo-deb` and GitHub `releases/latest` → `v3.7.0`] | Builds the `.deb` directly from `Cargo.toml` metadata + built binary | De facto standard for packaging Rust CLIs as `.deb`; actively maintained (release dated within the last few months per GitHub API), used by hundreds of Rust CLI projects |
| Rust target `x86_64-unknown-linux-musl` | ships with `rustc`/`rustup` | Static-link target eliminating glibc symbol versioning | [VERIFIED: `rustup target list` on this machine lists it as an installable target] — this is the standard "single binary, any glibc version" technique in the Rust ecosystem |
| `musl-tools` (Debian/Ubuntu apt package) | current distro version | Provides `musl-gcc`, needed by `rustup`'s musl target linker on a glibc build host | [ASSUMED — standard, well-documented requirement for musl cross-compiling on a glibc host; not independently verified in this session beyond package existing in apt, since no CI host was available to test] |

### Supporting
| Library/Tool | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `cross` or `cargo-zigbuild` | latest | Alternative musl cross-compilation toolchains if `rustup target add` + `musl-tools` proves fragile in CI | Only if the plain `rustup`+`musl-tools` route hits linker issues in CI — cargo-deb's own README explicitly names both as fallbacks for "onerous" cross-compilation requirements |
| Docker | any recent (20.10+) | Container runtime for D-05's 4-distro test matrix | GitHub-hosted runners ship Docker preinstalled; standard for this kind of matrix testing |
| `qemu-system-x86_64`, `swtpm`, `virt-viewer` | distro-packaged, version varies per target (Debian 12 ships QEMU 7.2, Debian 13/Ubuntu 26.04 ship QEMU 10.x — see Package Legitimacy Audit) | Real runtime dependencies exercised by this phase's lifecycle tests | Already the exact tools `src/lifecycle/*` invokes as subprocesses; nothing new to add to `Cargo.toml` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `cargo-deb` | Hand-rolled `debian/` + `dpkg-buildpackage` | Full manual control, but duplicates version tracking outside `Cargo.toml` (fights D-07) and requires maintaining `debian/rules`'s dh-sequence boilerplate by hand for no benefit this phase needs |
| musl static binary | glibc binary + per-distro build matrix | Would require 4 separate `.deb` builds (violates D-08's explicit "single package" requirement) and reintroduces glibc symbol-versioning fragility across Debian 12→13/Ubuntu 24.04→26.04 |
| Docker + `/dev/kvm` passthrough | Full nested VMs per target (e.g. via `libvirt`/`virt-install` inside the CI runner) | Heavier, slower CI (minutes → potentially 10s of minutes per target), no dependency on CI-host KVM availability — reserve as documented fallback, not primary path |
| Docker + `/dev/kvm` passthrough | `systemd-nspawn` | Comparable isolation to Docker for this use case, but far less GitHub Actions tooling/action ecosystem support; Docker is already what GH-hosted runners are built around |

**Installation:**
```bash
# One-time host setup (build machine or CI runner)
rustup target add x86_64-unknown-linux-musl
sudo apt-get install -y musl-tools
cargo install cargo-deb

# Build
cargo deb --target x86_64-unknown-linux-musl
# Output: target/x86_64-unknown-linux-musl/debian/ezkvm_0.1.0-1_amd64.deb
```

**Version verification performed this session:**
- `cargo-deb`: confirmed `v3.7.0` via `curl -A "..." https://api.github.com/repos/kornelski/cargo-deb/releases/latest` → tag `v3.7.0` [VERIFIED: GitHub API, direct fetch]. crates.io listing also confirmed the crate exists and is actively updated (`updated_at: 2026-05-02`).
- `qemu-system-x86` (binary package name — note: **not** `qemu-system-x86_64`, see Pitfall 1): confirmed present on Debian bookworm (`1:7.2+dfsg-7+deb12u18`), trixie (`1:10.0.11+ds-0+deb13u1`), Ubuntu noble, and Ubuntu resolute via `qa.debian.org/madison.php` and `packages.ubuntu.com` [VERIFIED: Debian/Ubuntu package archive, direct `curl`].
- `swtpm`: confirmed present on bookworm (`0.7.1-1.3`), trixie (`0.7.1-1.5`), Ubuntu noble, Ubuntu resolute [VERIFIED: same sources].
- `virt-viewer` (provides `remote-viewer` binary): confirmed present on bookworm (`11.0-2`), trixie (`11.0-3`), Ubuntu noble, Ubuntu resolute [VERIFIED: same sources].
- `looking-glass-client`: confirmed **present only on Ubuntu noble/jammy**, **absent from Debian bookworm/trixie and absent from Ubuntu resolute/questing** [VERIFIED: `packages.debian.org`/`packages.ubuntu.com` per-release lookups returned HTTP "Error" pages for the 3 targets lacking it] — see Pitfall 4, this changes the `Depends:` design.

## Package Legitimacy Audit

This phase's *build-time* dependency surface (new `Cargo.toml` entries) is minimal — `cargo-deb`
is a developer-installed CLI tool (`cargo install cargo-deb` or downloaded as a CI-cached binary),
not a `[dependencies]` entry in the shipped binary. No new Rust crates need to be added to
`Cargo.toml` for this phase (the packaging step operates on the already-built binary). Runtime
dependencies below are OS package names declared in the `.deb`'s `Depends`/`Recommends` fields,
not crates — the crate-level "SLOP/SUS/OK" gate doesn't directly apply to OS package names, but
the same "verify before trusting a name" discipline was applied via direct registry/archive
lookups:

| Package | Registry | Age/Provenance | Availability | Disposition |
|---------|----------|-----|-----------|-------------|
| `cargo-deb` | crates.io | Long-lived (100+ published versions per crates.io version-ID history fetched this session), actively maintained, canonical repo `github.com/kornelski/cargo-deb` | v3.7.0 current | Approved — build-tool only, not a runtime `Cargo.toml` dependency |
| `qemu-system-x86` | Debian/Ubuntu archive | Official Debian `qemu` source package binary; present in every stable/LTS suite checked | trixie, bookworm, noble, resolute — all confirmed | Approved — hard `Depends` |
| `swtpm` | Debian/Ubuntu archive | Official Debian package | trixie, bookworm, noble, resolute — all confirmed | Approved — hard `Depends` |
| `virt-viewer` | Debian/Ubuntu archive | Official Debian package (provides `remote-viewer`) | trixie, bookworm, noble, resolute — all confirmed | Approved — `Recommends` (optional; only needed for SPICE/VNC display type) |
| `looking-glass-client` | Debian/Ubuntu archive | Official Debian package where present | **noble + jammy only**; absent from bookworm/trixie/resolute | Flagged — **must be `Recommends`, never `Depends`** (installing the `.deb` on 3 of 4 targets would otherwise fail with an unsatisfiable dependency); document that Looking Glass display users on Debian/26.04 must install `looking-glass-client` from upstream release tarballs, not apt |

**Packages removed due to `[SLOP]` verdict:** none.
**Packages flagged as suspicious `[SUS]`:** none — `looking-glass-client` is flagged for
*availability*, not legitimacy; it is a real, long-standing upstream project
([looking-glass.io](https://looking-glass.io)), just inconsistently packaged across the 4 targets.

## Architecture Patterns

### System Architecture Diagram

```
                 ┌─────────────────────────┐
 developer /     │   `cargo deb --target   │
 CI runner       │   x86_64-unknown-       │
                 │   linux-musl`           │
                 └────────────┬────────────┘
                              │ reads [package.metadata.deb]
                              ▼
                 ┌─────────────────────────┐
                 │ target/…/debian/        │
                 │  ezkvm_0.1.0-1_amd64.deb│
                 └────────────┬────────────┘
                              │ dpkg -i (on each of 4 test targets)
                              ▼
     ┌──────────────────────────────────────────────────┐
     │ postinst script (D-03/D-04):                     │
     │  1. groupadd ezkvm (idempotent)                  │
     │  2. install -d -o root -g ezkvm /etc/ezkvm       │
     │  3. install -d -o root -g ezkvm /var/lib/ezkvm   │
     │  4. (tmpfiles.d asset creates /run/ezkvm @ boot) │
     └────────────────────────┬─────────────────────────┘
                              │ user in `ezkvm` group runs CLI
                              ▼
     ┌──────────────────────────────────────────────────┐
     │  ezkvm start <vm>  (src/lifecycle/start.rs)      │
     │   reads /etc/ezkvm/host.yaml + /etc/ezkvm/vm.d/  │
     │   spawns swtpm (if TPM) → waits socket ready     │
     │   spawns qemu-system-x86_64 (+QMP unix socket)   │
     │   writes /run/ezkvm/<vm>.state (VmHandle)        │
     │   optionally launches remote-viewer / LG client  │
     └────────────────────────┬─────────────────────────┘
                              │ QMP unix socket
                              ▼
     ┌──────────────────────────────────────────────────┐
     │ ezkvm stop / kill / reset <vm>                   │
     │  reads /run/ezkvm/<vm>.state, sends QMP command  │
     │  (system_powerdown / quit / system_reset)        │
     └──────────────────────────────────────────────────┘
```

CI verification wraps the above inside 4 parallel jobs, each targeting one distro/version
container with `--device /dev/kvm`:

```
GitHub Actions matrix job (per distro/version)
  └─ docker run --device /dev/kvm --rm -v $PWD/target/.../*.deb:/pkg.deb <distro-image>
       └─ apt-get install -y /pkg.deb
       └─ useradd -m ci; usermod -aG ezkvm,kvm ci
       └─ su ci -c "ezkvm start coruscant100 && sleep 5 && ezkvm status coruscant100
                     && ezkvm stop coruscant100"
```

### Recommended Project Structure
```
Cargo.toml                 # + [package.metadata.deb] section (this phase)
debian/                    # cargo-deb maintainer-scripts dir (new, this phase)
├── postinst                 # group creation + /etc,/var/lib dir creation
├── postrm                    # group removal (only on purge) + dir cleanup
├── ezkvm.conf                 # for /usr/lib/tmpfiles.d/ezkvm.conf via assets
└── host.yaml.example          # shipped default config, copied to /etc/ezkvm/host.yaml
.github/
└── workflows/
    └── package-verify.yml    # new — 4-target matrix, build + install + real-boot smoke test
```

### Pattern 1: cargo-deb metadata block (D-01/D-03/D-04)
**What:** All packaging decisions expressed declaratively in `Cargo.toml`.
**When to use:** Always, for this phase — avoids a parallel `debian/rules` build system.
**Example:**
```toml
# Source: cargo-deb README, fetched directly this session
# (https://github.com/kornelski/cargo-deb — README.md "Configuration" section)
[package.metadata.deb]
maintainer = "ezkvm maintainers"
copyright = "2026, ezkvm contributors"
depends = "qemu-system-x86, swtpm"
recommends = "virt-viewer, looking-glass-client"
section = "admin"
priority = "optional"
maintainer-scripts = "debian/"
assets = [
    ["target/release/ezkvm", "usr/bin/", "755"],
    ["debian/host.yaml.example", "etc/ezkvm/host.yaml", "640"],
    ["debian/ezkvm.conf", "usr/lib/tmpfiles.d/ezkvm.conf", "644"],
]
conf-files = ["/etc/ezkvm/host.yaml"]
```
Note: `depends`/`recommends` are plain strings (Debian control-file syntax), not arrays — verify
exact key names/syntax against the installed `cargo-deb --help`/README at plan time since minor
config-key spelling can change between major versions.

### Pattern 2: postinst group + directory creation (D-03/D-04)
**What:** Idempotent group creation and FHS directory setup at install time.
**When to use:** `debian/postinst`, invoked by dpkg on `configure`.
**Example:**
```sh
#!/bin/sh
# Source: Debian Policy Manual §maintainer scripts conventions (standard idiom)
set -e
if [ "$1" = "configure" ]; then
    getent group ezkvm >/dev/null || addgroup --system ezkvm
    install -d -o root -g ezkvm -m 0770 /var/lib/ezkvm
    # /etc/ezkvm already created by dpkg from the `assets` entries above;
    # tighten group ownership here since dpkg defaults to root:root
    chgrp ezkvm /etc/ezkvm
    chmod 0750 /etc/ezkvm
fi
#DEBHELPER#
```

### Pattern 3: tmpfiles.d for `/run/ezkvm` (discretion item, D-02-compatible)
**What:** A plain (non-systemd-service) tmpfiles.d drop-in recreates `/run/ezkvm` on every boot,
since `/run` is a tmpfs wiped on reboot.
**When to use:** Ship as a plain `assets` entry — do NOT use cargo-deb's
`[package.metadata.deb.systemd-units]` feature, which is for actual services and would require
`#DEBHELPER#`-token wiring this phase doesn't need.
**Example:**
```
# Source: systemd-tmpfiles(5) man page conventions; file: debian/ezkvm.conf
# installed to /usr/lib/tmpfiles.d/ezkvm.conf
d /run/ezkvm 0770 root ezkvm -
```
This works identically whether or not systemd is PID 1 for *running* a service (it isn't — D-02) —
`systemd-tmpfiles` itself runs as a one-shot boot-time helper on virtually all Debian/Ubuntu
installs regardless of what services are enabled, and is standard practice for non-daemon Debian
packages that just need a `/run` subdirectory to exist.

### Anti-Patterns to Avoid
- **Depending on `qemu-system-x86_64` as a package name:** No such Debian/Ubuntu binary package
  exists — the binary `qemu-system-x86_64` ships inside the package named `qemu-system-x86`. Using
  the wrong name in `Depends:` breaks `apt`/`dpkg` dependency resolution entirely (see Pitfall 1).
- **Hard-`Depends`-ing on `looking-glass-client`:** Breaks package installability on Debian
  12/13 and Ubuntu 26.04, where the package doesn't exist in the archive (see Pitfall 4).
- **Using `[package.metadata.deb.systemd-units]` for the tmpfiles.d file:** That feature assumes
  you're installing/enabling/starting a systemd *service*; using it for a tmpfiles.d-only need
  drags in `#DEBHELPER#` script-augmentation machinery this phase doesn't need and risks
  reintroducing implicit systemd-service semantics D-02 explicitly rejects.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| `.deb` control file / md5sums / data.tar assembly | A custom `dpkg-deb` wrapper script | `cargo-deb` | Solved problem; hand-rolling risks subtly malformed control files (missing `Installed-Size`, wrong compression, wrong permissions bits) that only surface as `dpkg` install failures on a fresh system |
| musl cross-linking flags | Manually setting `RUSTFLAGS`/linker paths per distro | `rustup target add` + `musl-tools` (`cargo build --target x86_64-unknown-linux-musl`) | Rust's own target infrastructure already encodes the correct linker/sysroot wiring; manual flag-juggling is a common source of "works on my machine" packaging bugs |
| Debian version string parsing/comparison | Custom semver→Debian-version mapper | `Cargo.toml`'s version field used as-is (D-07) + `-1` Debian revision suffix (cargo-deb default) | Debian version comparison (`dpkg --compare-versions`) has well-known epoch/tilde semantics; don't invent a competing scheme for a v1, single-package release |

**Key insight:** Every part of this phase that looks like "custom infrastructure" (control files,
version strings, systemd unit installation shell fragments) already has a solved, widely-used
Rust-ecosystem-native answer in `cargo-deb`. The only genuinely custom code this phase should
write is the ~10-line `postinst`/`postrm` pair for D-03's group creation, which is inherently
project-specific and can't be generic-library-solved.

## Common Pitfalls

### Pitfall 1: Wrong package name for the QEMU binary
**What goes wrong:** Assuming the Debian/Ubuntu package is named `qemu-system-x86_64` (matching
the binary name) and declaring `Depends: qemu-system-x86_64` — this package does not exist; `apt`
will fail to resolve the dependency and the `.deb` cannot be installed.
**Why it happens:** The binary and the package are named differently — Debian's `qemu` source
package produces a binary package called `qemu-system-x86` (covering x86 and x86_64 targets
together), which in turn provides the `/usr/bin/qemu-system-x86_64` executable.
**How to avoid:** `Depends: qemu-system-x86` (confirmed identical name across all 4 targets this
session via `qa.debian.org/madison.php` and `packages.ubuntu.com`).
**Warning signs:** `apt-get install ./ezkvm_*.deb` fails with "Unable to locate package
qemu-system-x86_64" or similar dependency resolution error during CI's install step.

### Pitfall 2: musl target not installed / `musl-gcc` missing on the build host
**What goes wrong:** `cargo build --target x86_64-unknown-linux-musl` fails with a linker error
(`error: linker \`musl-gcc\` not found`) if the build host (dev machine or CI runner) is a
standard glibc-based image without the musl toolchain installed.
**Why it happens:** `rustup target add x86_64-unknown-linux-musl` installs the Rust *standard
library* for that target, but the underlying C linker toolchain (`musl-gcc`, provided by the
`musl-tools` apt package) is a separate, OS-level dependency `rustup` does not install.
**How to avoid:** In CI, always run `apt-get install -y musl-tools` before the first musl build
step. `cargo-deb`'s own README recommends `cross` or `cargo-zigbuild` as an escape hatch if the
plain `musl-tools` route proves fragile on a given CI image.
**Warning signs:** Build succeeds for the default (`x86_64-unknown-linux-gnu`) target but fails
only when `--target x86_64-unknown-linux-musl` is added.

### Pitfall 3: CI runner user not in `kvm` group / `/dev/kvm` permission denied
**What goes wrong:** `qemu-system-x86_64 -enable-kvm ...` fails with "Could not access KVM
kernel module: Permission denied" even though `/dev/kvm` exists on the runner and is passed
through to the container.
**Why it happens:** GitHub-hosted runners expose `/dev/kvm` but the default runner user (and, by
extension, the default container user after `--device /dev/kvm` passthrough) isn't automatically
a member of the `kvm` group on the host, and permissions on `/dev/kvm` inside the container don't
automatically match the group inside the container's own `/etc/group` — this is a
[documented, recurring issue](https://github.com/actions/runner-images/issues/8542) confirmed live
in this session via GitHub's `runner-images` issue tracker.
**How to avoid:** In the workflow, before starting the container-based test, run
`sudo chmod 666 /dev/kvm` on the runner host (a widely-used one-line fix — accept the reduced
security posture in a throwaway CI VM) OR ensure the container's test user is added to a `kvm`
group whose GID matches `/dev/kvm`'s host GID.
**Warning signs:** Container-level `ezkvm start` succeeds at spawning the `qemu-system-x86_64`
process (PID exists) but the process immediately exits non-zero, or KVM-specific QEMU flags fail.

### Pitfall 4: `looking-glass-client` is not available on 3 of the 4 required targets
**What goes wrong:** Declaring `looking-glass-client` as a hard `Depends:` makes the package
uninstallable via `apt` on Debian 12/13 and Ubuntu 26.04 — verified live this session: the
package exists only for Ubuntu 24.04 (noble)/22.04 (jammy) in the archives checked; Debian
bookworm/trixie and Ubuntu 26.04 (resolute) return no such package.
**Why it happens:** `looking-glass-client` has historically had inconsistent Debian/Ubuntu
archive presence (it briefly existed in Debian bullseye, was dropped, and currently exists only in
Debian `sid` — not yet migrated to a stable release — while Ubuntu carries it in some LTS-adjacent
releases but not others).
**How to avoid:** Declare it as `Recommends`, never `Depends`. Document in the package
README/host.yaml.example comments that Looking Glass display users on distros lacking the apt
package must build/install it from upstream ([looking-glass.io](https://looking-glass.io)
releases) themselves — this is out of scope for the `.deb` to solve.
**Warning signs:** `dpkg -i`/`apt install` fails with an unsatisfiable-dependency error specifically
on the Debian 12/13 and Ubuntu 26.04 test targets, while succeeding on Ubuntu 24.04.

### Pitfall 5: `felucia/108.conf` cannot boot inside CI — real hardware passthrough
**What goes wrong:** Using `felucia/108.conf` (`wakiza` VM — Windows 11, AMD RX 7700S GPU PCI
passthrough via `hostpci0: 0000:03:00,pcie=1,x-vga=1`, physical USB device passthrough via
`usb0: host=1-2.2`) as this phase's "real VM boot" fixture will fail unconditionally in any
container or standard CI VM, since neither a physical discrete GPU at PCI address `0000:03:00`
nor a physical USB device at bus-port `1-2.2` can exist inside a CI environment.
**Why it happens:** felucia/108 is this project's designated canonical/most-complex corpus fixture
for round-trip *conversion* correctness (Phase 9), but conversion correctness and bootability are
different concerns — a config can convert to a perfectly valid QEMU commandline that still refuses
to boot because the referenced hardware is absent on the executing host.
**How to avoid:** Use `coruscant/100.conf` instead for the automated/CI boot-smoke-test: it is a
plain Linux VM (`ostype: l26`) with `virtio-scsi-pci` disks, `virtio` NIC, `vga: virtio` (no SPICE,
no Looking Glass, no PCI/USB passthrough, no TPM) — every device it needs is either fully
software-emulated by QEMU or a standard virtio device, making it bootable inside any
KVM-passthrough container. Reserve felucia/108's actual boot (with the real GPU/USB attached) as
an explicitly documented **manual-only** verification step (checkpoint:human-verify) on real
hardware, separate from this phase's automated CI matrix. Storage backing for
`coruscant/100.conf`'s `scsi0`/`scsi1`/`scsi2` resources will need synthetic block devices (e.g.
sparse files exposed as `resource.storage.block_device`, or a temporary loop device) since the
original `/dev/vm1/...` LVM volumes referenced in `storage.cfg` don't exist outside the source
Proxmox host — the planner should confirm `ezkvm`'s Runtime layer accepts a `host_resources`
override pointing at synthetic disk-image files instead (this is exactly the kind of host-config
indirection `HostConfig`/`ResourceSchema` already appear designed for based on `host_resources:
Vec<ResourceSchema>` in `HostConfig`).
**Warning signs:** `ezkvm start wakiza` (or any felucia-derived config) inside a container hangs
or QEMU immediately errors with "vfio: error opening /dev/vfio/..." / "-device usb-host,...: 
failed to open USB device".

### Pitfall 6: Package `Installed-Size`/binary size surprises from static musl linking
**What goes wrong:** A musl-statically-linked release binary is meaningfully larger than the
equivalent dynamically-linked glibc build (all of libc plus everything ezkvm links against is
now embedded in the single executable) — this can surprise reviewers expecting a "small CLI tool"
`.deb` and, if `strip`/LTO settings aren't tuned, needlessly bloat the package.
**Why it happens:** Static linking always trades install-time dependency simplicity for binary
size; this is the correct, expected tradeoff D-08 explicitly chose, not a bug.
**How to avoid:** Ensure `cargo-deb`'s default stripping runs (or pass `--no-strip` deliberately
only if debug symbols are wanted for the CI verification builds); consider `[profile.release]
lto = true` / `strip = true` in `Cargo.toml` for the shipped package build. Document expected
`.deb` size in the phase's verification notes so a large-but-expected size isn't mistaken for a
packaging bug.
**Warning signs:** `.deb` file size in the hundreds of MB range instead of low tens of MB would be
worth investigating (not necessarily a hard bug, but worth a sanity check against `--release`
build without `strip`).

## Code Examples

### GitHub Actions matrix workflow skeleton (D-05)
```yaml
# Source: composed from cargo-deb README (build step) + GitHub runner-images issue #8542
# discussion (KVM permission fix) — this project has no pre-existing workflow to extend
# (.github/workflows/ does not exist yet).
name: package-verify
on: [push, pull_request]
jobs:
  build-deb:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup target add x86_64-unknown-linux-musl
      - run: sudo apt-get update && sudo apt-get install -y musl-tools
      - run: cargo install cargo-deb --locked
      - run: cargo deb --target x86_64-unknown-linux-musl
      - uses: actions/upload-artifact@v4
        with:
          name: ezkvm-deb
          path: target/x86_64-unknown-linux-musl/debian/*.deb

  verify:
    needs: build-deb
    strategy:
      matrix:
        image: ["debian:bookworm", "debian:trixie", "ubuntu:24.04", "ubuntu:26.04"]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with: { name: ezkvm-deb, path: dist/ }
      # KVM permission fix — see Pitfall 3
      - run: sudo chmod 666 /dev/kvm
      - run: |
          docker run --rm --device /dev/kvm \
            -v "$PWD/dist:/dist" -v "$PWD:/repo:ro" "${{ matrix.image }}" \
            bash -c '
              set -eux
              apt-get update
              apt-get install -y /dist/*.deb qemu-utils
              useradd -m ci && usermod -aG ezkvm,kvm ci
              su ci -c "ezkvm start coruscant100"
              sleep 5
              su ci -c "ezkvm status coruscant100"
              su ci -c "ezkvm stop coruscant100"
            '
```

### Synthetic block-device resources for CI (Pitfall 5)
```yaml
# Source: inferred from src/lifecycle/host_config.rs's `host_resources: Vec<ResourceSchema>`
# and ResourceSchema (src/config/ezkvm/schema) — planner must confirm exact ResourceSchema
# shape at implementation time; not independently re-derived in this research session beyond
# confirming the field exists and is Vec<ResourceSchema>.
# host.yaml (CI-only override):
host_resources:
  - id: storage0
    storage:
      block_device: /tmp/ezkvm-ci/coruscant100-boot.img   # sparse file, `truncate -s 10G`
  - id: storage1
    storage:
      block_device: /tmp/ezkvm-ci/coruscant100-root.img
  - id: storage2
    storage:
      block_device: /tmp/ezkvm-ci/coruscant100-swap.img
  - id: net0
    network:
      bridge: vmbr0   # or a CI-created dummy/veth bridge — confirm at plan time
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-distro glibc `.deb` builds (one per Debian/Ubuntu version) | Single musl-static `.deb` | Long-standing technique, not new — but this is this project's first packaging phase, so it's "new to ezkvm" | Cuts CI/build matrix in half (build once, test on 4) and removes glibc-symbol-version fragility entirely |
| `debian/rules`+`dpkg-buildpackage` hand-rolled packaging | `cargo-deb` declarative `[package.metadata.deb]` | `cargo-deb` has been the Rust-ecosystem standard for years; nothing new changed here, just the recommended choice for this project | Single source of truth for version (D-07), far less boilerplate |

**Deprecated/outdated:** None specific to this phase — no library here is being replaced from a
prior ezkvm decision; this is greenfield packaging work.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `musl-tools` apt package name is identical (`musl-tools`) across all 4 target distro/version combinations | Standard Stack | Low — this is a build-host-only dependency (not shipped in the `.deb`), and even if the package name differed slightly on one distro, the fix is a one-line CI script change, not a design change |
| A2 | GitHub Actions is the CI provider this phase should target (no `.github/workflows/` exists yet, so this is a recommendation, not a confirmed existing convention) | Summary, Validation Architecture | Medium — if the project actually uses another CI provider not visible in this repo checkout, the workflow YAML syntax needs porting, but the underlying Docker+KVM-passthrough approach transfers to any CI vendor with Docker+privileged/device-passthrough support |
| A3 | `coruscant/100.conf`'s referenced LVM volumes can be safely replaced with synthetic sparse-file block devices via `HostConfig.host_resources` without further Runtime-layer code changes | Pitfall 5, Code Examples | Medium — if `ResourceSchema`'s `storage.block_device` strictly validates the path as an existing block device (not a regular file), a loop-mounted file (`losetup`) may be needed instead of a raw sparse file; planner should verify against `ResourceSchema`'s actual validation logic before writing the CI fixture |
| A4 | The Debian binary package name `qemu-system-x86` (not `qemu-system-x86_64`) is intentional upstream Debian packaging convention, not a temporary quirk that might change before Debian 13/Ubuntu 26.04 GA | Pitfall 1 | Low — confirmed identically named across bookworm (already GA), trixie (already GA as of this session's date), noble (GA), and resolute (GA) — this is a stable, long-standing Debian naming convention (the `qemu` source package has used this binary-package split for over a decade) |
| A5 | `musl-tools`' `musl-gcc` alone is sufficient for this codebase's musl cross-build (no additional per-crate C toolchain needed) | Standard Stack, Pitfall 2 | Low — corroborated by the Cargo.lock dependency audit showing zero C-linked/`-sys` crates, but not verified by actually performing a musl build in this session (no musl target/toolchain installed on this research machine) |

**If this table is empty:** N/A — see entries above. All other tooling/package-name claims in
this document were independently verified via live `curl` requests to crates.io, the GitHub API,
Debian's `qa.debian.org/madison.php`, and `packages.debian.org`/`packages.ubuntu.com` during this
research session (not relying on training-data recall alone).

## Open Questions (RESOLVED)

All 3 questions below were left open during research but are demonstrably resolved by the
resulting plans (`10-01-PLAN.md`/`10-02-PLAN.md`) — see each item's `RESOLVED` note.

1. **Exact `ResourceSchema.storage` validation behavior for non-block-device files**
   - What we know: `HostConfig.host_resources: Vec<ResourceSchema>` exists and is read at
     `HostConfig::load` time; `coruscant/100.conf`'s original config references LVM volumes
     (`vm1-pool:vm-100-boot` etc.) that won't exist in a CI container.
   - What's unclear: Whether `ResourceSchema`'s storage variant requires the `block_device` path
     to literally be a block special file (`/dev/...`) or accepts a plain regular file/sparse
     image transparently.
   - Recommendation: Planner's first CI-fixture task should read `src/config/ezkvm/schema/*.rs`
     (`ResourceSchema`) and, if needed, either use `losetup` to present a sparse file as a real
     loop block device inside the container, or confirm QEMU's own `-drive file=...` handling
     (which does accept plain regular files as disk images) means the *Rust* layer's path
     validation is the only thing that might reject a non-`/dev/*` path, not QEMU itself.
   - **RESOLVED:** `StorageResourceSchema::File { file: String }` exists in
     `src/config/ezkvm/schema/resources.rs` (confirmed live) — a plain regular/sparse file is
     natively accepted, no `losetup`/block-device workaround needed. `10-01-PLAN.md` Task 2's
     `read_first` cites this variant directly.

2. **Which CI provider does the ezkvm project actually intend to standardize on?**
   - What we know: no `.github/workflows/` directory currently exists in this repo — CI is
     entirely unestablished.
   - What's unclear: Whether GitHub Actions is a firm choice or simply the most obvious default
     given the repo is hosted on GitHub (`hurenkam/ezkvm`).
   - Recommendation: Default to GitHub Actions (repo is already on GitHub; `gh` CLI is listed as
     an available tool in this environment) unless the planner/user specifies otherwise during
     `/gsd-plan-phase 10`'s planning conversation.
   - **RESOLVED:** GitHub Actions was adopted consistently across all three plans
     (`.github/workflows/package-verify.yml`) — no objection was raised during planning.

3. **Debian revision suffix / `Installed-Size` acceptable ranges**
   - What we know: `cargo-deb` defaults to a `-1` Debian revision suffix appended to the
     `Cargo.toml` version (e.g. `0.1.0-1`), consistent with D-07's "track Cargo.toml directly, no
     separate changelog process."
   - What's unclear: Whether a future re-packaging of the *same* `Cargo.toml` version (e.g. a
     `postinst` script fix with no source code change) should bump to `0.1.0-2`, and whether that
     convention needs to be documented now or left to encounter later.
   - Recommendation: Note the `-N` revision-suffix convention in the package's README now (D-07
     already implicitly allows this — Debian versioning is monotonic/additive) so it isn't
     rediscovered ad hoc mid-phase.
   - **RESOLVED:** Documented in `10-01-PLAN.md`'s `debian/host.yaml.example` header and
     `10-02-PLAN.md`'s `Cargo.toml` comment (the `-N` suffix bumps on packaging-only changes with
     no `Cargo.toml` version bump).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / `rustc` | Build | ✓ | 1.94.0 | — |
| `rustup` | musl target management | ✓ | 1.29.0 | — |
| `x86_64-unknown-linux-musl` Rust target | D-08 static build | ✗ (not installed on this research machine) | — | `rustup target add x86_64-unknown-linux-musl` — trivial, no blocker |
| `musl-tools` (`musl-gcc`) | musl cross-linking | ✗ (not installed on this research machine) | — | `apt-get install musl-tools` — trivial, no blocker |
| `cargo-deb` | D-01 packaging | ✗ (not installed on this research machine) | — | `cargo install cargo-deb` — trivial, no blocker; confirmed real/current via crates.io/GitHub API this session |
| Docker | D-05 container test matrix | not checked on this research machine (network-restricted sandbox, no Docker daemon assumed) | — | GitHub-hosted runners ship Docker preinstalled — not a local-dev blocker |
| `/dev/kvm` (this research machine) | Not required for research; required for actual CI execution | not checked (out of scope — research was source/registry investigation, not a build) | — | N/A |
| `.github/workflows/` | CI scaffolding | ✗ (directory does not exist in this repo) | — | Must be created from scratch this phase — not a blocker, just confirms no existing pattern to extend |

**Missing dependencies with no fallback:** none — every missing tool/target above has a
one-line, well-documented install fallback.

**Missing dependencies with fallback:** `x86_64-unknown-linux-musl` target, `musl-tools`,
`cargo-deb` (all trivially installable); Docker availability on GitHub-hosted runners is a known
default, not something this phase needs to provision.

## Validation Architecture

`nyquist_validation` is `true` in `.planning/config.json` (not absent) — section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust's built-in `#[test]`/`cargo test` for existing integration tests (`tests/*.rs`); this phase ADDS a non-`cargo-test` layer — shell-driven Docker/GitHub-Actions verification, since real package install + real QEMU boot is not expressible as a plain Rust unit test |
| Config file | none new for the Rust side; new `.github/workflows/package-verify.yml` for the packaging/boot-verification side |
| Quick run command | `cargo deb --target x86_64-unknown-linux-musl` (build-only smoke test; confirms packaging metadata is well-formed without a full container run) |
| Full suite command | The GitHub Actions matrix job (4 containers × install + `ezkvm start`/`status`/`stop`/`kill`/`reset` cycle) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| QEMU-04 (existing, unchecked) | `coruscant/100.conf`-derived VM starts in real QEMU | integration (containerized) | `docker run --device /dev/kvm ... ezkvm start coruscant100` | ❌ Wave 0 — new CI fixture + workflow |
| DEPLOY-01 (proposed) | `.deb` builds successfully via `cargo-deb` | build smoke test | `cargo deb --target x86_64-unknown-linux-musl` | ❌ Wave 0 — needs `[package.metadata.deb]` added to `Cargo.toml` |
| DEPLOY-02 (proposed) | Package installs cleanly, `ezkvm` group + FHS dirs created | integration (containerized), per target | `docker run ... apt-get install -y /dist/*.deb && getent group ezkvm && test -d /etc/ezkvm && test -d /var/lib/ezkvm` | ❌ Wave 0 — new `postinst`/`postrm` scripts |
| DEPLOY-03 (proposed) | `Depends`/`Recommends` correctly resolved on all 4 targets | integration (containerized), per target | same install step as DEPLOY-02, asserting exit code 0 | ❌ Wave 0 |
| DEPLOY-04 (proposed) | `ezkvm start`/`status`/`stop`/`kill`/`reset` all succeed against real QEMU inside container | integration (containerized), per target | full lifecycle sequence in workflow skeleton above | ❌ Wave 0 — new synthetic-disk CI fixture (see Pitfall 5) |
| felucia/108 real-hardware boot | Boots with real GPU/USB passthrough | manual-only (`checkpoint:human-verify`) | N/A — requires physical hardware, cannot automate | manual-only, document explicitly |

### Sampling Rate
- **Per task commit:** `cargo deb --target x86_64-unknown-linux-musl` (fast — confirms packaging
  metadata parses and the binary still builds for the musl target; does not require Docker/KVM).
- **Per wave merge:** Full 4-target Docker/KVM matrix (the actual `package-verify.yml` workflow).
- **Phase gate:** All 4 targets green (install + full start/status/stop/kill/reset cycle) before
  `/gsd-verify-work`; felucia/108's real-hardware boot remains an explicitly separate, manual,
  non-blocking verification note (cannot gate an automated phase-completion on hardware this
  project's CI will never have).

### Wave 0 Gaps
- [ ] `Cargo.toml` — add `[package.metadata.deb]` section (no file exists yet for this)
- [ ] `debian/postinst`, `debian/postrm` — group creation/removal, directory setup (new)
- [ ] `debian/ezkvm.conf` (tmpfiles.d drop-in) and `debian/host.yaml.example` (default config) — new
- [ ] `.github/workflows/package-verify.yml` — new, no existing CI to extend
- [ ] CI-only synthetic disk-image fixture generation for `coruscant/100.conf` (script or workflow
      step producing sparse files / loop devices in place of the original LVM volumes) — new

*(Framework install: none needed — `cargo test` infra already exists from prior phases; the new
layer here is entirely packaging/container tooling, not Rust test-framework tooling.)*

## Security Domain

`security_enforcement` is `true` in `.planning/config.json` (not absent) — section included.
`security_asvs_level`: 1, `security_block_on`: "high".

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | No authentication surface introduced by packaging |
| V3 Session Management | No | Not applicable |
| V4 Access Control | Yes | D-03's `ezkvm` group is itself the access-control primitive gating `/dev/kvm` and (per D-04) `/etc/ezkvm`/`/var/lib/ezkvm`/`/run/ezkvm` access; `postinst` must set restrictive group ownership (`0750`/`0770`, never world-writable) on all three FHS directories — this is the single most important security-relevant decision this phase's `postinst` script makes |
| V5 Input Validation | Yes (inherited, unchanged) | Existing `HostConfig::resolve_vm_name` (rejects `/`, `..`, leading `.`) and `VmHandle`'s `O_NOFOLLOW`/symlink-rejection already provide V5 controls at the CLI layer; packaging doesn't add new untrusted-input surfaces, but the shipped default `/etc/ezkvm/host.yaml` must not itself introduce an insecure default (e.g. don't ship `qemu_default_args` granting extra privileges) |
| V6 Cryptography | No | Not applicable — packaging introduces no secrets/crypto |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| World-writable or root-writable-only-but-group-`ezkvm`-readable `/etc/ezkvm`/`/var/lib/ezkvm` allowing any `ezkvm`-group member to read other users' VM configs/state | Information Disclosure / Elevation of Privilege | `postinst` sets `0750`/`0770` (group-restricted, not world-readable) on all FHS directories per D-04; never `0777` or `chmod -R a+rw` shortcuts even for "convenience" during development |
| `postinst`/`postrm` scripts running as root at install time performing unsafe operations (unquoted variables, following symlinks when creating directories) | Tampering | Use `install -d` (not `mkdir -p` + separate `chmod`/`chown`, which has a TOCTOU gap) for atomic directory-creation-with-permissions; scripts must `set -e` and avoid interpolating any external/attacker-influenced input (none exists in this phase's scripts, but keep the discipline) |
| Adding the invoking user to the `ezkvm` group grants `/dev/kvm` access — equivalent to significant local-privilege capability (KVM guests can, depending on QEMU flags, access host resources) | Elevation of Privilege | This is an accepted, documented tradeoff already locked by D-03 (same tradeoff the standard `kvm` group makes) — no new mitigation needed beyond the existing documentation requirement to warn users what `ezkvm`-group membership grants |
| CI workflow's `sudo chmod 666 /dev/kvm` (Pitfall 3 fix) loosens host device permissions for the duration of the CI job | Elevation of Privilege (CI-scoped only) | Acceptable **only** in a throwaway, single-job GitHub-hosted CI VM that is destroyed after the job — must NOT be replicated as installation guidance for real user machines (real users should be added to a proper group, not `chmod 666` their own `/dev/kvm`) |

## Sources

### Primary (HIGH confidence — direct repo source reading)
- `src/main.rs` — confirmed CLI shape (`ezkvm --config-dir <dir> start|stop|kill|reset|status
  <vm_name>`, `--config-dir` defaults to `/etc/ezkvm`)
- `src/lifecycle/host_config.rs` — confirmed `state_dir` defaults to `/run/ezkvm`, `vm_dir`
  defaults to `<config_dir>/vm.d`, and `HostConfig` requires absolute tool paths in `host.yaml`
- `src/lifecycle/start.rs`, `src/lifecycle/ui_client.rs` — confirmed swtpm/qemu/UI-client spawn
  logic and that UI client launch is conditional on per-VM `display` config, not unconditional
- `src/lifecycle/process.rs`, `src/lifecycle/vm_handle.rs` — confirmed `libc` usage is limited to
  plain syscalls (`setsid`, `kill`, `O_NOFOLLOW`), no musl-incompatible patterns
- `Cargo.toml`, `Cargo.lock` — confirmed dependency set (46 crates total), zero C-linked/`-sys`
  crates, confirming no known musl cross-compilation blockers
- `input/felucia/108.conf`, `input/coruscant/100.conf` — confirmed hardware-passthrough content
  of the canonical fixture vs. a CI-safe alternative
- `.planning/PROJECT.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`,
  `.planning/phases/10-deployment-packaging/10-CONTEXT.md`/`10-DISCUSSION-LOG.md`,
  `.planning/phases/08-vm-lifecycle/08-CONTEXT.md` — phase scope, locked decisions, prior
  testing-discipline precedent
- `.planning/config.json` — confirmed `nyquist_validation: true`, `security_enforcement: true`,
  `security_asvs_level: 1`

### Primary (HIGH confidence — live external verification via `curl` this session)
- `https://raw.githubusercontent.com/kornelski/cargo-deb/main/README.md` — cargo-deb
  configuration surface (`assets`, `maintainer-scripts`, `conf-files`, `--target` cross-compilation,
  musl recommendation)
- `https://raw.githubusercontent.com/kornelski/cargo-deb/main/systemd.md` — `systemd-units`
  feature scope/behavior (confirmed why it should NOT be used for the plain tmpfiles.d need)
- `https://api.github.com/repos/kornelski/cargo-deb/releases/latest` — confirmed `v3.7.0`
- `https://crates.io/api/v1/crates/cargo-deb` — confirmed crate exists, actively updated
- `https://qa.debian.org/madison.php?package=<pkg>` — confirmed `qemu-system-x86`, `swtpm`,
  `virt-viewer` present on bookworm/trixie; confirmed `looking-glass-client` present only in
  bullseye/sid, absent bookworm/trixie
- `https://packages.ubuntu.com/<release>/<pkg>` — confirmed `qemu-system-x86`, `swtpm`,
  `virt-viewer` present on noble/resolute; confirmed `looking-glass-client` present on
  noble/jammy only, absent resolute/questing
- `https://api.github.com/search/issues?q=repo:actions/runner-images+...` — confirmed `/dev/kvm`
  exists on GitHub-hosted x86_64 Linux runners (issue titles/context re: ARM-only gap, kvm-group
  membership fix) and confirmed this project has no `.github/workflows/` to build on

### Secondary (MEDIUM confidence)
- `musl-tools` apt package name assumed identical across all 4 targets — standard, long-standing
  convention, not independently re-verified per-distro this session (see Assumption A1)
- GitHub Actions as the CI provider — inferred from repo hosting, not confirmed against an
  explicit user/project decision (see Open Question 2)

### Tertiary (LOW confidence / `[ASSUMED]`)
- Exact musl cross-build success for this specific codebase — inferred from a clean Cargo.lock
  dependency audit (zero C-linked crates), not from an actual musl build execution in this
  research session (no musl target/toolchain installed on the research sandbox — see Environment
  Availability)

## Metadata

**Confidence breakdown:**
- Standard stack (cargo-deb, package names): HIGH — verified via direct API/archive lookups this
  session, not training-data recall
- Architecture (FHS layout, postinst patterns): HIGH — cross-checked against both cargo-deb's own
  docs and this codebase's existing `HostConfig` defaults, which already anticipate `/etc/ezkvm`
  and `/run/ezkvm`
- Test environment (D-05, KVM-in-CI): MEDIUM-HIGH — GitHub `runner-images` issue tracker
  confirms `/dev/kvm` exists and the permission-fix pattern, but no actual container+KVM run was
  performed in this research session (network-restricted sandbox, no Docker daemon)
- Pitfalls: HIGH — all 6 pitfalls are either directly reproduced from source-reading (Pitfall 5's
  hardware dependency, confirmed by reading the actual `.conf` file) or confirmed via live
  external archive/issue-tracker lookups (Pitfalls 1, 3, 4)

**Research date:** 2026-07-30
**Valid until:** ~30 days for the packaging-tooling recommendations (cargo-deb/Cargo ecosystem
moves slowly); ~14 days for the specific package-availability findings (Debian/Ubuntu archives,
especially `looking-glass-client`'s sid→stable migration status, can change between point
releases) — re-verify package names/availability at plan time if this research is more than 2
weeks old.
