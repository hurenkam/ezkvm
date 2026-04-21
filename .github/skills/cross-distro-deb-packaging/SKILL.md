---
name: cross-distro-deb-packaging
description: "Use when creating or reviewing ezkvm Debian packaging intended to run on both Debian Trixie and Ubuntu 26.04 from one binary package per architecture."
argument-hint: "What packaging change should remain cross-distro compatible?"
user-invocable: true
---

# Cross-Distro Deb Packaging

Scope: Build and validate one installable ezkvm package per architecture that works on both Debian Trixie and Ubuntu 26.04.

## What This Skill Produces

- A packaging plan that targets both distros with a shared dependency baseline.
- Concrete `debian/*` updates for dependencies, file layout, and runtime directory policy.
- Validation evidence for build, install, and runtime behavior on both distros.
- A pass/fail release gate for cross-distro compatibility.

## Description

This skill standardizes the workflow for maintaining one binary `.deb` per architecture across Debian Trixie and Ubuntu 26.04. It minimizes distro branching, keeps dependency policy explicit, and requires dual-distro install/runtime checks before marking packaging work complete.

## Use When

- editing `debian/control`, `debian/rules`, `debian/install`, or maintainer scripts
- changing package dependencies or package split strategy
- changing default runtime/config paths under `/etc`, `/run`, `/var/lib`, or `/var/log`
- validating release-readiness for distro compatibility
- reviewing packaging PRs for compatibility regressions

## Workflow

1. Define architecture scope
   - Select target architectures (`amd64`, `arm64`, etc.).
   - Produce one binary artifact per target architecture.

2. Set dependency policy
   - Keep `Depends` limited to shared baseline packages.
   - Use `Recommends` for optional capabilities.
   - Use alternative dependency expressions when package names vary by distro.

3. Align filesystem and runtime policy
   - Ensure executable, config, runtime, and state/log paths follow agreed layout.
   - Keep conffile behavior safe for admin-edited configuration.

4. Build and lint artifacts
   - Build with `dpkg-buildpackage -us -uc -b` or equivalent.
   - Run `lintian` and inspect package contents (`dpkg-deb -c`).

5. Validate on both distros
   - Install on Debian Trixie and Ubuntu 26.04.
   - Verify dependency resolution and runtime launch readiness.
   - Validate runtime directory creation and ownership expectations.

6. Record exceptions
   - If compatibility cannot be met with one package contract, document reason and propose minimal split strategy.

## Distro Delta Handling

- Prefer a shared package definition first.
- Handle naming differences with alternative dependency expressions.
- Avoid distro-specific maintainer script branching unless unavoidable and documented.

## Completion Gate

- package builds cleanly per target architecture
- metadata/lint checks pass
- package installs on Debian Trixie and Ubuntu 26.04
- ezkvm runtime starts with packaged defaults on both distros
- config and runtime path contract is unchanged or explicitly documented
- any compatibility exceptions are approved and documented

## Example Prompts

- `/cross-distro-deb-packaging validate this debian/control for a single package that runs on Debian Trixie and Ubuntu 26.04.`
- `/cross-distro-deb-packaging propose Depends/Recommends split for optional networking helpers without distro-specific forks.`
- `/cross-distro-deb-packaging generate a dual-distro packaging verification checklist for this release.`
