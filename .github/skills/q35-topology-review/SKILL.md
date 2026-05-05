---
name: q35-topology-review
description: "Use when reviewing or planning Q35 PCIe and legacy PCI hierarchy, bridge topology, hotplug paths, and guest-visible slot stability in ezkvm and Proxmox-imported VMs."
argument-hint: "Describe the topology change or import mapping you want reviewed (for example bus placement, bridge depth, hotplug support, or slot stability)."
---

# q35-topology-review

Scope: Focused skill for Q35 topology correctness and compatibility review.

## Use when

- reviewing bus and address placement for imported Proxmox VMs
- checking PCIe versus legacy PCI hierarchy correctness
- evaluating hotplug behavior for Q35 paths
- validating bridge and bus-number growth risks
- assessing whether device slot identity drift can break guest behavior

## Core Rules

1. Keep PCIe and legacy PCI hierarchies separated.
- PCIe devices should be placed behind pcie-root-port or PCIe downstream ports.
- Legacy PCI devices should be placed behind pcie-pci-bridge and pci-bridge chains.

2. Keep hierarchy flat by default.
- Prefer root-port fanout over deep switch trees unless bus budget constraints require otherwise.

3. Preserve guest-visible slot identity for imported sensitive devices.
- Treat NIC, GPU, and guest-agent-related bus and addr changes as compatibility risks unless migration rationale is explicit.

4. Validate topology budgets when bridge depth changes.
- Confirm IO-space and bus-number impact is intentional and documented.

5. Verify parity against Proxmox references for import-related changes.
- Use dry-run output and fixture comparisons to prevent topology regressions.

## Review Checklist

- PCIe and legacy PCI separation still holds
- Bridge and switch additions are justified
- Hotplug model matches device class and path
- Imported slot identity is stable or explicitly migration-documented
- Dry-run parity and fixture updates cover topology changes

## Output Expectations

1. Findings ordered by severity with file references.
2. Open questions or assumptions.
3. Residual risk summary for topology compatibility.
