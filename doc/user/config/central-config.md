# Central Config

Central config is loaded from:

- `/etc/ezkvm/ezkvm.yaml`
- `/etc/ezkvm.yaml` (legacy fallback)

Override path with `EZKVM_CONFIG`.

Use central config for:

- host tool paths and integration binaries
- shared host directories
- host capability defaults used by portable runtime resolution

Do not use it for:

- guest-visible topology
- per-VM semantics that belong in VM YAML or profiles
- importer-specific semantic overrides

## Schema

```yaml
tools:
  qemu: "/usr/bin/qemu-system-x86_64"
  swtpm: "/usr/bin/swtpm"
  remote_viewer: "/usr/bin/remote-viewer"
  looking_glass: "/usr/bin/looking-glass-client"

locations:
  run_dir: "/var/run/ezkvm"
  pid_dir: "/var/run/ezkvm/pids"
  socket_dir: "/var/run/ezkvm/sockets"
  log_dir: "/var/log/ezkvm"
  ovmf_dir: "/usr/share/OVMF"
  vm_dir: "/etc/ezkvm/vms"
  template_dir: "/etc/ezkvm/templates"
  profile_dir: "/etc/ezkvm/profiles.d"

looking_glass:
  program: "/usr/bin/looking-glass-client"

host_capabilities:
  runtime:
    run_dir: "/var/run/ezkvm"
    pid_dir: "/var/run/ezkvm/pids"
    socket_dir: "/var/run/ezkvm/sockets"
    log_dir: "/var/log/ezkvm"
  firmware:
    ovmf_dir: "/usr/share/OVMF"
  network:
    preferred_backend: "bridge"
    bridge_helper: "/usr/lib/qemu/qemu-bridge-helper"
    bridge_name: "br0"
  tpm:
    swtpm_binary: "/usr/bin/swtpm"
    state_dir: "/var/lib/ezkvm/tpm"
    socket_dir: "/var/run/ezkvm/tpm"
  integrations:
    remote_viewer:
      program: "/usr/bin/remote-viewer"
    looking_glass:
      program: "/usr/bin/looking-glass-client"
      shared_memory_device: "/dev/kvmfr0"
```

The preferred place for portability-related defaults is `host_capabilities`. The older `tools`, `locations`, and top-level `looking_glass` sections remain available for compatibility.

## Host Capability Sections

`host_capabilities.runtime`

- `run_dir`: shared runtime root
- `pid_dir`: pid file directory
- `socket_dir`: unix socket directory
- `log_dir`: runtime log directory

`host_capabilities.firmware`

- `ovmf_dir`: firmware discovery directory

`host_capabilities.network`

- `preferred_backend`: default host networking strategy for portable runtime
- `bridge_helper`: bridge-helper binary path
- `bridge_name`: default bridge device name

`host_capabilities.tpm`

- `swtpm_binary`: swtpm executable path
- `state_dir`: TPM state directory
- `socket_dir`: TPM control socket directory

`host_capabilities.integrations`

- `remote_viewer.program`: remote-viewer binary path
- `looking_glass.program`: looking-glass-client binary path
- `looking_glass.shared_memory_device`: shared-memory device exposed by the host

## Runtime Usage

- Runtime precedence contract: `CLI overrides > VM-local explicit values > profile defaults > central host defaults > built-in fallback`.
- `ezkvm start` and `ezkvm start --dry-run` execute the same preflight validation pass before runtime orchestration.
- `host_capabilities.tpm.swtpm_binary` or legacy `tools.swtpm` + `system.tpm.backend: emulator`: ezkvm can launch swtpm.
- TPM socket path precedence: `--tpm-socket-path` -> `system.tpm.state_path` -> `host_capabilities.tpm.socket_dir/<vm>.swtpm` -> runtime root `<vm>.swtpm`.
- Bridge helper precedence for `backend.type: bridge`: VM `backend.helper` -> `host_capabilities.network.bridge_helper` -> `PATH` `qemu-bridge-helper` -> distro defaults.
- If bridge helper resolution fails (or preferred backend is set to `user`/`user-mode`), ezkvm downgrades the NIC to user-mode and emits a warning.
- `host_capabilities.integrations.remote_viewer.program` or legacy `tools.remote_viewer` + `spice.enabled`: ezkvm can launch remote-viewer.
- VM/profile `options.looking_glass.program` is used unless a CLI override is provided.
- If no CLI or VM/profile value is set, ezkvm falls back to `host_capabilities.integrations.looking_glass.program`, then legacy central `looking_glass.program`, then legacy `tools.looking_glass`.

Supported CLI runtime overrides on `ezkvm start`:

- `--run-dir`
- `--swtpm-binary`
- `--tpm-socket-path`
- `--remote-viewer-program`
- `--looking-glass-program`
- `--ovmf-dir`

Looking Glass client settings should be defined in VM/profile config under `options.looking_glass` so profile-specific behavior stays with the profile.

Preflight behavior:

- Required capability checks (fail fast):
  - QEMU binary availability.
  - swtpm binary availability when `system.tpm.backend: emulator` is configured in `socket` placement mode, except when `system.tpm.state_path` is explicitly provided (externally managed parity socket).
  - OVMF/UEFI firmware file availability when `system.boot.firmware` is `uefi` or `ovmf`.
  - Permission-sensitive runtime and socket parent directories are writable/creatable.
- Optional capability checks (warning-only degradation):
  - Bridge helper fallback warnings for bridge NICs downgraded to user-mode.
  - `/dev/net/tun` accessibility warnings when bridge-helper networking is selected.
  - remote-viewer launcher prerequisites for SPICE workflows.
  - Looking Glass client launcher prerequisites and shared-memory path presence.
- Preflight output order is deterministic across `start` and `start --dry-run`, so failure and warning ordering remains stable.

CLI override impact on preflight checks:

- `--run-dir`: affects runtime and derived socket directory checks.
- `--swtpm-binary`: satisfies swtpm required-binary checks.
- `--tpm-socket-path`: changes TPM socket parent-directory checks.
- `--remote-viewer-program`: affects remote-viewer optional integration checks.
- `--looking-glass-program`: affects Looking Glass optional integration checks.
- `--ovmf-dir`: affects OVMF firmware discovery checks.

## Practical Defaults

- Runtime socket paths are derived from the effective runtime run directory.
- `host_capabilities.runtime.run_dir` overrides legacy `locations.run_dir` for runtime path resolution.
- `host_capabilities.tpm.swtpm_binary` overrides legacy `tools.swtpm`.
- `host_capabilities.integrations.remote_viewer.program` overrides legacy `tools.remote_viewer`.
- Profile files still resolve from `locations.profile_dir`, else `/etc/ezkvm/profiles.d`.
- `locations.vm_dir` is the canonical VM config directory key; legacy `locations.vms_dir` still parses as an alias.

## Compatibility Notes

- Unknown fields in central config are rejected.
- The compatibility fallbacks above apply only inside central config resolution.
- Broader runtime precedence is now implemented and enforced in code for both `start` and `start --dry-run` paths.

## See also

- [VM structure](vm-structure.md)
- [Profiles and merge](profiles-and-merge.md)
- [System and boot](system-and-boot.md)