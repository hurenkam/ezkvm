# Proxmox Profile Template

This file documents the structure of Proxmox-specific profiles that capture runtime defaults not stored in Proxmox `.conf` files.

## Location

See `etc/profiles.d/proxmox-base.yaml` and `etc/profiles.d/proxmox-q35-uefi.yaml` for actual implementations.

## Key Sections

### Machine Type + Firmware

Proxmox uses `q35` machine type with UEFI for most modern VMs:

```yaml
name: proxmox-q35-uefi
labels:
  import-source: proxmox
  variant: q35-uefi

system:
  machine: q35
  boot:
    firmware: uefi
  acpi: true
  smbios_mode: dmidecode
```

### Network Policies (for tap-backed NICs)

Proxmox assigns NICs to standard PCI slots. This policy ensures every imported tap-backed NIC gets the same placement as Proxmox would assign:

```yaml
policies:
  networks:
    - match:
        backend_type: tap
      defaults:
        bus: pci.0
      placement:
        addr:
          scope: bus
          bus: pci.0
          start: "0x12"    # First NIC at 0x12 (Proxmox standard)
          step: 1           # Each additional NIC gets +1: 0x13, 0x14, ...
```

**Rationale**: Windows and some Linux guests treat different PCI slots as different adapters. Keeping the same slot avoids driver reassignment and network loss.

### Serial Controllers (for guest-agent, SPICE, etc.)

Guest-agent is typically pinned in Proxmox imports. SPICE vdagent gets its own controller:

```yaml
# No explicit serial config needed here; handled by mapper and args generation
# See src/qemu/args/guest_agent.rs for the conditional logic
```

### Audio and Display

Proxmox uses SPICE for remote display and Looking Glass support:

```yaml
display:
  type: spice
  port: 5900

audio:
  type: spice
```

(Actual audio/display config may be in separate profiles like `looking-glass.yaml`)

### Storage Policies

Storage backends (scsi, virtio) use standard controller placements:

```yaml
policies:
  storage:
    - match:
        interface: scsi
      defaults:
        bus: scsi0  # Proxmox standard SCSI controller ID
```

## Full Example Profile

```yaml
---
name: proxmox-base
labels:
  import-source: proxmox
  description: "Runtime defaults for Proxmox VE 9.x VMs not stored in .conf files"

system:
  machine: q35
  boot:
    firmware: uefi
  acpi: true
  smbios_mode: dmidecode
  halt_signal: acpi  # ACPI power button by default

cpu:
  cores: 4
  sockets: 1
  type: host

memory:
  amount_gb: 8

policies:
  networks:
    - match:
        backend_type: tap
      defaults:
        bus: pci.0
      placement:
        addr:
          scope: bus
          bus: pci.0
          start: "0x12"
          step: 1

  storage:
    - match:
        interface: scsi
      defaults:
        bus: scsi0

display:
  type: spice
  port: 5900

audio:
  type: spice

# Inherit base profile
features:
  - name: guest-agent
    enabled: true
```

## When to Create a New Profile

Create a profile when a group of Proxmox VMs share common runtime settings:

- **proxmox-base.yaml**: Core defaults (network placement, machine type, audio)
- **proxmox-q35-uefi.yaml**: UEFI-specific (firmware, boot order)
- **proxmox-windows-10.yaml**: Windows 10 specifics (CPU, memory, drivers)
- **proxmox-linux-l26.yaml**: Linux-specific (console, virtio tuning)

## Profile Inheritance

Profile inheritance allows stacking:

```yaml
# In a specific VM config:
profiles:
  - proxmox-base           # Provides core network/storage policies
  - proxmox-q35-uefi       # Adds UEFI boot
  - looking-glass          # Adds GPU passthu, SPICE tuning
  - windows-11-drivers    # Adds Windows-specific device config
```

Each profile layers defaults; later profiles can override earlier ones.

## Related Files

- **Proxmox base profile**: `etc/profiles.d/proxmox-base.yaml`
- **UEFI profile**: `etc/profiles.d/proxmox-q35-uefi.yaml`
- **Profile schema**: `src/config/vm_schema/profile_schema.rs`
- **Profile merge logic**: `src/config/merge.rs`
- **Integration tests**: `tests/integration/proxmox_import.rs` (validates profile application)
