---
name: debian-trixie
description: 'Use when working on Debian Trixie packaging, architecture compatibility, Debian tooling, or distro-specific build and integration guidance.'
---

# debian-trixie

**Scope**: Workspace skill for Debian Trixie packaging, architecture, and distro-specific workflow support.

## Description

This skill captures Debian Trixie expertise for package building, architecture compatibility, tooling, and distribution-specific design decisions. Use it when working on code changes, packaging guidance, or system integration that depends on Debian Trixie conventions.

## Use when

- designing or building Debian packages for Trixie
- choosing Debian architecture support and ABI compatibility
- integrating with Debian tooling like `dpkg`, `apt`, `debhelper`, or `pbuilder`
- documenting packaging and distribution expectations for Trixie
- validating package dependencies, policy, and repository layout
- creating distro-specific examples or automation scripts

## Workflow

1. **Understand the target architecture**
   - Identify whether the package is intended for `amd64`, `arm64`, `i386`, `riscv64`, or another Debian Trixie-supported architecture
   - Confirm ABI requirements and any architecture-specific build options
   - Check whether the package is architecture-specific or `all`

2. **Review Debian packaging context**
   - Look for existing `debian/` metadata or packaging scripts
   - Determine whether the workflow is building a source package, binary package, or repository metadata
   - Identify dependencies and compatibility constraints from `control` and `rules`

3. **Implement packaging changes**
   - Use Debian policy-compliant packaging conventions and recommended helpers
   - Prefer reproducible builds and explicit dependency declarations
   - Keep the package layout simple and aligned with Debian filesystem hierarchy standards
   - Validate maintainer scripts and `changelog` entries for Trixie compatibility

4. **Validate package build**
   - Build in a clean environment using `sbuild`, `pbuilder`, or `docker` if available
   - Test installability with `dpkg -i` and `apt-get -f install`
   - Ensure correct package metadata, dependencies, and architecture tags
   - Verify runtime behavior on the target Debian Trixie environment where possible

5. **Document and finalize**
   - Update README or packaging notes with Trixie-specific guidance
   - Include architecture compatibility, dependency versions, and build instructions
   - Add examples for `debuild`, `dpkg-buildpackage`, or repo upload steps
   - Capture any distro-specific limitations or known issues

## Decision points

- **Package type**: source-only vs binary package vs architecture-independent package
- **Build environment**: native, containerized, or chroot-based build using Debian tools
- **Dependency handling**: runtime vs build dependencies and `Depends` vs `Build-Depends`
- **Policy compliance**: follow Debian policy for file placement, scripts, and versioning
- **Repository format**: `deb`, `dsc`, and support for Debian archive layout

## Quality criteria

- package metadata is clean, explicit, and Trixie-compatible
- build commands and scripts use Debian packaging best practices
- package installs cleanly and lists correct architecture metadata
- produced artifacts are suitable for Debian repository workflow
- documentation includes package build instructions and dependency notes
- any distro-specific assumptions are stated clearly

## Example prompts

- `Use the debian-trixie skill to add Debian packaging guidance for building this tool on Trixie.`
- `Use the debian-trixie skill to document architecture restrictions and build directives for Debian packaging.`
- `Use the debian-trixie skill to recommend Trixie-compatible packaging tools and validation steps.`
- `Use the debian-trixie skill to create a packaging checklist for Debian Trixie builds.`

## Next customization ideas

- Add a workspace instruction for Debian packaging conventions and `debian/` layout
- Create a prompt template for Debian package build requests (`debian-trixie-package.prompt.md`)
- Add a `copilot-instructions.md` for Debian-specific build and packaging standards
