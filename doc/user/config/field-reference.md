# Field Reference

This document provides comprehensive field tables for all top-level configuration sections.

## Configuration Search Paths

Configuration files are YAML and are searched in this order:

1. current directory
2. `~/.ezkvm`
3. `/etc/ezkvm`

## Top-Level Schema (Compatibility Reference)

**Note**: This section documents historical and compatibility-oriented shapes collected from earlier configuration docs. For new configurations, use the canonical schema documented in [vm-structure.md](vm-structure.md), [system-and-boot.md](system-and-boot.md), [devices.md](devices.md), and [platform-features.md](platform-features.md).

Legacy top-level sections:
- `general`
- `profiles`
- `system`
- `gpu`
- `display`
- `spice`
- `vnc`
- `host`
- `storage`
- `network`
- `extras`

### Legacy Top-Level Fields

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `general` | mapping | present | section | `general: { name: demo }` |
| `profiles` | sequence of strings | empty | existing profile names in the configured profile directory | `profiles: [ "proxmox-q35-uefi", "windows-11" ]` |
| `system` | mapping | present | section | `system: {}` |
| `gpu` | mapping | `type: no_gpu` | section | `gpu: { type: virtio }` |
| `display` | mapping | `type: no_display` | section | `display: { type: gtk }` |
| `spice` | mapping or null | null | section | `spice: { addr: 127.0.0.1, port: 5900 }` |
| `vnc` | mapping or null | null | section | `vnc: { addr: 127.0.0.1, port: 5901 }` |
| `host` | mapping or null | null | section | `host: { pci: [], usb: [] }` |
| `storage` | sequence | empty | controller entries | `storage: [ { controller: pvscsi } ]` |
| `network` | sequence | empty | NIC entries | `network: [ { type: bridge } ]` |
| `extras` | sequence of strings | empty | raw QEMU args | `extras: [ "-overcommit mem-lock=on" ]` |

## Legacy general

VM identity and control-plane sockets.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `name` | string | `anonymous` | any non-empty name | `name: ubuntu-dev` |
| `uuid` | string | none | UUID string | `uuid: c0e240a5-859a-4378-a2d9-95088f531142` |
| `monitor` | yes/no | `no` | `yes`, `no` | `monitor: yes` |
| `agent` | yes/no | `no` | `yes`, `no` | `agent: yes` |

## Legacy system

Core machine, firmware, CPU/memory, TPM, and optional devices.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `chipset` | mapping | `type: q35` | `q35`, `i440fx` | `chipset: { type: q35 }` |
| `bios` | mapping | `type: seabios` | `seabios`, `ovmf` | `bios: { type: ovmf, file: /dev/vm1/efi }` |
| `memory` | mapping | `max: 16384` | see memory table | `memory: { max: 8192, balloon: true }` |
| `cpu` | mapping | qemu64 profile | see CPU table | `cpu: { cores: 8, model: host }` |
| `tpm` | mapping | `type: no_tpm` | `no_tpm`, `swtpm`, `passthrough` | `tpm: { type: swtpm, disk: /dev/vm1/tpm, socket: /run/vm.tpm }` |
| `applesmc` | mapping or null | null | object with `osk` | `applesmc: { osk: your-key }` |
| `virtio_rng` | mapping or null | null | object with optional `filename` | `virtio_rng: { filename: /dev/urandom }` |
| `serial` | mapping or null | null | currently `type: socket` | `serial: { type: socket, path: /var/run/qemu-server/301.serial0 }` |
| `vmgenid` | string or null | null | GUID string | `vmgenid: b42d5b83-fee2-47dc-98a8-7856b18542ec` |
| `numa_nodes` | sequence | empty | node entries | see NUMA example |
| `numa_distances` | sequence | empty | distance entries | `- { src: 0, dst: 1, val: 20 }` |

### Legacy system.chipset

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `q35` | `q35`, `i440fx` | `type: q35` |
| `machine` (q35) | string | `q35` | any QEMU machine string | `machine: q35` |
| `pcie_root_ports` (q35) | sequence | empty | root-port objects | `pcie_root_ports: [ { id: rp0, chassis: 10, slot: 1, bus: pcie.0, addr: 0x2 } ]` |
| `xhci_enabled` (q35) | bool | `true` | `true`, `false` | `xhci_enabled: false` |
| `xhci_bus` (q35) | string | `pci.1` | bus name | `xhci_bus: pci.1` |
| `xhci_addr` (q35) | string | `0x1b` | hex-like slot | `xhci_addr: 0x1b` |

