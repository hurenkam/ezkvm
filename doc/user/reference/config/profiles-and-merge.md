# Profiles and Merge

## profiles

`profiles` is an ordered list. Merge order is:

1. empty base
2. profiles in declared order
3. VM file values

## policies

`policies` applies defaults after merge and before validation.

Supported policy families:

- `policies.drives`
- `policies.networks`
- `policies.displays`
- `policies.serials`
- `policies.hostpci`
- `policies.usb_devices`
- `policies.xhci_controllers`
- `policies.audio_devices`
- `policies.scsi_controllers`
- `policies.iscsi_disks`

Each entry supports:

- `match`
- `defaults`
- optional `placement` (`drives.scsi_id`, `networks.addr`)

## Merge Semantics

- Scalars: replace (last writer wins)
- Maps: deep merge
- Lists: path-aware strategy

Path-aware list behavior:

- id-merge lists: `host.pci`, `host.usb`, `controllers.scsi`, `controllers.xhci`, `devices.audio`
- append-all lists: `devices.drives`, `devices.networks`, and `policies.*` families
- append-unique lists: `system.cpu.features`, `system.machine_options`, `options.global_options`
- all others: replace

## Notes

- Configure networks with `devices.networks[].backend`.

## See also

- [VM structure](vm-structure.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Platform features and options](platform-features.md)
- [Examples](../../how-to/config/examples.md)