# Platform Features and Options

## options

Required values in current schema:

- `options.enable_kvm` (bool)
- `options.daemonize` (bool)

Other fields:

- `nodefaults`
- `global_options`
- `rtc.base`, `rtc.driftfix`
- `pid_file`, `log_dir`, `log_keep`
- `uefi_vars` (options-level field exists; boot flow uses `system.boot.uefi_vars`)

## options.guest_agent (canonical)

- `enabled` (default true)
- `socket_path` (optional)
- `freeze_cpu` (default false)
- optional placement: `bus`, `addr`

Legacy alias accepted during migration: top-level `guest_agent`.

## options.qmp (canonical)

- `enabled` (default true)
- `socket_path` (optional absolute path)
- `socket_type` (`unix` default, `tcp`)

Legacy alias accepted during migration: top-level `qmp`.

## spice (top-level)

- `enabled` (default true)
- `port` (default 5900)
- `addr` (default 127.0.0.1)
- `disable_ticketing`, `audio`, `vdagent`

## hyperv (top-level)

Feature flags for Windows optimization:

- `enabled`, `relaxed`, `vapic`, `time`
- `crash`, `reset`, `vendor_id`
- `frequencies`, `reenlightenment`, `tlbflush`, `ipi`, `spinlock_retry`

## system.memory.ballooning (canonical)

- `enabled` (default true)
- `free_page_reporting`
- `model` (virtio-balloon-pci or virtio-balloon-ccw)
- optional `id`, `bus`, `addr`

Legacy alias accepted during migration: top-level `ballooning`.

## system.memory.ivshmem (canonical)

- `enabled` (default true)
- `size` MiB (default 32)
- `vectors` (default 1)
- `id` (default ivshmem0)
- optional `bus`
- `mem_path` (default /dev/kvmfr0)

Legacy alias accepted during migration: top-level `ivshmem`.

## system.cpu.numa (canonical)

NUMA node entries:

- `id`, `memory`, `cpus`
- optional `host_node`

Legacy alias accepted during migration: top-level `numa`.

## See also

- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Profiles and merge](profiles-and-merge.md)
- [Examples](examples.md)