# Phase 2 Move Map: User and Community Documentation

Date: 2026-05-23
Owner: Documentation maintainers
Related backlog ticket: L-03 (depends on L-02)

## Purpose

Provide a per-file migration map for Phase 2 (user/community split), based on the current inventory.

## Source Inventory Snapshot

User docs currently under `doc/user/`:

- `doc/user/UBUNTU_NETPLAN_BRIDGE.md`
- `doc/user/config/README.md`
- `doc/user/config/central-config.md`
- `doc/user/config/code-backed-shapes.md`
- `doc/user/config/devices.md`
- `doc/user/config/examples.md`
- `doc/user/config/field-reference.md`
- `doc/user/config/import-proxmox.md`
- `doc/user/config/import-qemu-cmd.md`
- `doc/user/config/platform-features.md`
- `doc/user/config/profiles-and-merge.md`
- `doc/user/config/system-and-boot.md`
- `doc/user/config/troubleshooting.md`
- `doc/user/config/vm-structure.md`

Participation/contribution source docs:

- `doc/dev/CONTRIBUTING.md`

## Target Paths and Actions

| Source | Target | Action | Notes |
|---|---|---|---|
| `doc/user/UBUNTU_NETPLAN_BRIDGE.md` | `doc/user/how-to/networking/ubuntu-netplan-bridge.md` | move | Keep operator-focused troubleshooting/how-to content in user area |
| `doc/user/config/README.md` | `doc/user/reference/config/README.md` | move | Becomes config reference entrypoint |
| `doc/user/config/central-config.md` | `doc/user/reference/config/central-config.md` | move | Reference doc |
| `doc/user/config/code-backed-shapes.md` | `doc/user/reference/config/code-backed-shapes.md` | move | Keep as advanced reference |
| `doc/user/config/devices.md` | `doc/user/reference/config/devices.md` | move | Reference doc |
| `doc/user/config/examples.md` | `doc/user/how-to/config/examples.md` | move | Practical usage/how-to |
| `doc/user/config/field-reference.md` | `doc/user/reference/config/field-reference.md` | move | Full field reference |
| `doc/user/config/import-proxmox.md` | `doc/user/how-to/import/proxmox.md` | move | Procedural operator guide |
| `doc/user/config/import-qemu-cmd.md` | `doc/user/how-to/import/qemu-cmd.md` | move | Procedural operator guide |
| `doc/user/config/platform-features.md` | `doc/user/reference/config/platform-features.md` | move | Reference doc |
| `doc/user/config/profiles-and-merge.md` | `doc/user/reference/config/profiles-and-merge.md` | move | Reference doc |
| `doc/user/config/system-and-boot.md` | `doc/user/reference/config/system-and-boot.md` | move | Reference doc |
| `doc/user/config/troubleshooting.md` | `doc/user/troubleshooting/config.md` | move | Consolidate troubleshooting area |
| `doc/user/config/vm-structure.md` | `doc/user/reference/config/vm-structure.md` | move | Foundational reference |
| `doc/dev/CONTRIBUTING.md` | `doc/community/contribution-guide/README.md` | split+move | Keep contribution workflow in community; retain temporary stub at old path |

## Planned New Directories (Phase 2)

- `doc/user/how-to/`
- `doc/user/how-to/config/`
- `doc/user/how-to/import/`
- `doc/user/how-to/networking/`
- `doc/user/reference/`
- `doc/user/reference/config/`
- `doc/user/troubleshooting/`
- `doc/community/contribution-guide/`

## Stub and Link Policy for Phase 2 Moves

For each moved file:

1. Leave a forwarding stub at old path with pointer to new path.
2. Update links in:
   - `doc/README.md`
   - `doc/user/**`
   - `README.md` and `.github/**` where applicable
3. Keep stubs until Phase 6 link/reference verification passes.

## Out of Scope for Phase 2

- No movement of architecture/workflow/domain/analysis docs (Phase 3).
- No backlog model migration (Phase 4).
- No helper instruction rewiring (Phase 6).

## Batch Execution Log

### Batch 1 (2026-05-23)

Moved documents:
- `doc/user/config/examples.md` -> `doc/user/how-to/config/examples.md`
- `doc/user/config/import-proxmox.md` -> `doc/user/how-to/import/proxmox.md`
- `doc/user/config/import-qemu-cmd.md` -> `doc/user/how-to/import/qemu-cmd.md`

Transition actions applied:
- forwarding stubs added at each old path
- `doc/user/config/README.md` links updated to new canonical locations
- moved documents relinked for correct relative references

### Batch 2 (2026-05-23)

Moved document:
- `doc/dev/CONTRIBUTING.md` -> `doc/community/contribution-guide/README.md`

Transition actions applied:
- forwarding stub added at old path `doc/dev/CONTRIBUTING.md`
- `doc/community/README.md` updated to point to new canonical contribution guide path
- `doc/README.md` updated with direct contribution-guide entrypoint