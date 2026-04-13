# VM Configuration Docs

This directory contains the user-facing configuration reference for ezkvm.

## Reading Order

1. [vm-structure.md](vm-structure.md)
2. [central-config.md](central-config.md)
3. [profiles-and-merge.md](profiles-and-merge.md)
4. [system-and-boot.md](system-and-boot.md)
5. [devices.md](devices.md)
6. [platform-features.md](platform-features.md)
7. [examples.md](examples.md)

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

Legacy aliases are accepted during migration and normalized before validation.
If both legacy and canonical scalar/object paths are present, canonical values win.
For moved list families, final order is legacy entries first, then canonical entries.

## See also

- [VM structure](vm-structure.md)
- [Central config](central-config.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Platform features and options](platform-features.md)
- [Examples](examples.md)