### Legacy system.bios

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `seabios` | `seabios`, `ovmf` | `type: ovmf` |
| `uuid` | string | empty/none | UUID | `uuid: c0e240a5-859a-4378-a2d9-95088f531142` |
| `file` (ovmf) | string | `NO_SETTINGS_FILE_PROVIDED` | path | `file: /dev/vm1/efi` |
| `arch` (ovmf) | string | `64bit` | `64bit`, `32bit` | `arch: 64bit` |
| `size` (ovmf) | string | `4M` | `2M`, `4M` | `size: 4M` |
| `secure_boot` (ovmf) | bool | `true` | `true`, `false` | `secure_boot: true` |
| `boot_menu` | bool | `true` | `true`, `false` | `boot_menu: true` |
| `boot_strict` | bool | `true` | `true`, `false` | `boot_strict: true` |
| `reboot_timeout` | integer | `1000` | non-negative integer | `reboot_timeout: 1000` |
| `boot_order` | string | none | QEMU boot order string | `boot_order: cd` |
| `boot_once` | string | none | QEMU one-shot boot string | `boot_once: c` |

### Legacy system.memory

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `max` | integer (MiB) | `16384` | positive integer | `max: 32768` |
| `balloon` | bool | unset | `true`, `false` | `balloon: true` |
| `hugepages` | bool | unset | `true`, `false` | `hugepages: true` |
| `prealloc` | bool | unset | `true`, `false` | `prealloc: true` |
| `mem_path` | string | unset | path | `mem_path: /run/hugepages/kvm/1048576kB` |

**NUMA backend behavior:**
- If `memory.hugepages: true` and `numa_nodes` is not empty, ezkvm emits per-node `memory-backend-file` objects and `-numa node,...,memdev=...`.
- Otherwise, ezkvm emits flat memory arguments such as `-m`, optional `-mem-path`, and optional `-mem-prealloc`.

### Legacy system.cpu

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `model` | string | `qemu64` | QEMU CPU model | `model: host` |
| `sockets` | integer | `1` | positive integer | `sockets: 1` |
| `dies` | integer | unset | positive integer | `dies: 2` |
| `clusters` | integer | unset | positive integer | `clusters: 2` |
| `cores` | integer | `4` | positive integer | `cores: 8` |
| `threads` | integer | `1` | positive integer | `threads: 2` |
| `flags` | string | `+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce` | CPU flag list | `flags: "+aes,enforce"` |

### Legacy system.tpm

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `no_tpm` | `no_tpm`, `swtpm`, `passthrough` | `type: swtpm` |
| `disk` (swtpm) | string | required for swtpm | path | `disk: /dev/vm1/vm-108-tpmstate` |
| `socket` (swtpm) | string | required for swtpm | path | `socket: /var/ezkvm/wakiza-tpm.socket` |

### Legacy system.serial

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | none | `socket` | `type: socket` |
| `path` | string | none | unix path | `path: /var/run/qemu-server/301.serial0` |

### Legacy system.numa_nodes and system.numa_distances

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `nodeid` | integer | inferred from index | non-negative integer | `nodeid: 0` |
| `cpus` | string | unset | CPU range/list | `cpus: "0-11"` |
| `mem` | integer (MiB) | unset | non-negative integer | `mem: 32768` |
| `host_nodes` | integer | unset | host NUMA node id | `host_nodes: 0` |
| `policy` | string | unset | `bind`, `preferred`, `interleave` | `policy: bind` |
| `src` | integer | none | node id | `src: 0` |
| `dst` | integer | none | node id | `dst: 1` |
| `val` | integer | none | distance value | `val: 20` |

## Legacy gpu

Selects guest GPU model or passthrough profile.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `no_gpu` | `no_gpu`, `passthrough`, `virtio`, `vmware-svga` | `type: virtio` |
| `pci` (passthrough) | sequence | empty | `Pci` objects | `pci: [ { vm_id: "0", host_id: "01:00" } ]` |
| `vga` (virtio) | yes/no | `yes` | `yes`, `no` | `vga: yes` |
| `gl` (virtio) | yes/no | `no` | `yes`, `no` | `gl: yes` |
| `pci` or `pcie` address (virtio) | mapping | optional | bus/address pair | `pcie: { bus: 0, address: 2 }` |
| `pci_address` (vmware-svga) | string | `0x1` | slot string | `pci_address: 0x2` |

## Legacy display

