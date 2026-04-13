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

## Legacy Compatibility

Legacy keys are accepted and normalized.

- Scalar/object conflicts: canonical path wins.
- Moved list conflicts: merged as legacy entries first, then canonical entries.

Use canonical keys for all new configs.

Legacy to canonical examples:

- `boot` -> `system.boot`
- `tpm` -> `system.tpm`
- `smbios` -> `system.smbios`
- `guest_agent` -> `options.guest_agent`
- `qmp` -> `options.qmp`
- `system.vcpus` -> `system.cpu.vcpus`
- `system.cpu_model` -> `system.cpu.model`
- `system.cpu_features` -> `system.cpu.features`
- `numa` -> `system.cpu.numa`
- `system.memory` -> `system.memory.size`
- `ballooning` -> `system.memory.ballooning`
- `ivshmem` -> `system.memory.ivshmem`
- `scsi_controllers` -> `controllers.scsi`
- `xhci_controllers` -> `controllers.xhci`
- `hostpci` -> `host.pci`
- `usb_devices` -> `host.usb`
- `input_devices` -> `devices.input`
- `audio_devices` -> `devices.audio`

## See also

- [Central config](central-config.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Platform features and options](platform-features.md)
- [Examples](examples.md)