# Examples

## Minimal Canonical VM

```yaml
name: "basic-ubuntu"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  cpu:
    model: "host"
    vcpus: 2
  memory:
    size: 2048
  boot:
    firmware: "uefi"
    boot_order: ["disk", "cdrom"]

devices:
  drives:
    - id: "root"
      path: "/var/lib/ezkvm/ubuntu-22.04.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"

options:
  enable_kvm: true
  daemonize: false
```

## Canonical Profile-Based Layout

```yaml
name: "wakiza"
backend: "qemu"
profiles:
  - "proxmox-q35-uefi"
  - "windows-common"
  - "windows-11"
  - "looking-glass"
  - "gpu-passthrough"
  - "storage-virtio-scsi-pci"

system:
  memory:
    size: 16384
  cpu:
    vcpus: 8
  boot:
    uefi_vars: "/dev/vm1/vm-108-efidisk"

options:
  guest_agent:
    # Explicit Proxmox-style override; if omitted, default is <runtime_root>/<vm-name>.qga
    socket_path: "/var/run/qemu-server/108.qga"
  qmp:
    enabled: true
    socket_path: "/var/run/qemu-server/108.qmp"
  pid_file: "/tmp/ezkvm/wakiza.pid"
  log_dir: "/tmp/ezkvm/logs"
  log_keep: 5
```

## Environment Variables

```yaml
name: "test-env-vars"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory:
    size: ${TEST_MEMORY}
  cpu:
    model: "host"
    vcpus: 2

options:
  enable_kvm: true
  daemonize: false
```

Both `${VAR}` and `$VAR` are supported.

## Desktop with SPICE

```yaml
name: "ubuntu-desktop"
backend: "qemu"
profiles:
  - "proxmox-q35-uefi"
  - "remote-viewer-spice"

system:
  architecture: "x86_64"
  machine: "q35"
  cpu:
    model: "host"
    vcpus: 8
  memory:
    size: 16384

devices:
  displays:
    - type: "virtio-gpu"
      vram: 256
  drives:
    - path: "/dev/vm1/ubuntu-root"
      interface: "virtio"
      type: "disk"
      format: "raw"
      boot_index: 0
  networks:
    - model: "virtio-net"
      backend:
        type: "bridge"
        bridge: "vmbr0"

spice:
  enabled: true
  addr: "127.0.0.1"
  port: 5900
  disable_ticketing: false
  audio: true
  vdagent: true

options:
  enable_kvm: true
  daemonize: false
```

## GPU Passthrough Workstation

```yaml
name: "gaming-01"
backend: "qemu"
profiles:
  - "proxmox-q35-uefi"
  - "gpu-passthrough"

system:
  architecture: "x86_64"
  machine: "q35"
  cpu:
    model: "host"
    vcpus: 12
  memory:
    size: 32768

devices:
  drives:
    - path: "/dev/vm3/gaming-disk0"
      interface: "virtio"
      type: "disk"
      format: "raw"
      boot_index: 0
  displays:
    - type: "none"

host:
  pci:
    - device: "0000:01:00.0"
      x_vga: true
      pcie: true
    - device: "0000:01:00.1"
  usb:
    - host: "1-7.6"
    - host: "0451:16a0"

options:
  enable_kvm: true
  daemonize: false
```

## NUMA and Hugepages

```yaml
name: "numa-hpc"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  cpu:
    model: "host"
    vcpus: 24
    numa:
      - id: 0
        memory: 32768
        cpus: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
      - id: 1
        memory: 32768
        cpus: [12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23]
  memory:
    size: 65536
    hugepages:
      enabled: true
      size_kib: 1048576

devices:
  drives:
    - path: "/dev/vm4/root"
      interface: "scsi"
      type: "disk"
      format: "raw"
      boot_index: 0

controllers:
  scsi:
    - type: "virtio-scsi-pci"

options:
  enable_kvm: true
  daemonize: false
```

## Controller-Owned Storage Layout

```yaml
name: "storage-layout"
backend: "qemu"

devices:
  drives:
    - path: ""
      interface: "ide"
      type: "cdrom"
      format: "raw"
  controllers:
    scsi:
      - id: "scsihw0"
        type: "virtio-scsi-pci"
        drives:
          - path: "/dev/vm5/root"
            type: "disk"
            format: "raw"
            scsi_id: 0
            boot_index: 0
          - path: "/dev/vm5/data"
            type: "disk"
            format: "raw"
            scsi_id: 1

options:
  enable_kvm: true
  daemonize: false
```

## See Also

- [VM Structure](vm-structure.md)
- [Profiles and Merge](profiles-and-merge.md)
- [System and Boot](system-and-boot.md)
- [Devices, Controllers, and Host Passthrough](devices.md)
- [Platform Features and Options](platform-features.md)
- [Code-Backed Schema Examples](code-backed-shapes.md)