Selects local/remote presentation behavior.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `no_display` | `gtk`, `looking_glass`, `remote-viewer`, `no_display` | `type: remote-viewer` |
| `gl` (gtk) | bool | `true` | `true`, `false` | `gl: false` |
| `device` (looking_glass) | mapping | required | `path`, `size` | `device: { path: /dev/shm/looking-glass, size: 64M }` |
| `window` (looking_glass) | mapping | optional | `size`, `full_screen` | `window: { size: 1920x1080, full_screen: false }` |
| `input` (looking_glass) | mapping | optional | `grab_keyboard`, `escape_key` | `input: { grab_keyboard: true, escape_key: KEY_RIGHTCTRL }` |
| `auto_resize` (remote-viewer) | bool | `true` | `true`, `false` | `auto_resize: true` |
| `full_screen` (remote-viewer) | bool | `false` | `true`, `false` | `full_screen: false` |
| `usb_tablet` (remote-viewer) | bool | `false` | `true`, `false` | `usb_tablet: true` |
| `render_node` (remote-viewer) | string | optional | path | `render_node: /dev/dri/renderD128` |

When `display.type: remote-viewer` and `usb_tablet: true`, ezkvm adds `-device usb-tablet`.

## Legacy spice

SPICE transport, optional GL display path, and security options.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `addr` | string | `127.0.0.1` | IP/host | `addr: 127.0.0.1` |
| `port` | integer | `5900` | TCP port | `port: 5900` |
| `path` | string | unset | unix socket path | `path: /var/run/ezkvm/spice.sock` |
| `gl` | string | `off` | `off`, `on` | `gl: on` |
| `render_node` | string | unset | render node path | `render_node: /dev/dri/renderD128` |
| `disable_ticketing` | bool | `true` | `true`, `false` | `disable_ticketing: true` |
| `password` | string | unset | any string | `password: secret` |
| `tls_port` | integer | unset | TCP port | `tls_port: 61000` |
| `tls_ciphers` | string | unset | OpenSSL cipher string | `tls_ciphers: HIGH` |
| `x509_dir` | string | unset | path | `x509_dir: /etc/pki/qemu` |
| `seamless_migration` | bool | unset | `true`, `false` | `seamless_migration: true` |
| `websocket_port` | integer | unset | TCP port | `websocket_port: 6100` |

## Legacy vnc

VNC endpoint and optional auth/transport flags.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `addr` | string | `127.0.0.1` | IP/host | `addr: 0.0.0.0` |
| `port` | integer | `5900` | TCP port | `port: 5901` |
| `path` | string | unset | unix socket path | `path: /var/run/ezkvm/vnc.sock` |
| `password` | bool | unset | `true`, `false` | `password: true` |
| `websocket_port` | integer | unset | TCP port | `websocket_port: 6101` |
| `sasl` | bool | unset | `true`, `false` | `sasl: true` |

**Note**: TCP `port: 5900` maps to VNC display `:0` in emitted QEMU args.

## Legacy host

Host passthrough devices.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `pci` | sequence | empty | PCI passthrough entries | `pci: [ { vm_id: "0", host_id: "01:00" } ]` |
| `usb` | sequence | empty | USB passthrough entries | `usb: [ { vendor_id: "0451", product_id: "16a0" } ]` |

### Legacy host.pci entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vm_id` | string | none | index-like string | `vm_id: "0"` |
| `host_id` | string | none | host BDF | `host_id: "01:00"` |
| `port` | string | `1` (implicit bus fallback) | root port suffix | `port: "2"` |
| `multi_function` | bool | unset | `true`, `false` | `multi_function: true` |

### Legacy host.usb entry

Two forms are supported.

**Bus/port form:**

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vm_port` | string | none | USB port string | `vm_port: "1"` |
| `host_bus` | string | none | host bus number string | `host_bus: "1"` |
| `host_port` | string | none | host port path string | `host_port: "7.6"` |

**Vendor/product form:**

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vendor_id` | string | none | 4-digit hex text | `vendor_id: "0451"` |
| `product_id` | string | none | 4-digit hex text | `product_id: "16a0"` |

### Controller-centric shorthand (compatibility)

The parser also accepts a controller-centric shorthand where devices are nested under controllers and then normalized into canonical sections.

Example:

```yaml
controllers:
  scsi:
    - type: pvscsi
      drives:
        - path: "/dev/vm1/vm-108-boot"
          interface: scsi
          type: disk
          format: raw

  xhci:
    - p2: 15
      p3: 15
      usb:
        - hostbus: "1"
          hostport: "2.2"
```

**Normalization rules:**
- `controllers.scsi[].drives[]` is moved to `devices.drives[]`.
- Missing drive `controller` is inferred from the enclosing SCSI controller id.
- `controllers.xhci[].usb[]` is moved to `host.usb[]`.
- Missing USB `bus` is inferred as `<xhci_id>.0`.
- Missing ids for normalized controllers and USB entries are auto-generated.

### Auto-generated ids (canonical schema)

In canonical config, several `id` fields may be omitted (or set to an empty string).
During load/normalization, ezkvm assigns deterministic defaults before validation.

