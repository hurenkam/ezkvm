# Code-Backed Schema Examples

These minimal examples are derived from active serde-backed shapes in the implementation.

## Example 1: CPU Features

CPU model with explicit feature flags:

```yaml
system:
  cpu:
    model: "host"
    vcpus: 4
    features:
      - "hv_time"
      - "+kvm_pv_unhalt"
```

## Example 2: Hugepages Memory Backend

```yaml
system:
  memory:
    size: 32768
    hugepages:
      enabled: true
      size_kib: 1048576
      mem_path: /run/hugepages/kvm/1048576kB
```

## Example 3: Display Device

```yaml
devices:
  displays:
    - type: virtio-gpu
      vram: 256
```

## Example 4: SPICE Endpoint

Secure SPICE endpoint with GL acceleration:

```yaml
spice:
  enabled: true
  addr: "127.0.0.1"
  port: 5900
  disable_ticketing: false
  audio: true
  vdagent: true
```

## Example 5: Network Bridge Backend

Optimized network interface:

```yaml
devices:
  networks:
    - model: virtio-net
      backend:
        type: bridge
        bridge: vmbr0
      mac: "BC:24:11:FF:76:89"
```

## Example 6: SCSI Controller Plus Drive

```yaml
controllers:
  scsi:
    - id: scsi0
      type: virtio-scsi-pci

devices:
  drives:
    - id: root
      path: /dev/vm1/root
      interface: scsi
      type: disk
      format: raw
      controller: scsi0
      scsi_id: 0
```

## Example 7: PCI and USB Passthrough

```yaml
host:
  pci:
    - id: hostpci0
      device: "0000:01:00.0"
      pcie: true
      x_vga: true
      multifunction: true
  usb:
    - id: usb0
      host: "1-7.6"
```

## Example 8: TPM Emulator

Software TPM backed by persistent file:

```yaml
system:
  tpm:
    backend: emulator
    version: "2.0"
    model: tpm-tis
    state_path: /var/run/ezkvm/vm-301.swtpm
    state_backend_uri: /var/lib/ezkvm/vm-301-tpmstate
```

Notes:

- For regular-file backends like `/var/lib/ezkvm/vm-301-tpmstate`, ezkvm appends
  `,mode=0600` by default when building swtpm `--tpmstate backend-uri=...`.
- For local device-node backends like `/dev/vm1/vm-301-tpmstate`, ezkvm does not
  append a default mode, which avoids unprivileged mode-change failures.

## See Also

- [System and Boot](system-and-boot.md)
- [Devices, Controllers, and Host Passthrough](devices.md)
- [Platform Features and Options](platform-features.md)
