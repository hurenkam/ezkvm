---
name: "Cross-Distro Debian Package Guard"
description: "Use when editing Debian packaging metadata, install/runtime paths, or package dependency policy to ensure one ezkvm .deb per architecture remains installable on both Debian Trixie and Ubuntu 26.04."
applyTo: "debian/**, Cargo.toml, README.md, doc/CONFIGURATION.md, doc/user/**/*.md, etc/**/*.yaml"
---

# Cross-Distro Debian Package Guard

For packaging-related changes, maintain a single cross-distro package contract unless explicitly approved otherwise.

## Required Contract

1. Produce one binary package per CPU architecture (`amd64`, `arm64`, etc.) that is intended to install on both Debian Trixie and Ubuntu 26.04.
2. Keep runtime dependencies in the shared Debian/Ubuntu baseline.
3. Use `Recommends` for optional integrations whenever possible.
4. If package names may differ by distro, use alternative dependency expressions in `Depends` rather than introducing distro-specific package variants by default.
5. Keep filesystem layout stable:
   - `/usr/bin` for executables
   - `/etc/ezkvm/` for configuration defaults
   - `/run/ezkvm` (or `/var/run/ezkvm`) for runtime sockets/state
   - `/var/lib/ezkvm` and `/var/log/ezkvm` only when required by behavior
6. Preserve conffile-safe behavior for admin-edited configs.

## Required Validation Evidence

When packaging metadata or runtime defaults change, require evidence for both distros:

1. Build and metadata checks:
   - `dpkg-buildpackage -us -uc -b` (or equivalent)
   - `lintian` artifact validation
2. Installation checks on Debian Trixie and Ubuntu 26.04:
   - install success
   - dependency resolution success
3. Runtime checks on both:
   - required files/dirs present
   - ezkvm dry-run/start confirms QEMU launch readiness
4. Upgrade behavior check:
   - admin-edited conffiles are preserved

## Escalation Rule

If shared dependency baseline cannot be maintained, document why and propose the minimal split package strategy (`common` + distro shim) with explicit approval notes.

## Final Response Requirement

For matched changes, summarize:
- dependency policy deltas
- path/runtime directory deltas
- dual-distro validation status (`pass`, `partial`, `not run`)
