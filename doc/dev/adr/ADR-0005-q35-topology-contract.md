# ADR-0005: Q35 Topology Contract

**Date:** 2026-05-05
**Status:** Accepted
**Context:** Proxmox-imported Q35 VMs rely on stable PCIe/PCI topology and slot identity for runtime compatibility.

## Question

How should ezkvm model `q35` PCIe/PCI hierarchy for imported VMs so guest behavior stays compatible with Proxmox while still remaining maintainable?

## Decision

ezkvm adopts a Q35 topology contract:

- Use a PCIe-first hierarchy for PCIe-capable devices.
- Keep legacy PCI devices in dedicated legacy PCI islands through `pcie-pci-bridge` and `pci-bridge`.
- Prefer flatter hierarchies over deep switch trees unless bus-budget constraints require extra layers.
- Treat imported guest-visible slot identity as a compatibility requirement for sensitive devices (for example NIC/GPU/guest-agent-related placement).
- Require dry-run parity checks against captured Proxmox command lines for import topology-affecting changes.

## Rationale

- Q35 in QEMU is PCIe-centric and has practical IO/bus-number constraints that become significant when bridge depth grows.
- Proxmox templates intentionally pre-wire Q35 bridge topology and fixed placements for compatibility with guest expectations.
- Guest operating systems, especially Windows, can treat PCI slot changes as new hardware, causing network and integration regressions.
- A documented contract reduces accidental topology churn and makes review criteria explicit.

## Consequences

Positive:

- More predictable imported VM behavior and fewer regressions from accidental re-slotting.
- Clear review and testing criteria for topology-related changes.
- Better alignment between architecture docs, import instructions, and review automation.

Negative:

- Less freedom to simplify topology without migration notes.
- Some advanced topologies require explicit bus/IO budget planning.

## Related ADRs

- [ADR-0002: Import Normalization Contract](ADR-0002-import-normalization-contract.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md)

## References

- QEMU Q35 implementation notes and PCIe guidelines (`docs/pcie.txt`, `hw/i386/pc_q35.c`, `hw/pci-host/q35.c` in upstream QEMU)
- Proxmox Q35 templates (`/usr/share/qemu-server/pve-q35.cfg`, `/usr/share/qemu-server/pve-q35-4.0.cfg`)
- Local Q35 overview reference (`/home/hurenkam/Downloads/Q35.pdf`)