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

Implicit placement semantics are part of this contract:

- When `bus` is omitted for a PCI device on Q35, effective default placement resolves via QEMU's default bus lookup and typically lands on `pcie.0`.
- When `addr` is omitted, effective default is QEMU auto-assignment, selecting the first free non-reserved slot at function `0` on the selected bus.
- `addr=<slot>` implies function `0`; `addr=<slot>.<fn>` sets an explicit function.
- Import and replay paths must preserve whether placement was explicit or implicit.

Placement precedence for import and command-build paths:

- Explicit `bus`/`addr` in effective config
- Imported explicit `bus`/`addr` from source command/config
- ezkvm placement policy defaults
- QEMU implicit defaults (runtime fallback)

ezkvm shall follow proxmox policies with respect to bus assignments for the following devices, and only deviate from this if explicitly instructed to do so by the config file:

- virtio-vga-gl -> preferred bus: pcie.0, preferred address: 0x01+
- ivshmem-plain -> preferred bus: pcie.0, preferred address: 0x08+
- virtio-balloon-pci -> preferred bus: pci.0, preferred address: 0x03
- pvscsi -> preferred bus: pci.0, preferred address: 0x05+
- virtio-serial -> preferred bus: pci.0, preferred address: 0x08+
- virtio-net-pci -> preferred bus: pci.0, preferred address: 0x12+
- qemu-xhci -> preferred bus: pci.1, preferred address: 0x1b+
- ich9-intel-hda -> preferred bus: pci.2, preferred address: 0x0c+
- virtio-scsi-pci -> preferred bus: pci.3, preferred address: 0x01+
- vfio-pci -> preferred bus: ich9-pcie-port-1..8, preferred address: 0x0.0+

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
- Clear expectation that preserving implicit placement state requires deterministic command ordering and stable bridge topology.

Negative:

- Less freedom to simplify topology without migration notes.
- Some advanced topologies require explicit bus/IO budget planning.

## Related ADRs

- [ADR-0002: Import Normalization Contract](ADR-0002-import-normalization-contract.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md)

## References

- QEMU Q35 implementation notes and PCIe guidelines (`docs/pcie.txt`, `hw/i386/pc_q35.c`, `hw/pci-host/q35.c` in upstream QEMU)
- Proxmox Q35 templates (`/usr/share/qemu-server/pve-q35.cfg`, `/usr/share/qemu-server/pve-q35-4.0.cfg`)
- QEMU bus/address assignment analysis (`doc/dev/domain-knowledge/qemu-bus-and-addr-assignment.md`)
- Local Q35 overview reference (`/home/hurenkam/Downloads/Q35.pdf`)