- `devices.drives[].id`: `${interface}${index}` when not explicitly set
- `devices.networks[].id`: `net${index}`
- `controllers.scsi[].id`: `scsihw${index}`
- `controllers.sata[].id`: `sata${index}`
- `controllers.xhci[].id`: `xhci` for index `0`, then `xhci${index}`
- `host.pci[].id`: `hostpci${index}`
- `host.usb[].id`: `usb${index}`

When these fields remain empty, serializer output omits them for compact YAML, and load-time normalization regenerates deterministic IDs from device type and list position before validation.

## Legacy storage

List of controller blocks. Each controller carries a `drives` array.

### Legacy storage controller entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `controller` | string | none | `pvscsi`, `sata`, `ide`, `virtio-scsi-single` | `controller: pvscsi` |
| `offset` | integer | `0` | non-negative integer | `offset: 1` |
| `drives` | sequence | empty | drive entries | see drive table |

### Legacy storage drive entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | none | `hd`, `cd`, and QEMU drive device suffixes used by controller | `type: hd` |
| `file` | string | none | path or `none` for empty CD tray | `file: /dev/vm1/disk0` |
| `discard` | string | unset | typically `on`/`off` | `discard: on` |
| `cache` | string | `none` | QEMU cache mode | `cache: write-back` |
| `format` | string | `raw` | image format | `format: qcow2` |
| `detect_zeroes` | string | `unmap` | QEMU detect-zeroes mode | `detect_zeroes: off` |
| `rotation_rate` | integer | unset | 1-255 | `rotation_rate: 1` |
| `boot_index` | integer | unset | non-negative integer | `boot_index: 0` |
| `aio` | string | unset | `threads`, `native`, `io_uring` | `aio: io_uring` |
| `serial` | string | unset | disk serial string | `serial: rootdisk-01` |
| `snapshot` | bool | unset | `true`, `false` | `snapshot: true` |
| `werror` | string | unset | `enospc`, `stop`, `report`, `ignore` | `werror: report` |
| `rerror` | string | unset | `report`, `ignore`, `stop` | `rerror: report` |
| `iothread` | string | unset | iothread id | `iothread: iothread0` |
| `throttle` | mapping | unset | throttle fields | `throttle: { iops_write: 500 }` |
| `extra_drive_options` | sequence | empty | raw drive options | `extra_drive_options: [ "detect-zeroes=off" ]` |
| `extra_device_options` | sequence | empty | raw device options | `extra_device_options: [ "share-rw=on" ]` |

### Legacy storage.throttle entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `bps_total` | integer | unset | bytes per second | `bps_total: 104857600` |
| `bps_read` | integer | unset | bytes per second | `bps_read: 52428800` |
| `bps_write` | integer | unset | bytes per second | `bps_write: 52428800` |
| `iops_total` | integer | unset | IOPS | `iops_total: 1000` |
| `iops_read` | integer | unset | IOPS | `iops_read: 500` |
| `iops_write` | integer | unset | IOPS | `iops_write: 500` |

## Legacy network

List of NIC entries composed of a payload (`type`) plus footer options.

**Supported payload types**: `bridge`, `user`, `tap`, `socket`, `macvtap`, `vhost_user`, `proxmox_tap`, `x550vf`.

### Legacy network entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | none | payload type list above | `type: bridge` |
| `mac` | string | random | MAC address | `mac: BC:24:11:FF:76:89` |
| `queues` | integer | unset | >1 enables multiqueue | `queues: 4` |
| `extra_netdev_options` | sequence | empty | raw netdev options | `extra_netdev_options: [ "vhost=on" ]` |
| `extra_device_options` | sequence | empty | raw device options | `extra_device_options: [ "romfile=" ]` |

### Legacy network type-specific field examples

| Type | Key fields |
| --- | --- |
| `bridge` | `bridge`, `driver` |
| `user` | `id`, `hostfwd` |
| `tap` | `ifname`, `script`, `downscript`, `driver` |
| `socket` | `listen`, `connect`, `driver` |
| `macvtap` | `ifname`, `driver` |
| `vhost_user` | `chardev`, `driver` |
| `proxmox_tap` | Proxmox-compatible tap fields |
| `x550vf` | Intel x550 VF-specific fields |

## Legacy extras

Raw QEMU arguments appended after generated arguments.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `extras` | sequence of strings | empty | any QEMU CLI fragment | `extras: [ "-overcommit mem-lock=on", "-global ICH9-LPC.disable_s3=1" ]` |

Use this section for advanced options not yet modeled in schema.

## See Also

- [VM Structure](vm-structure.md)
- [System and Boot](system-and-boot.md)
- [Devices, Controllers, and Host Passthrough](devices.md)
- [Profiles and Merge](profiles-and-merge.md)
- [Platform Features and Options](platform-features.md)
