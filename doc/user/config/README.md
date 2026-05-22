# VM Configuration Docs

This directory contains the user-facing configuration reference for ezkvm.

## Reading Order

**Foundation:**
1. [vm-structure.md](vm-structure.md) — Top-level schema overview and canonical keys
2. [central-config.md](central-config.md) — Central configuration and runtime location defaults

**Core Configuration:**
3. [profiles-and-merge.md](profiles-and-merge.md) — Profile layering and merge semantics
4. [system-and-boot.md](system-and-boot.md) — System/CPU/memory/boot configuration
5. [devices.md](devices.md) — Devices, controllers, and host passthrough
6. [platform-features.md](platform-features.md) — Platform options, hyperv, and special features

**Reference and Examples:**
7. [examples.md](examples.md) — Practical end-to-end configuration examples
8. [code-backed-shapes.md](code-backed-shapes.md) — Minimal schema patterns from implementation
9. [field-reference.md](field-reference.md) — Complete field tables (legacy schema reference)

**Special Topics:**
10. [import-proxmox.md](import-proxmox.md) — Proxmox import plus portable runtime operator guidance
11. [import-qemu-cmd.md](import-qemu-cmd.md) — QEMU command import workflow, warnings, and validation
12. [troubleshooting.md](troubleshooting.md) — Common issues and diagnostic checks

## Canonical First

Use canonical paths for new configs:

- `system.cpu.*`
- `system.memory.size`
- `system.memory.ballooning`
- `system.memory.ivshmem`
- `system.boot`
- `system.tpm`
- `system.smbios`
- `options.guest_agent`
- `options.qmp`
- `controllers.scsi`
- `controllers.xhci`
- `host.pci`
- `host.usb`
- `devices.input`
- `devices.audio`

Do not use legacy top-level sections (`general`, `gpu`, `display`, `storage`, `network`, `extras`). See [field-reference.md](field-reference.md) for historical documentation.

## Quick Links

- **Getting Started**: Start with [vm-structure.md](vm-structure.md)
- **Practical Examples**: See [examples.md](examples.md)
- **Profiles**: Learn about layering in [profiles-and-merge.md](profiles-and-merge.md)
- **Hardware Passthrough**: See [devices.md](devices.md)
- **Migrating from Proxmox**: Check [import-proxmox.md](import-proxmox.md)
- **Importing Captured QEMU Commands**: Check [import-qemu-cmd.md](import-qemu-cmd.md)
- **Debugging Issues**: Use [troubleshooting.md](troubleshooting.md)
- **Field Reference**: See [field-reference.md](field-reference.md) for complete tables

## Document Structure

Each document focuses on a specific area:

| Document | Focus |
| --- | --- |
| [vm-structure.md](vm-structure.md) | Top-level keys and required fields |
| [central-config.md](central-config.md) | `EZKVM_CONFIG`, shared locations, and `host_capabilities` |
| [profiles-and-merge.md](profiles-and-merge.md) | Profile stacking and merge rules |
| [system-and-boot.md](system-and-boot.md) | CPU, memory, boot, TPM, NUMA |
| [devices.md](devices.md) | Drives, networks, controllers, passthrough |
| [platform-features.md](platform-features.md) | Hyper-V, IOMMU, options, QMP, guest agent |
| [examples.md](examples.md) | Real-world scenarios (desktop, headless, GPU, NUMA) |
| [code-backed-shapes.md](code-backed-shapes.md) | Schema examples from serde implementation |
| [field-reference.md](field-reference.md) | Legacy field tables and auto-ID rules |
| [import-proxmox.md](import-proxmox.md) | Proxmox config mapping and checklist |
| [import-qemu-cmd.md](import-qemu-cmd.md) | QEMU command import workflow and warning model |
| [troubleshooting.md](troubleshooting.md) | Memory, display, boot, PCI, USB issues |