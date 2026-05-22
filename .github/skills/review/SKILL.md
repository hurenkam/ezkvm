---
name: review
description: 'Use when reviewing Proxmox configuration files and QEMU command lines against ezkvm support, identifying gaps, or planning implementation work.'
---

# review

**Scope**: Workspace skill for analyzing Proxmox configuration files and QEMU commands to review ezkvm implementation coverage.

## Description

This skill captures a systematic review methodology for comparing Proxmox-generated QEMU commands and configuration files against ezkvm's YAML schema and command generation. Use it when evaluating ezkvm's feature completeness, identifying gaps, and planning implementation improvements based on real Proxmox configurations.

## Use when

- analyzing Proxmox .conf and .cmd files to understand VM configurations
- comparing Proxmox QEMU command generation with ezkvm equivalents
- identifying missing features or configuration options in ezkvm
- reviewing ezkvm's QEMU argument generation for completeness
- planning feature additions based on real-world Proxmox usage
- validating ezkvm configuration schema against Proxmox capabilities

## Workflow

1. **Gather input files**
   - Locate Proxmox .conf files (<vmid>.conf) containing VM definitions
   - Find corresponding .cmd files with generated QEMU commands
   - Identify any existing ezkvm YAML files and generated commands for comparison
   - Ensure files are from the same VM for accurate comparison

2. **Analyze Proxmox configuration**
   - Parse .conf file format (key=value pairs, sections)
   - Identify all configured options: system, devices, boot, options
   - Note Proxmox-specific settings and their QEMU translations
   - Document storage, network, and device configurations

3. **Analyze generated QEMU commands**
   - Parse the complete QEMU command line from .cmd file
   - Categorize arguments: system, CPU, memory, devices, networking, etc.
   - Identify QEMU-specific options and their purposes
   - Note any Proxmox-specific modifications or optimizations

4. **Compare with ezkvm capabilities**
   - Map Proxmox configuration options to ezkvm YAML schema
   - Check which QEMU arguments ezkvm currently generates
   - Identify missing configuration options or QEMU arguments
   - Evaluate differences in approach (declarative YAML vs imperative config)

5. **Generate review outputs**
   - Create REVIEW_COMMENTS.md with detailed findings and comparisons
   - Produce TODO.md with prioritized implementation plan
   - Include code examples and configuration snippets
   - Suggest specific changes to ezkvm modules

## Decision points

- **Configuration completeness**: Does ezkvm support all Proxmox configuration options?
- **QEMU argument coverage**: Are all generated QEMU arguments supported by ezkvm?
- **Schema design**: Should ezkvm YAML mirror Proxmox .conf format or be more user-friendly?
- **Implementation priority**: Which missing features are most critical for compatibility?
- **Backwards compatibility**: How to add new options without breaking existing configs?

## Quality criteria

- comprehensive analysis of all input files and their relationships
- accurate mapping between Proxmox and ezkvm configuration concepts
- clear identification of gaps with specific examples
- actionable TODO items with implementation details
- review comments include both technical findings and user impact
- suggestions consider ezkvm's design philosophy and constraints

## Validation Lifecycle Reporting

For refactor or behavior-changing Rust reviews, include validation lifecycle results:

- Intake baseline (before edits):
   - cargo fmt --all --check
   - cargo clippy --all-targets --all-features -- -D warnings
   - cargo test --quiet
- Exit validation (after edits): same commands
- Delta summary: newly failing, newly fixed, unchanged failing

Do not mark review completion if exit results regress versus intake baseline.

## Unresolved Failure Reporting

When any failure remains unresolved, add an explicit section:

- Identifier: failing test/check name
- Reproduce: exact command
- Suspected first bad commit: sha or unknown
- Unrelated rationale: short justification when classified unrelated
- User decision: fix now or approved deferral
- Tracking reference: backlog/tracking ticket or row

## Proxmox Configuration Analysis

### Common .conf File Sections
- **System settings**: cores, memory, cpu type, machine type
- **Boot settings**: boot order, firmware, kernel/initrd options
- **Device mappings**: disks, network interfaces, USB devices
- **Advanced options**: numa, hugepages, iothread settings

### QEMU Command Categories
- **Core VM**: -machine, -cpu, -m, -smp
- **Storage**: -drive, -device virtio-blk/scsi, -cdrom
- **Networking**: -netdev, -device virtio-net/e1000
- **Display**: -vga, -device virtio-gpu
- **Other**: -boot, -serial, -monitor, -qmp

### Topology Review Checklist (Q35)
- **PCIe/PCI hierarchy correctness**: PCIe devices in PCIe hierarchies, legacy PCI devices behind legacy bridges
- **IO and bus budget risk**: bridge/switch additions justified against IO-space and bus-number limits
- **Hotplug model correctness**: native PCIe hotplug paths are used for PCIe devices; bridge-based paths used for legacy PCI devices
- **Guest identity stability**: imported sensitive devices preserve guest-visible `bus/addr` identity unless migration notes explicitly allow change

### ezkvm Mapping Considerations
- YAML structure should remain human-readable
- Support Proxmox naming conventions where possible
- Provide defaults that match Proxmox behavior
- Allow advanced QEMU options through extension mechanisms

## Example prompts

- `Use the review skill to analyze the Proxmox config files in the input directory and compare with ezkvm capabilities.`
- `Use the review skill to identify missing QEMU arguments in ezkvm's command generation.`
- `Use the review skill to create a feature roadmap based on real Proxmox VM configurations.`
- `Use the review skill to document differences between Proxmox and ezkvm configuration approaches.`

## Next customization ideas

- Add a workspace instruction for Proxmox configuration file parsing
- Create a prompt template for configuration comparison tasks (`config-review.prompt.md`)
- Add `copilot-instructions.md` for systematic code review and feature gap analysis
