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

## options.looking_glass

- `program` (path to looking-glass-client)
- `full_screen`
- `size` (`WIDTHxHEIGHT`)
- `grab_keyboard`
- `escape_key`

## options.guest_agent (canonical)

- `enabled` (default true)
- `socket_path` (optional)
- when `socket_path` is omitted, default is `<runtime_root>/<vm-name>.qga`
- `freeze_cpu` (default false)
- optional placement: `bus`, `addr`

For Q35 with bridge topology (loaded readconfig or portable synthesized mode),
omitted guest-agent `bus`/`addr` default to `pci.0` and `0x8`.

## options.qmp (canonical)

- `enabled` (default true)
- `socket_path` (optional absolute path)
- `socket_type` (`unix` default, `tcp`)

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

## system.memory.ivshmem (canonical)

- `enabled` (default true)
- `size` MiB (default 32)
- `vectors` (default 1)
- `id` (default ivshmem0)
- optional `bus`, `addr`
- `mem_path` (default /dev/kvmfr0)

When Q35 bridge readconfig is loaded (`pve-q35-*` or `ezkvm-q35.cfg`), omitted
`bus`/`addr` default to `pcie.0` and `0x8` for `ivshmem-plain` emission.
Portable Q35 mode keeps this same default placement through synthesized topology.
This default is applied at runtime for both parity/static and portable-synthesized
Q35 topology modes.
For virtio-gpu placement defaults, see `devices.displays` in
[Devices, controllers, and host passthrough](devices.md).

## system.cpu.numa (canonical)

NUMA node entries:

- `id`, `memory`, `cpus`
- optional `host_node`

## See also

- [System and boot](system-and-boot.md)
- [Devices, controllers, and host passthrough](devices.md)
- [Profiles and merge](profiles-and-merge.md)
- [Examples](examples.md)