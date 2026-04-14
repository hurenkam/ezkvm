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

**Negative:**
- Some Proxmox configs may be inexpressible without new canonical fields
- Requires iterative mapper expansion as new fields are discovered
- Import failures block import (no partial mode without --force)

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
```

## Related ADRs
- [ADR-0001: Base Selection](ADR-0001-base-selection.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md) (for extension mappers)

## References

- Current canonical schema: `doc/user/config/vm-structure.md`
- v1 Proxmox parser: `/home/hurenkam/Workspace/ezkvm_v1/src/import/proxmox_parser.rs`
- Backlog task: `doc/backlog/BACKLOG.md` - B-01, B-02
