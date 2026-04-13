# Central Config

Central config is loaded from:

- `/etc/ezkvm/ezkvm.yaml`
- `/etc/ezkvm.yaml` (legacy fallback)

Override path with `EZKVM_CONFIG`.

## Schema

```yaml
tools:
  swtpm: "/usr/bin/swtpm"
  remote_viewer: "/usr/bin/remote-viewer"

locations:
  run_dir: "/var/run/ezkvm"
  ovmf_dir: "/usr/share/OVMF"
  vm_dir: "/etc/ezkvm/vms"
  template_dir: "/etc/ezkvm/templates"
  profile_dir: "/etc/ezkvm/profiles.d"
```

## Runtime Usage

- `tools.swtpm` + `system.tpm.backend: emulator`: ezkvm can launch swtpm.
- `tools.remote_viewer` + `spice.enabled`: ezkvm can launch remote-viewer.
- `options.looking_glass.program` + `system.memory.ivshmem.enabled`: ezkvm can launch looking-glass-client.

Looking Glass client settings should be defined in VM/profile config under `options.looking_glass` so profile-specific behavior stays with the profile.

## Practical Defaults

- TPM socket path defaults to `${run_dir}/${vm-name}.swtpm` when `run_dir` is set.
- Otherwise TPM socket path defaults to `/var/run/qemu-server/${vm-name}.swtpm`.
- Profile files resolve from `locations.profile_dir`, else `/etc/ezkvm/profiles.d`.

## See also

- [VM structure](vm-structure.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)