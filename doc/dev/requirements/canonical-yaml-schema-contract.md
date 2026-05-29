# Canonical YAML Schema Contract

Status: Draft  
Date: 2026-05-28

## Purpose

Define the canonical YAML structure that represents source-agnostic virtual machine intent in a human-readable, deterministic form.

## Requirement Traceability

- FR-003 Canonical YAML Core Config
- FR-004 Model Separation Pipeline
- FR-006 Validation Before Execution
- NFR-001 Usability
- RDR-002 Typed Internal Contracts
- RDR-006 Profile/Composition Friendly Config

## Schema Contract

1. The top-level document must be YAML mapping data.
2. The canonical document must include these top-level sections:
   - `metadata`
   - `virtual_machine`
3. `virtual_machine` must be source-agnostic and sufficient to describe guest intent.
4. Unknown top-level keys must be rejected unless explicitly marked as extension namespace.

## Required Core Fields

- `metadata.schema_version`: semantic schema version string.
- `metadata.vm_name`: stable VM identifier (must be the same as the basename of the config file)
- `virtual_machine.system.chipset`: canonical chipset identifier.
- `virtual_machine.system.cpu.model`: canonical CPU model intent.
- `virtual_machine.system.memory.min`: integer MiB memory value.

## Validation Rules

1. Required fields must be present and type-correct.
2. Scalar normalization must be deterministic (for example booleans and integers).
3. List fields that influence rendering order must define stable ordering semantics.
4. Validation errors must include field path and reason.

## Composition Rules

- Profile layering should resolve to one effective canonical model before rendering.
- Layering precedence must be explicit and documented by the runtime command.
- Conflicting immutable fields in composed profiles must fail validation.

## Global config file 
`/etc/ezkvm/config.yaml`

```yaml
host: "proxmox9"
```

## Host config file
`/etc/ezkvm/host/proxmox9.yaml`

```yaml
# Proxmox 9 Host
state_path: "/var/run/qemu-server"
qemu:
  network:
    tap:
      script: "/usr/libexec/qemu-server/pve-bridge"
      downscript: "/usr/libexec/qemu-server/pve-bridgedown"
  ovmf:
    path: "/usr/share/pve-edk2-firmware"
    firmware:
      normal: "OVMF_CODE_4M.fd"
      secure: "OVMF_CODE_4M.secboot.fd"
```

## Typical Virtual Machine config file
`/etc/ezkvm/vm/win11-dev.yaml`

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "win11-dev"

# Windows 11 VM
virtual_machine:
  system:
    chipset: "q35"
    bios: { type: "ovmf", secure: true, vars: "/dev/vm1/vm-108-efidisk" }
    cpu: { model: "host" }
    memory: { min: 8192, max: 16384 }
    tpm: { type: "swtpm", state: "/dev/vm1/vm-108-tpmstate" }
    smbios:
      uuid: "04d064c3-66a1-4aa7-9589-f8b3ecf91cd7"
      vm_generation_id: "c13b46f3-9743-49f0-92e8-297b9feac497"
  network:
    - { driver: "virtio-net-pci", type: "tap", mac: "BC:24:11:9F:A0:E4" }
  storage:
    - controller: "pvscsi"
      drives:
        - { interface: "scsi", type: "disk", path: "/dev/vm1/vm-108-boot" }
```

## Notes

- This contract defines shape and semantics, not every optional field.
- Additional field catalogs should remain compatible with this contract.
