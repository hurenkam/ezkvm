# VM Structure

## Minimal Shape

```yaml
name: "my-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  cpu:
    model: "host"
    vcpus: 2
  memory:
    size: 2048

options:
  enable_kvm: true
  daemonize: false
```

## Top-Level Keys

- `name` (required)
- `backend` (required, currently `qemu`)
- `profiles` (optional)
- `system` (required)
- `devices` (optional)
- `controllers` (optional)
- `host` (optional)
- `spice` (optional)
- `hyperv` (optional)
- `iscsi_disks` (optional)
- `options` (required fields: `enable_kvm`, `daemonize`)

## Canonical Mapping

Preferred canonical locations:

- `system.boot`
- `system.tpm`
- `system.smbios`
- `system.cpu.model`
- `system.cpu.vcpus`
- `system.cpu.features`
- `system.cpu.numa`
- `system.memory.size`
- `system.memory.ballooning`
- `system.memory.ivshmem`
- `options.guest_agent`
- `options.qmp`
- `controllers.scsi`
- `controllers.xhci`
- `host.pci`
- `host.usb`
- `devices.input`
- `devices.audio`

## Canonical-Only Schema

Use canonical keys only. Legacy aliases and legacy top-level moved sections are not part of the supported user schema.

## See also

- [Central config](central-config.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Platform features and options](platform-features.md)
- [Examples](examples.md)