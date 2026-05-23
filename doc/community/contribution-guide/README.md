# Contributing (Developer Workflow)

This document defines the minimum workflow for architecture-safe contributions.

## Required Reading

1. `doc/dev/workflow/coding-guidelines.md`
2. `doc/dev/architecture/architecture-guidelines.md`
3. `doc/dev/architecture/module-ownership.md`
4. Relevant ADRs in `doc/dev/architecture/decisions/`
5. `doc/dev/architecture/extensibility-seams.md` when touching import or runtime extension boundaries

## Standard Workflow

1. Open or link a tracked task.
2. Confirm layer ownership impact before coding.
3. Implement minimal, focused change.
4. Add/update tests for behavior changes.
5. Run quality gates:
   - `cargo fmt --all --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --quiet`
6. Update docs when user-facing or architecture behavior changes.

## Layer-Boundary Review Checklist

Use this checklist in PR review for architecture-affecting changes.

1. Dependency direction respected: `CLI -> Config -> QEMU -> Runtime -> OS`
2. No circular module dependencies introduced.
3. `cli/` changes do not absorb schema or process execution internals.
4. `config/` changes keep validation/merge/schema responsibilities local.
5. `qemu/` changes keep deterministic command-building concerns local.
6. `runtime/` orchestration remains coordination-only (no schema rewrites).
7. `state/` remains metadata storage/access only.
8. `import/` outputs canonical schema only (no alternate runtime schema).
9. Anti-patterns avoided: cross-layer callbacks, mutable singletons, hidden side effects.
10. At least one integration or regression test covers the changed layer path.

## Building Packages

### Debian Package (Debian Trixie / Ubuntu 26.04 Resolute)

The `debian/` directory uses `dh-cargo` and produces a cross-distro `.deb`.

1. Install build prerequisites:
   ```bash
   sudo apt install build-essential cargo rustc debhelper dh-cargo devscripts fakeroot lintian
   ```
2. Build from the repository root:
   ```bash
   dpkg-buildpackage -us -uc -b
   ```
3. The `.deb` is placed one directory above the repo root. Install with:
   ```bash
   sudo dpkg -i ../ezkvm_*.deb
   ```

The package is validated against both Debian Trixie and Ubuntu 26.04.
See `debian/control` for the runtime dependency policy and `doc/backlog/implemented_features/DEBIAN_PACKAGE.md` for the full packaging contract.

### Arch Linux Package

The `pkg/arch/` directory contains a `PKGBUILD` and a helper script.

1. Install build prerequisites:
   ```bash
   sudo pacman -S --needed rust cargo base-devel
   ```
2. Build using the helper:
   ```bash
   cd pkg/arch
   ./build.sh
   ```
3. Install the resulting package:
   ```bash
   sudo pacman -U ezkvm-*.pkg.tar.zst
   ```

See `pkg/arch/PKGBUILD` for the full package definition.

## Documentation Expectations

1. Link new architecture decisions in `doc/dev/architecture/decisions/`.
2. Update `doc/dev/architecture/module-ownership.md` if ownership boundaries change.
3. Update user docs if commands/behavior change.
