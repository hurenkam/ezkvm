# System and Boot

## system

### Required

- `system.architecture` (x86_64, aarch64, x86, ppc64, riscv64)
- `system.machine`
- CPU via canonical `system.cpu.*` (or legacy aliases)
- memory via canonical `system.memory.size` (legacy scalar `system.memory` is accepted)

### CPU

Canonical:

```yaml
system:
  cpu:
    model: "host"
    vcpus: 4
    features:
      - "hv_time"
      - "+kvm_pv_unhalt"
    numa: []
```

Legacy aliases accepted during migration:

- `system.cpu_model`
- `system.vcpus`
- `system.cpu_features` (string entries or `{ name: "..." }`)
- top-level `numa`

### Memory

Accepted forms:

```yaml
system:
  memory: 4096
```

or

```yaml
system:
  memory:
    size: 4096
    ballooning:
      enabled: true
    ivshmem:
      enabled: false
```

Rules:

- `system.memory` object must include `size`.
- scalar/object conflicts resolve to canonical object `size` value.

## system.boot

`system.boot` fields:

- `firmware` (`uefi`, `bios`, `ovmf`)
- `boot_order` (`disk`, `cdrom`, `network`, `hd`, `cd`)
- `menu`, `strict`, `reboot_timeout`, `splash`
- `kernel`, `initrd`, `cmdline`
- `uefi_code`, `uefi_vars`, `uefi_vars_size`, `secure_boot`

## system.tpm

- `version` (`1.2`, `2.0`)
- `backend` (`emulator`, `passthrough`)
- `model` (`tpm-tis`, `tpm-crb`)
- `state_path`, `state_dir`, `state_backend_uri`

## system.smbios

Optional fields:

- `manufacturer`, `product`, `version`, `serial`, `uuid`, `sku`, `family`, `vm_generation_id`

## Legacy Shortcut Support

These are accepted during migration:

- top-level `boot`, `tpm`, `smbios`

They normalize to `system.boot`, `system.tpm`, and `system.smbios` before validation.

## See also

- [VM structure](vm-structure.md)
- [Central config](central-config.md)
- [Platform features and options](platform-features.md)
- [Examples](examples.md)