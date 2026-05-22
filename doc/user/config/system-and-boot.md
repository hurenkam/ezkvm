# System and Boot

## system

### Required

- `system.architecture` (x86_64, aarch64, x86, ppc64, riscv64)
- `system.machine`
- CPU via `system.cpu.*`
- memory via `system.memory.size`

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

### Memory

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

## system.machine_layout

Optional hierarchy-first machine topology model.

- Parent bus is implied by nesting (`buses -> devices -> buses`).
- Use this when you need explicit, deterministic topology representation in config.

Hierarchy-first form:

```yaml
system:
  machine_layout:
    buses:
      - id: pcie.0
        devices:
          - id: ich9-pcie-port-1
            driver: pcie-root-port
            addr: "1c.0"
            buses:
              - id: ich9-pcie-port-1.0
                devices:
                  - id: hostpci0
                    driver: vfio-pci
                    addr: "0x0.0"
```

Compatibility normalization form:

- `system.machine_layout.nodes` accepts a flat node list with `parent_id` links.
- ezkvm normalizes this into the hierarchy-first `buses` tree before validation.
- Validation rejects duplicate IDs, cycles, and broken `parent_id` links.

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

## See also

- [VM structure](vm-structure.md)
- [Central config](central-config.md)
- [Platform features and options](platform-features.md)
- [Examples](examples.md)