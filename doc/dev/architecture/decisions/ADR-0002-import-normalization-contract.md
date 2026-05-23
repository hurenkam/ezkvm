# ADR-0002: Import Normalization Contract

**Date:** 2026-04-15  
**Status:** Accepted  
**Context:** ezkvm Incremental Convergence Initiative - Proxmox Import Feature

## Question

How should Proxmox configuration imports be normalized and validated to work within ezkvm's current canonical schema?

## Decision

**All imported Proxmox configs must normalize to current canonical paths.** There is no separate v1-compatible schema for imports; the mapper output must pass current config validation and be runnable directly.

### Import Pipeline

```
Proxmox .conf file
    ↓
[Parser] → Intermediate typed model
    ↓
[Mapper] → Current canonical YAML paths (system.*, devices.*, controllers.*, host.*, options.*)
    ↓
[Validator] → Full current config validation
    ↓
[Output] → Executable canonical YAML
```

## Rationale

1. **Single Source of Truth** - Current canonical schema is the only config structure. No dual paths or compatibility layers.
2. **Schema Simplicity** - Prevents divergence and versioning hell
3. **Clear Failure Points** - Import failures are explicit mismatches between Proxmox and canonical, not schema resolution issues
4. **Field Mapping Clarity** - Every Proxmox field has a canonical destination or is unsupported (reported as warning)
5. **Reuse Current Validation** - Mapper output passes existing config validation without extensions

## Field Mapping Philosophy

### Supported Mapping (becomes canonical)
- `cpu` → `system.cpu.model`
- `cores` → `system.cpu.vcpus` (combined with sockets)
- `memory` → `system.memory.size`
- `hostpci0` → `host.pci[].device`
- `net0` → `devices.networks[]` with backend inference
- `scsi0` → `controllers.scsi[]`
- `scsihw` → auto-select SCSI controller
- `tpm` → `system.tpm`
- `smbios1` → `system.smbios`

### Unsupported Fields (reported as warnings)
- Custom QEMU args not expressible in canonical schema
- Device types not yet supported
- Advanced Proxmox-specific features with no current equivalent

### Conflict Resolution
- If field maps to multiple canonical paths: choose primary path, document secondary as comment
- If Proxmox provides both old and new syntax: prefer new syntax, warn on old
- If canonical has required field, import defaults use sensible values (e.g., `format: qcow2` if not specified)
- If canonical has required field, import defaults use sensible values (e.g., `format: qcow2` if not specified)

### Single Defaults Principle

**There is exactly one set of defaults — the data model and its profiles.**

The import pipeline must not maintain a separate set of defaults that diverge from what the runtime uses. When Proxmox applies a runtime value that is not stored in the `.conf` file (e.g. boot menu, CPU flags, drive cache policy), that value must be supplied by a profile assigned during import — not hardcoded in the mapper — so the same value is applied at both import time and run time.

**Profile responsibilities:**
| Profile | Assigned when | Provides |
|---|---|---|
| `proxmox-base` | Always (all Proxmox imports) | Boot menu settings, `kvm-pit.lost_tick_policy=discard`, SCSI drive tuning (`cache=none`, `aio=io_uring`, `detect-zeroes=unmap`), SCSI id placement (`scope: controller, start: 0`), tap network vhost/queue sizes/scripts |
| `proxmox-windows` | OS type `win10` or `win11` | Proxmox HV CPU flags (`hv_ipi`, `hv_spinlocks=0x1fff`, `kvm=off`, etc.) |
| `proxmox-q35-uefi` | Machine type Q35 + UEFI firmware | `hpet=off`, UEFI firmware path |

Updating a Proxmox runtime default means updating the relevant profile file, not the mapper.

### Device ID Preservation

Drive and network IDs **must** be set from the Proxmox source key (e.g. `disk.key` → `"scsi0"`, `network.key` → `"net0"`). This is required so that boot-order lookup (`"scsi0" → boot_index`) functions correctly at import time.

### Placement Preservation and Precedence

For topology-sensitive PCI devices, import normalization must preserve whether placement was explicit or implicit in source input. This is required for deterministic replay and parity with QEMU runtime behavior.

Placement precedence for normalized output and command-build behavior:

- Explicit `bus`/`addr` in effective config
- Imported explicit `bus`/`addr` from source command/config
- ezkvm placement policy defaults
- QEMU implicit defaults (runtime fallback)

### Import Output Compactness

Mapper output may omit fields that equal the assigned profile defaults — but only because the compaction pass (`compact_profile_owned_fields`) will verify that the profile restores them at runtime. A field must never be silently dropped without a profile guarantee.

## Validation Rules

1. **Pre-validation** - Proxmox .conf structure validation before parsing
2. **Mapping** - Ensure all parsed fields have a destination (or explicit "unsupported")
3. **Post-mapping** - Validate resulting YAML against current schema
4. **Warnings** - Collect all unsupported/partially-supported fields in structured report
5. **Strict Mode** - `--strict` flag causes non-zero exit if warnings present

## Consequences

**Positive:**

- Clean normalization with no schema duplication
- Current validation gates apply immediately to imports
- Debugging import issues uses current config tools
- Future schema improvements apply to imports automatically
- Profile-supplied Proxmox defaults are visible, versioned, and overridable (not hidden in mapper code)
- Changing a Proxmox default means updating a profile file, not hunting through mapper logic

**Negative:**
- Some Proxmox configs may be inexpressible without new canonical fields
- Requires iterative mapper expansion as new fields are discovered
- Import failures block import (no partial mode without --force)
- New Proxmox runtime defaults discovered after initial import require a profile update (not a re-import)

## Example Output

Input (Proxmox `100.conf`):
```
name: ubuntu-22
cpu: host
cores: 4
memory: 8192
hostpci0: 0000:03:00.0,pcie=1
net0: virtio,bridge=vmbr0
```

Output (canonical YAML):
```yaml
name: ubuntu-22
system:
  cpu:
    model: host
    vcpus: 4
  memory:
    size: 8192
host:
  pci:
    - id: pci0
      device: "0000:03:00.0"
      pcie: true
devices:
  networks:
    - id: net0
      model: virtio-net
      backend:
        type: bridge
        bridge: vmbr0

      ## Correctness Validation

      The standard correctness check for an import is a **dry-run diff** against a captured Proxmox reference command:

      ```
      ./target/debug/ezkvm start <vm.yaml> --dry-run | diff - input/<host>/<id>.qemu.cmd
      ```

      Expected intentional differences: QMP sockets, `-daemonize`, binary path (`/usr/bin/kvm` vs `qemu-system-x86_64`), runtime-generated tap `ifname`, and argument ordering. Any other substantive difference is a potential bug.
```

## Related ADRs
- [ADR-0001: Base Selection](ADR-0001-base-selection.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md) (for extension mappers)
- [ADR-0005: Q35 Topology Contract](ADR-0005-q35-topology-contract.md) (for placement semantics and precedence)

## References

- Current canonical schema: `doc/user/config/vm-structure.md`
- v1 Proxmox parser: `/home/hurenkam/Workspace/ezkvm_v1/src/import/proxmox_parser.rs`
- Backlog task: `doc/backlog/BACKLOG.md` - B-01, B-02
