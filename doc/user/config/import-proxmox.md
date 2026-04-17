# Proxmox Import Mapping

This document summarizes how Proxmox VM config fields map into canonical ezkvm YAML during import.

## Scalar and Section Mapping

| Proxmox input | ezkvm output | Notes |
| --- | --- | --- |
| `name` | `name` | Can be overridden by import CLI name argument. |
| `cores`, `sockets`, `cpu` | `system.cpu.*` | `cpu.flags` converted from semicolon to comma list. |
| `memory` | `system.memory.size` | MiB value preserved. |
| `balloon` | `system.memory.ballooning` | Mapped to canonical ballooning settings. |
| `hugepages` | `system.memory.hugepages` | Non-zero numeric values map to enabled. |
| `machine: q35...` | `system.machine` and `system.machine_options` | Unsupported machine values are warned. |
| `bios` / `efidisk0` | `system.boot.*` | `efidisk0` implies UEFI when BIOS is absent. |
| `tpmstate0` | `system.tpm` | Requires resolvable absolute source path. |
| `serial0: socket` | `devices.serials[]` | Path inferred from vmid or VM name when possible. |
| `numa` + `hugepages` | `system.cpu.numa` | Nodes derived from sockets/cores/memory. |
| `vmgenid` | `system.smbios.vm_generation_id` | Passed through directly. |
| `vga` | `devices.displays[]` | Display device inferred from model. |
| `boot: order=...` | `devices.drives[].boot_index` / `devices.networks[].boot_index` | Converted to deterministic order positions. |
| `scsihw` | `controllers.scsi[]` | Controller type mapped to canonical controller config. |
| `netN` with `bridge` | `devices.networks[]` | Uses canonical network backend mapping. |
| `hostpciN` | `host.pci[]` | `x-vga` devices preserved in passthrough settings. |
| `usbN` | `host.usb` | Supports `<bus>-<port>` and `<vendor>:<product>` host selectors. |
| `args` | Canonical typed fields where possible | Unsupported tokens are emitted as warnings. |

## Import Caveats and Migration Checklist

### Profile-Aware Import Output

Import output is profile-aware and compacted:
- importer-inferred `profiles` are emitted
- redundant VM-local fields already provided by those profiles may be omitted
- sparse id-merge sections can omit repeated default fields when they are owned by schema defaults (for example repeated controller `type` values)

By default, `import-proxmox` uses `--output-mode compact`.

### Export Modes

`import-proxmox` supports explicit output modes:

- `--output-mode compact` (default): profile-overlay form with profile-owned fields omitted
- `--output-mode canonical`: full explicit mapped YAML before profile compaction
- `--output-mode debug`: canonical output plus deterministic IDs and source comments

Examples:

```bash
# default compact profile-overlay output
ezkvm import-proxmox /etc/pve/qemu-server/108.conf --dry-run

# full explicit mapped output
ezkvm import-proxmox /etc/pve/qemu-server/108.conf --output-mode canonical --dry-run

# debug-friendly output with source comments and deterministic ids
ezkvm import-proxmox /etc/pve/qemu-server/108.conf --output-mode debug --dry-run
```

Debug mode includes source comments such as:

```yaml
# from Proxmox ostype: win11
```

### List Path Merge Safety

For merge-safe list paths (for example id-merged controller/host device lists and append-unique option lists), redundant overlay entries may be pruned while preserving the same merged runtime result.

In sparse sections, repeated values may also be omitted when profile base data or schema defaults provide the same effective value after merge.

### Profile Resolution Requirement

Re-parsing imported YAML requires profile resolution to remain available via `locations.profile_dir` (or `EZKVM_CONFIG`).

### Known Limitations

- `virtio` disk bus is currently skipped during typed controller mapping
- Non-absolute storage references may require manual path translation
- Some Proxmox CPU options are not represented in typed schema and may need manual `extras` entries
- VGA models without direct typed mapping can be preserved as raw `extras`

### Post-Import Validation Checklist

After import, validate:
1. Boot order (`devices.drives[].boot_index`, `devices.networks[].boot_index`)
2. Display path (`devices.displays[]` plus `spice`/`vnc` settings as needed)
3. Passthrough device identity (`host.pci[].device`, `host.usb[]` entries)
4. Network backend type (`devices.networks[].backend.type` for migration target)

## See Also

- [Troubleshooting](troubleshooting.md)
- [VM Structure](vm-structure.md)
- [Central Config](central-config.md)
