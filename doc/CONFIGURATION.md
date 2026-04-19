# Configuration

Configuration files are YAML and are searched in this order:

1. current directory
2. `~/.ezkvm`
3. `/etc/ezkvm`

This document is organized as chapters and includes field reference tables for each top-level section.

## Table of Contents

- [Central Config](#central-config)
- [Chapter 1: Top-Level Schema](#chapter-1-top-level-schema)
- [Chapter 2: general](#chapter-2-general)
- [Chapter 3: system](#chapter-3-system)
- [Chapter 4: gpu](#chapter-4-gpu)
- [Chapter 5: display](#chapter-5-display)
- [Chapter 6: spice](#chapter-6-spice)
- [Chapter 7: vnc](#chapter-7-vnc)
- [Chapter 8: host](#chapter-8-host)
- [Chapter 9: storage](#chapter-9-storage)
- [Chapter 10: network](#chapter-10-network)
- [Chapter 11: extras](#chapter-11-extras)
- [Chapter 12: Comprehensive Examples](#chapter-12-comprehensive-examples)
- [Chapter 13: Troubleshooting](#chapter-13-troubleshooting)
- [Chapter 14: Proxmox Import Mapping](#chapter-14-proxmox-import-mapping)
- [Chapter 15: Code-Backed Shape Examples](#chapter-15-code-backed-shape-examples)

## Central Config

The central config is distinct from VM config.

Use it for:

- host tool paths
- shared directory locations
- host capability defaults and deployment policy

Do not use it for:

- guest-visible topology
- per-VM semantics that belong in VM YAML or profiles
- importer-specific semantic overrides

Current central config top-level fields:

| Field | Type | Purpose |
| --- | --- | --- |
| `tools` | mapping | legacy-compatible tool paths and generic executables |
| `locations` | mapping | shared directories such as VM config and profile directories |
| `looking_glass` | mapping | legacy-compatible central defaults for Looking Glass client options |
| `host_capabilities` | mapping | host capability defaults used by portable runtime resolution |

### Central Config: `host_capabilities`

`host_capabilities` stores host environment facts and deployment policy. It is the preferred place for portability-related defaults.

| Field | Type | Purpose |
| --- | --- | --- |
| `runtime` | mapping | runtime directory layout |
| `firmware` | mapping | firmware discovery defaults |
| `network` | mapping | network backend/helper policy |
| `tpm` | mapping | swtpm binary and TPM state/socket placement |
| `integrations` | mapping | optional host integration programs and devices |

#### `host_capabilities.runtime`

| Field | Type | Example |
| --- | --- | --- |
| `run_dir` | string | `/var/run/ezkvm` |
| `pid_dir` | string | `/var/run/ezkvm/pids` |
| `socket_dir` | string | `/var/run/ezkvm/sockets` |
| `log_dir` | string | `/var/log/ezkvm` |

#### `host_capabilities.firmware`

| Field | Type | Example |
| --- | --- | --- |
| `ovmf_dir` | string | `/usr/share/OVMF` |

#### `host_capabilities.network`

| Field | Type | Example |
| --- | --- | --- |
| `preferred_backend` | string | `bridge` |
| `bridge_helper` | string | `/usr/lib/qemu/qemu-bridge-helper` |
| `bridge_name` | string | `br0` |

#### `host_capabilities.tpm`

| Field | Type | Example |
| --- | --- | --- |
| `swtpm_binary` | string | `/usr/bin/swtpm` |
| `state_dir` | string | `/var/lib/ezkvm/tpm` |
| `socket_dir` | string | `/var/run/ezkvm/tpm` |

#### `host_capabilities.integrations`

| Field | Type | Example |
| --- | --- | --- |
| `remote_viewer.program` | string | `/usr/bin/remote-viewer` |
| `looking_glass.program` | string | `/usr/bin/looking-glass-client` |
| `looking_glass.shared_memory_device` | string | `/dev/kvmfr0` |

### Central Config precedence inside host capability resolution

For fields that exist in both the new capability sections and older compatibility sections, the capability section wins.

Examples:

- `host_capabilities.tpm.swtpm_binary` overrides legacy `tools.swtpm`
- `host_capabilities.runtime.run_dir` overrides legacy `locations.run_dir`
- `host_capabilities.integrations.remote_viewer.program` overrides legacy `tools.remote_viewer`

This precedence is limited to central-config-internal compatibility. Broader runtime precedence between CLI, VM-local config, profiles, and central config is defined separately by the runtime precedence contract.

## Chapter 1: Top-Level Schema

Top-level sections:

1. `general`
2. `profiles`
3. `system`
4. `gpu`
5. `display`
6. `spice`
7. `vnc`
8. `host`
9. `storage`
10. `network`
11. `extras`

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

Profile layering behavior:

- Profiles are loaded from the configured profile directory before VM-local values are applied.
- VM-local values override profile values on conflict.
- Profile names map to YAML files in the profile directory. For example `windows-11` resolves to `windows-11.yaml`.
- The central config field `locations.profile_dir` controls the directory, with `/etc/ezkvm/profiles.d` as the default.
- Profile layering is applied both when loading VM configs from files and when loading from YAML strings.

Current built-in layered profile names in this repo include:

- `proxmox-q35-uefi`
- `storage-virtio-scsi-single`
- `storage-virtio-scsi-pci`
- `windows-common`
- `windows-10`
- `windows-11`
- `linux-l26-common`
- `macos-kvm`
- `remote-viewer-spice`
- `looking-glass`
- `gpu-passthrough`
- `headless-serial`

## Chapter 2: general

VM identity and control-plane sockets.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `name` | string | `anonymous` | any non-empty name | `name: ubuntu-dev` |
| `uuid` | string | none | UUID string | `uuid: c0e240a5-859a-4378-a2d9-95088f531142` |
| `monitor` | yes/no | `no` | `yes`, `no` | `monitor: yes` |
| `agent` | yes/no | `no` | `yes`, `no` | `agent: yes` |

## Chapter 3: system

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

### system.chipset

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `q35` | `q35`, `i440fx` | `type: q35` |
| `machine` (q35) | string | `q35` | any QEMU machine string | `machine: q35` |
| `pcie_root_ports` (q35) | sequence | empty | root-port objects | `pcie_root_ports: [ { id: rp0, chassis: 10, slot: 1, bus: pcie.0, addr: 0x2 } ]` |
| `xhci_enabled` (q35) | bool | `true` | `true`, `false` | `xhci_enabled: false` |
| `xhci_bus` (q35) | string | `pci.1` | bus name | `xhci_bus: pci.1` |
| `xhci_addr` (q35) | string | `0x1b` | hex-like slot | `xhci_addr: 0x1b` |

### system.bios

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

### system.memory

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `max` | integer (MiB) | `16384` | positive integer | `max: 32768` |
| `balloon` | bool | unset | `true`, `false` | `balloon: true` |
| `hugepages` | bool | unset | `true`, `false` | `hugepages: true` |
| `prealloc` | bool | unset | `true`, `false` | `prealloc: true` |
| `mem_path` | string | unset | path | `mem_path: /run/hugepages/kvm/1048576kB` |

NUMA backend behavior:

- If `memory.hugepages: true` and `numa_nodes` is not empty, ezkvm emits per-node `memory-backend-file` objects and `-numa node,...,memdev=...`.
- Otherwise, ezkvm emits flat memory arguments such as `-m`, optional `-mem-path`, and optional `-mem-prealloc`.

### system.cpu

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `model` | string | `qemu64` | QEMU CPU model | `model: host` |
| `sockets` | integer | `1` | positive integer | `sockets: 1` |
| `dies` | integer | unset | positive integer | `dies: 2` |
| `clusters` | integer | unset | positive integer | `clusters: 2` |
| `cores` | integer | `4` | positive integer | `cores: 8` |
| `threads` | integer | `1` | positive integer | `threads: 2` |
| `flags` | string | `+aes,+pni,+popcnt,+sse4.1,+sse4.2,+ssse3,enforce` | CPU flag list | `flags: "+aes,enforce"` |

### system.tpm

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `no_tpm` | `no_tpm`, `swtpm`, `passthrough` | `type: swtpm` |
| `disk` (swtpm) | string | required for swtpm | path | `disk: /dev/vm1/vm-108-tpmstate` |
| `socket` (swtpm) | string | required for swtpm | path | `socket: /var/ezkvm/wakiza-tpm.socket` |

### system.serial

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | none | `socket` | `type: socket` |
| `path` | string | none | unix path | `path: /var/run/qemu-server/301.serial0` |

### system.numa_nodes and system.numa_distances

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

## Chapter 4: gpu

Selects guest GPU model or passthrough profile.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | `no_gpu` | `no_gpu`, `passthrough`, `virtio`, `vmware-svga` | `type: virtio` |
| `pci` (passthrough) | sequence | empty | `Pci` objects | `pci: [ { vm_id: "0", host_id: "01:00" } ]` |
| `vga` (virtio) | yes/no | `yes` | `yes`, `no` | `vga: yes` |
| `gl` (virtio) | yes/no | `no` | `yes`, `no` | `gl: yes` |
| `pci` or `pcie` address (virtio) | mapping | optional | bus/address pair | `pcie: { bus: 0, address: 2 }` |
| `pci_address` (vmware-svga) | string | `0x1` | slot string | `pci_address: 0x2` |

## Chapter 5: display

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

## Chapter 6: spice

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

## Chapter 7: vnc

VNC endpoint and optional auth/transport flags.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `addr` | string | `127.0.0.1` | IP/host | `addr: 0.0.0.0` |
| `port` | integer | `5900` | TCP port | `port: 5901` |
| `path` | string | unset | unix socket path | `path: /var/run/ezkvm/vnc.sock` |
| `password` | bool | unset | `true`, `false` | `password: true` |
| `websocket_port` | integer | unset | TCP port | `websocket_port: 6101` |
| `sasl` | bool | unset | `true`, `false` | `sasl: true` |

Note: TCP `port: 5900` maps to VNC display `:0` in emitted QEMU args.

## Chapter 8: host

Host passthrough devices.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `pci` | sequence | empty | PCI passthrough entries | `pci: [ { vm_id: "0", host_id: "01:00" } ]` |
| `usb` | sequence | empty | USB passthrough entries | `usb: [ { vendor_id: "0451", product_id: "16a0" } ]` |

### host.pci entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vm_id` | string | none | index-like string | `vm_id: "0"` |
| `host_id` | string | none | host BDF | `host_id: "01:00"` |
| `port` | string | `1` (implicit bus fallback) | root port suffix | `port: "2"` |
| `multi_function` | bool | unset | `true`, `false` | `multi_function: true` |

### host.usb entry

Two forms are supported.

Bus/port form:

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vm_port` | string | none | USB port string | `vm_port: "1"` |
| `host_bus` | string | none | host bus number string | `host_bus: "1"` |
| `host_port` | string | none | host port path string | `host_port: "7.6"` |

Vendor/product form:

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `vendor_id` | string | none | 4-digit hex text | `vendor_id: "0451"` |
| `product_id` | string | none | 4-digit hex text | `product_id: "16a0"` |

### Controller-centric shorthand (compatibility)

The parser also accepts a controller-centric shorthand where devices are nested under
controllers and then normalized into canonical sections.

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

Normalization rules:
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

When these fields remain empty, serializer output omits them for compact YAML,
and load-time normalization regenerates deterministic IDs from device type and
list position before validation.

## Chapter 9: storage

List of controller blocks. Each controller carries a `drives` array.

### storage controller entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `controller` | string | none | `pvscsi`, `sata`, `ide`, `virtio-scsi-single` | `controller: pvscsi` |
| `offset` | integer | `0` | non-negative integer | `offset: 1` |
| `drives` | sequence | empty | drive entries | see drive table |

### storage drive entry

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

### storage.throttle entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `bps_total` | integer | unset | bytes per second | `bps_total: 104857600` |
| `bps_read` | integer | unset | bytes per second | `bps_read: 52428800` |
| `bps_write` | integer | unset | bytes per second | `bps_write: 52428800` |
| `iops_total` | integer | unset | IOPS | `iops_total: 1000` |
| `iops_read` | integer | unset | IOPS | `iops_read: 500` |
| `iops_write` | integer | unset | IOPS | `iops_write: 500` |

## Chapter 10: network

List of NIC entries composed of a payload (`type`) plus footer options.

Supported payload types: `bridge`, `user`, `tap`, `socket`, `macvtap`, `vhost_user`, `proxmox_tap`, `x550vf`.

### network entry

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `type` | string | none | payload type list above | `type: bridge` |
| `mac` | string | random | MAC address | `mac: BC:24:11:FF:76:89` |
| `queues` | integer | unset | >1 enables multiqueue | `queues: 4` |
| `extra_netdev_options` | sequence | empty | raw netdev options | `extra_netdev_options: [ "vhost=on" ]` |
| `extra_device_options` | sequence | empty | raw device options | `extra_device_options: [ "romfile=" ]` |

### network type-specific field examples

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

## Chapter 11: extras

Raw QEMU arguments appended after generated arguments.

| Field | Type | Default | Valid values | Example |
| --- | --- | --- | --- | --- |
| `extras` | sequence of strings | empty | any QEMU CLI fragment | `extras: [ "-overcommit mem-lock=on", "-global ICH9-LPC.disable_s3=1" ]` |

Use this section for advanced options not yet modeled in schema.

## Chapter 12: Comprehensive Examples

### Example A: Desktop with Remote Viewer + SPICE TLS

```yaml
general:
  name: ubuntu-desktop
  monitor: yes
  agent: yes

system:
  chipset: { type: q35 }
  bios:
    type: ovmf
    file: /dev/vm1/ubuntu-desktop-efi
    uuid: c0e240a5-859a-4378-a2d9-95088f531142
  cpu: { model: host, sockets: 1, cores: 8, threads: 1 }
  memory: { max: 16384, balloon: true }
  tpm: { type: swtpm, disk: /dev/vm1/ubuntu-desktop-tpmstate, socket: /var/ezkvm/ubuntu-desktop-tpm.socket }

gpu:
  type: virtio
  gl: yes

display:
  type: remote-viewer
  auto_resize: true
  full_screen: false
  usb_tablet: true

spice:
  addr: 127.0.0.1
  port: 5900
  tls_port: 61000
  tls_ciphers: HIGH
  x509_dir: /etc/pki/qemu
  seamless_migration: true
  disable_ticketing: true

storage:
  - controller: pvscsi
    drives:
      - { type: hd, file: /dev/vm1/ubuntu-root, discard: on, boot_index: 0 }

network:
  - { type: bridge, bridge: vmbr0, mac: BC:24:11:FF:76:89 }
```

### Example B: Headless Serial + VNC

```yaml
general:
  name: build-runner

system:
  chipset: { type: q35, xhci_enabled: false }
  bios: { type: seabios, uuid: e7f0f0f1-0000-4e4a-a999-111111111111 }
  cpu: { model: qemu64, sockets: 1, cores: 4, threads: 1 }
  memory: { max: 8192, hugepages: false }
  serial: { type: socket, path: /var/run/qemu-server/301.serial0 }

gpu:
  type: no_gpu

display:
  type: no_display

vnc:
  addr: 127.0.0.1
  port: 5901
  password: true

storage:
  - controller: sata
    drives:
      - { type: hd, file: /dev/vm2/build-root, format: raw, cache: none, boot_index: 0 }

network:
  - { type: user }
```

### Example C: PCI/USB Passthrough Workstation

```yaml
general:
  name: windows-gaming

system:
  chipset: { type: q35 }
  bios:
    type: ovmf
    file: /dev/vm3/windows-gaming-efi
  cpu: { model: host, sockets: 1, cores: 12, threads: 2 }
  memory: { max: 32768 }
  vmgenid: b42d5b83-fee2-47dc-98a8-7856b18542ec

gpu:
  type: passthrough
  pci:
    - { vm_id: "0", host_id: "01:00", port: "1", multi_function: true }
    - { vm_id: "1", host_id: "01:00", port: "1" }

display:
  type: no_display

host:
  usb:
    - { vendor_id: "0451", product_id: "16a0" }
    - { vm_port: "1", host_bus: "1", host_port: "7.6" }

storage:
  - controller: virtio-scsi-single
    drives:
      - { type: hd, file: /dev/vm3/windows-disk0, discard: on, boot_index: 0 }

network:
  - { type: bridge, bridge: vmbr0 }
```

### Example D: NUMA + Hugepages

```yaml
general:
  name: numa-linux

system:
  chipset: { type: q35 }
  bios: { type: ovmf, file: /dev/vm4/numa-efi }
  cpu: { model: host, sockets: 2, dies: 1, clusters: 1, cores: 12, threads: 1 }
  memory:
    max: 65536
    hugepages: true
    mem_path: /run/hugepages/kvm/1048576kB
  numa_nodes:
    - nodeid: 0
      cpus: "0-11"
      mem: 32768
      host_nodes: 0
      policy: bind
    - nodeid: 1
      cpus: "12-23"
      mem: 32768
      host_nodes: 1
      policy: bind
  numa_distances:
    - { src: 0, dst: 1, val: 20 }
    - { src: 1, dst: 0, val: 20 }

gpu:
  type: no_gpu

display:
  type: no_display

network:
  - { type: bridge, bridge: vmbr1 }

storage:
  - controller: pvscsi
    drives:
      - { type: hd, file: /dev/vm4/root, cache: none, discard: on, boot_index: 0 }
```

## Chapter 13: Troubleshooting

Common issues and quick checks.

### VM fails with memory backend or hugepages errors

Symptoms:

- QEMU reports memory backend creation errors.
- Startup fails when `hugepages` and `numa_nodes` are enabled.

Likely causes:

- `system.memory.mem_path` does not exist on host.
- Hugepages are enabled but host hugepage mount is not available.
- `numa_nodes[].mem` values do not align with intended node sizing.

Checks:

- Confirm configured path exists and is mounted for hugepages.
- Temporarily disable `hugepages` to validate basic NUMA shape.
- Start with one NUMA node, then expand.

### No remote display appears

Symptoms:

- No viewer window appears.
- Viewer opens but cannot connect.

Likely causes:

- `display.type` is `no_display`.
- Missing or conflicting `spice`/`vnc` endpoint config.
- Local environment cannot launch remote-viewer binary.

Checks:

- For SPICE flow, use `display.type: remote-viewer` and a valid `spice` section.
- For headless operation, use `display.type: no_display` and connect via serial/VNC intentionally.
- If using VNC TCP mode, verify expected display/port mapping.

### Imported Proxmox VM boots from wrong disk

Symptoms:

- Guest enters firmware menu.
- Wrong drive selected as first boot target.

Likely causes:

- Proxmox `boot: order=...` did not map to expected `boot_index` values.
- Controller translation changed device naming.

Checks:

- Verify `storage[].drives[].boot_index` ordering.
- Ensure target root disk has the lowest valid boot index.

### PCI passthrough does not attach

Symptoms:

- Device missing inside guest.
- QEMU rejects VFIO device arguments.

Likely causes:

- Host device id is wrong or missing function suffix.
- `host.pci[].port` / `vm_id` combination conflicts.

Checks:

- Validate `host.pci[].host_id` format and function suffix.
- Start with one passthrough device, then add multi-function siblings.

### USB passthrough ignored

Symptoms:

- USB device not visible in guest.

Likely causes:

- Wrong selector format for the chosen mode.
- Device re-enumerates on host and bus/port changed.

Checks:

- Bus/port mode requires `vm_port`, `host_bus`, and `host_port`.
- Vendor/product mode requires `vendor_id` and `product_id` as hex text.

## Chapter 14: Proxmox Import Mapping

This section summarizes how Proxmox VM config fields map into ezkvm YAML during import.

### Scalar and section mapping

| Proxmox input | ezkvm output | Notes |
| --- | --- | --- |
| `name` | `general.name` | Can be overridden by import CLI name argument. |
| extracted VM UUID | `general.uuid` | If available from Proxmox input. |
| `agent` | `general.agent` | Mapped to yes/no form. |
| `monitor` | `general.monitor` | Mapped to yes/no form. |
| `cores`, `sockets`, `cpu` | `system.cpu.*` | `cpu.flags` converted from semicolon to comma list. |
| `memory` | `system.memory.max` | MiB value preserved. |
| `balloon` | `system.memory.balloon` | bool-ish parsing. |
| `hugepages` | `system.memory.hugepages` | Non-zero numeric values map to enabled. |
| `machine: q35...` | `system.chipset.type: q35` | Unsupported machine values are skipped with warning. |
| `bios` / `efidisk0` | `system.bios.*` | `efidisk0` implies ovmf when bios absent. |
| `tpmstate0` | `system.tpm` (`swtpm`) | Requires resolvable absolute source path. |
| `rng0` | `system.virtio_rng.filename` | `max_bytes` and `period` are warned as unmapped. |
| `serial0: socket` | `system.serial` | Path inferred from vmid or VM name. |
| `numa` + `hugepages` | `system.numa_nodes` | Nodes derived from sockets/cores/memory. |
| `vmgenid` | `system.vmgenid` | Passed through directly. |
| `vga` and passthrough grouping | `gpu`, `display`, `spice`, `vnc` | Display path inferred; remote-viewer path defaults to SPICE. |
| `boot: order=...` | `storage[].drives[].boot_index` | Converted to 1-based order position. |
| `scsihw: virtio-scsi-single` | storage controller type | SCSI bus maps to `virtio-scsi-single` instead of `pvscsi`. |
| `netN` with `bridge` | `network[]` | Uses `proxmox_tap` when vmid is known, else `bridge`. |
| `hostpciN` | `gpu.pci` or `host.pci` | `x-vga` devices grouped into GPU passthrough. |
| `usbN` | `host.usb` | Supports `<bus>-<port>` and `<vendor>:<product>` host selectors. |
| `args` | `extras[]` | Shell-split raw tokens appended to extras. |

### Import caveats and migration checklist

- Import output is profile-aware and compacted: importer-inferred `profiles` are emitted and redundant VM-local fields already provided by those profiles may be omitted.
- For merge-safe list paths (for example id-merged controller/host device lists and append-unique option lists), redundant overlay entries may be pruned while preserving the same merged runtime result.
- Re-parsing imported YAML therefore requires profile resolution to remain available via `locations.profile_dir` (or `EZKVM_CONFIG`).
- `virtio` disk bus is currently skipped during typed controller mapping.
- Non-absolute storage references may require manual path translation.
- Some Proxmox CPU options are not represented in typed schema and may need manual `extras` entries.
- VGA models without direct typed mapping can be preserved as raw `extras`.
- After import, validate: boot order, display path, passthrough device identity, and network backend type.

## Chapter 15: Code-Backed Shape Examples

These minimal examples are derived from active serde-backed shapes in the implementation.

### Shape 1: system.serial socket

```yaml
system:
  serial:
    type: socket
    path: /var/run/qemu-server/301.serial0
```

### Shape 2: host.usb vendor/product

```yaml
host:
  usb:
    - vendor_id: "0451"
      product_id: "16a0"
```

### Shape 3: display.remote-viewer

```yaml
display:
  type: remote-viewer
  auto_resize: true
  full_screen: false
  usb_tablet: true
```

### Shape 4: system.numa_nodes with hugepages memory

```yaml
system:
  memory:
    max: 32768
    hugepages: true
    mem_path: /run/hugepages/kvm/1048576kB
  numa_nodes:
    - nodeid: 0
      cpus: "0-11"
      mem: 32768
      host_nodes: 0
      policy: bind
```

### Shape 5: storage controller + drive throttle

```yaml
storage:
  - controller: pvscsi
    drives:
      - type: hd
        file: /dev/vm1/root
        cache: none
        format: raw
        throttle:
          bps_read: 52428800
          iops_write: 500
```