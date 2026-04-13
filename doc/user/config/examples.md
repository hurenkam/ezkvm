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
  - "windows_11_base"
  - "proxmox_q35_topology"
  - "spice_looking_glass"
  - "gpu_passthrough"
  - "storage_network_defaults"

system:
  memory:
    size: 16384
  cpu:
    vcpus: 8
  boot:
    uefi_vars: "/dev/vm1/vm-108-efidisk"

options:
  guest_agent:
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

## See also

- [VM structure](vm-structure.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Platform features and options](platform-features.md)