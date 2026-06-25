# ezkvm YAML Export Format

`ezkvm convert --input.type proxmox --output.type ezkvm` emits valid YAML that favors compact flow style for simple nested values.

## What Is Compact

Simple nested maps and short lists are emitted inline when possible.

Example:

```yaml
host:
  spice: { port: 5900, listen: 0.0.0.0, disable_ticketing: true }
  resources:
    - { id: hostpci0, pcie: { address: 0000:0e:11.6, rombar: false } }
  
virtual_machine:
  machine: { family: pc, chipset: q35 }
  devices:
    - pcie: { bus: 0, device: 0, function: 0, type: pv_scsi }
    - pcie: { bus: 0, device: 0, function: 1, type: passthrough, resource: hostpci0 }
```

For PCIe passthrough, host-literal details (host PCIe address and passthrough options) are emitted in `host.resources[].pcie`, while `virtual_machine.devices[].pcie` carries a stable `resource` reference.

## What Stays Expanded

Values that are complex or deeply nested stay in regular block style.

## Compatibility

The exported file remains standard YAML and is equivalent in meaning to expanded block formatting.
