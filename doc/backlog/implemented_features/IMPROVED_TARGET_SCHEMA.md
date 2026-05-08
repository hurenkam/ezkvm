# Improved Target Schema

This document defines the target VM schema layout and migration semantics for the VM Config Schema Restructure effort.

## Canonical Target Layout

```yaml
name: "my-vm"
backend: "qemu"
profiles: []

system:
  architecture: "x86_64"
  machine: "q35"

  cpu:
    model: "host"
    vcpus: 8
    features:
      - "hv_time"
      - "+kvm_pv_unhalt"
    numa: []

  memory:
    size: 16384
    ballooning:
      enabled: true
      model: "virtio-balloon-pci"
    ivshmem:
      enabled: false

  boot: {}
  tpm: {}
  smbios: {}

controllers:
  scsi: []
  xhci: []

host:
  pci: []
  usb: []

devices:
  drives: []
  networks: []
  displays: []
  serials: []
  input: []
  audio: []

options:
  guest_agent: {}
  qmp: {}
```

## Legacy To Target Path Mapping

- `boot` -> `system.boot`
- `tpm` -> `system.tpm`
- `smbios` -> `system.smbios`
- `guest_agent` -> `options.guest_agent`
- `qmp` -> `options.qmp`
- `system.cpu_model` -> `system.cpu.model`
- `system.vcpus` -> `system.cpu.vcpus`
- `system.cpu_features[].name` -> `system.cpu.features[]`
- `numa` -> `system.cpu.numa`
- `system.memory` (scalar) -> `system.memory.size`
- `ballooning` -> `system.memory.ballooning`
- `ivshmem` -> `system.memory.ivshmem`
- `scsi_controllers` -> `controllers.scsi`
- `xhci_controllers` -> `controllers.xhci`
- `hostpci` -> `host.pci`
- `usb_devices` -> `host.usb`
- `input_devices` -> `devices.input`
- `audio_devices` -> `devices.audio`

## Normalization And Precedence Rules

1. Build merged YAML using existing profile merge behavior first.
2. Normalize merged YAML into canonical target paths before validation.
3. If both legacy and target paths are present, target path wins for scalar/object fields.
4. For moved list fields, canonical list is built by concatenating legacy then target entries, preserving declaration order from the merged document.
5. For `system.cpu.features`, accept both source forms during normalization:
   - legacy object entries (`{ name: "..." }`)
   - target string entries (`"..."`)
   Normalize to canonical `Vec<String>`.
6. For memory, support both legacy scalar and target object input:
   - scalar only -> `system.memory.size = scalar`
   - object only -> use object as-is
   - both provided -> `system.memory.size` from target object wins
7. Validation and QEMU composition consume canonical target paths only.

## Merge Semantics At Canonical Paths

- Scalars: replace (last writer wins).
- Objects/maps: deep merge.
- Lists: keep current path-aware strategy after remapping path names:
  - append-in-order families remain append-in-order
  - id-merge families remain id-merge
  - append-unique families remain append-unique
  - all others remain full-replace

## Scope Note

- This design is migration-safe and keeps backward compatibility while enabling future removal of legacy aliases in a later deprecation cycle.
