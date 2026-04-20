---
name: proxmox
description: 'Use when working on Proxmox VE design, VM or LXC workflows, storage or networking integration, migration planning, or Proxmox automation.'
---

# proxmox

**Scope**: Workspace skill for Proxmox-related design, integration, migration, and automation workflows.

## Description

This skill captures a reusable Proxmox expert workflow for engineering tasks that involve Proxmox Virtual Environment (PVE) clusters, VM/LXC management, storage/network configuration, API automation, and migration planning. Focuses on Proxmox VE 9.x with understanding of Proxmox-specific packages vs Debian Trixie generic packages, especially for KVM/QEMU components. Use it when working on repository changes that touch Proxmox concepts, deployment automation, or support guidance.

## Use when

- designing or implementing Proxmox VM or container provisioning
- automating Proxmox operations via REST API, `pvesh`, or SDKs
- mapping existing virtualization workflows to Proxmox primitives
- creating migration guidance from other hypervisors or tools
- troubleshooting Proxmox storage, networking, or HA cluster behavior
- writing docs, examples, or scripts for Proxmox usage
- working with Proxmox configuration files (VM definitions, storage configs, cluster settings)

## Workflow

1. **Clarify the target environment**
   - Identify Proxmox version (focus on VE 9.x features and compatibility)
   - Determine whether workload is KVM, LXC, or storage/network automation
   - Confirm whether the goal is deployment, migration, backup, or monitoring
   - Consider package differences: Proxmox-specific vs Debian Trixie generic packages

2. **Review existing repository context**
   - Look for current virtualization or config automation code
   - Determine whether the task is integration, migration, or documentation
   - Check for any Proxmox-specific references or templates

3. **Understand Proxmox configuration files**
   - **VM definitions**: Located at `/etc/pve/qemu-server/<VMID>.conf` for KVM VMs and `/etc/pve/lxc/<VMID>.conf` for LXC containers
   - **Storage configurations**: Defined in `/etc/pve/storage.cfg` with sections for different storage types (dir, lvm, zfs, ceph, nfs, etc.)
   - **Cluster configuration**: `/etc/pve/corosync.conf` for cluster membership and `/etc/pve/cluster.conf` for cluster settings
   - **Network configuration**: `/etc/network/interfaces` for host networking and bridge definitions
   - Use `qm config <VMID>` or `pct config <VMID>` to view current VM/container configurations

4. **Choose the right integration path**
   - Prefer Proxmox REST API for automation and orchestration
   - Use `pvesh` or `qm`/`pct` CLI for quick admin tasks or scripts
   - Use LXC for lightweight containers and KVM for full VMs
   - Use storage types appropriately: local-lvm, ZFS, Ceph, NFS, or CIFS

5. **Implement safely**
   - Keep changes incremental and reversible
   - Favor idempotent operations and explicit state management
   - Validate API calls and CLI commands before applying changes
   - Preserve version compatibility for Proxmox and underlying storage
   - When modifying config files, backup originals and test changes in non-production first

6. **Validate and document**
   - Test provisioning and configuration against a real or sandbox PVE cluster
   - Confirm that network and storage settings work as expected
   - Update documentation with sample commands, API payloads, or YAML templates
   - Include migration and rollback guidance when applicable
   - Document configuration file changes with before/after examples

## Proxmox VE 9.x and Package Differences

### Proxmox-Specific vs Debian Trixie Packages

**KVM/QEMU Packages:**
- **Proxmox packages**: `pve-qemu-kvm`, `pve-qemu-kvm-8.1+dfsg-2` (or similar versioned packages)
  - Include Proxmox patches for better PVE integration
  - Support for Proxmox-specific features like backup integration, live migration enhancements
  - Optimized for Proxmox's storage backends (ZFS, Ceph, LVM)
- **Debian Trixie packages**: `qemu-system-x86`, `qemu-kvm`, `qemu-utils`
  - Upstream QEMU without Proxmox modifications
  - May lack some Proxmox-specific optimizations
  - Different version numbering and patch levels

**Key Differences:**
- **Storage integration**: Proxmox packages have enhanced support for ZFS, Ceph, and LVM storage pools
- **Live migration**: Proxmox packages include optimizations for cluster migration
- **Backup integration**: Proxmox-specific hooks for vzdump backup system
- **Security**: Proxmox packages may include additional hardening patches
- **Version compatibility**: Proxmox packages are tested specifically with PVE kernel and tools

### Proxmox VE 9.x Features
- **Kernel**: Based on Debian Trixie with Proxmox patches for virtualization and clustering
- **QEMU version**: Typically 8.x series with Proxmox modifications
- **Storage**: Enhanced ZFS support, improved Ceph integration, better NFS performance
- **Networking**: Open vSwitch integration, improved bridge performance
- **Security**: AppArmor profiles, improved container isolation
- **API**: REST API v2 with enhanced automation capabilities

### Package Management Considerations
- **Repository**: Proxmox uses its own repository (`deb https://enterprise.proxmox.com/debian/pve trixie pve-enterprise`)
- **Dependencies**: Proxmox packages may require specific versions of supporting libraries
- **Updates**: Proxmox packages follow PVE release cycle, not Debian's
- **Mixing packages**: Avoid mixing Proxmox and Debian packages for core virtualization components

## Decision points

- **VM vs Container**: Use KVM for full virtualization and LXC for containerized workloads
- **API vs CLI**: Use REST API for automation; CLI is acceptable for ad-hoc or bootstrap tasks
- **Storage backend**: Choose local-lvm/ZFS for performance, Ceph for distributed storage, NFS/CIFS for shared access
- **Network mode**: Use Linux bridges for typical L2 networking, Open vSwitch for advanced SDN, VLANs for segmentation
- **Cluster HA**: Only enable HA after validating node stability and fencing behavior
- **Configuration editing**: Use `qm`/`pct` commands for VM/container configs; edit storage.cfg directly only when necessary; prefer API for programmatic changes
- **Package selection**: Use Proxmox-specific packages for KVM/QEMU when targeting PVE; consider Debian packages only for generic Linux deployments
- **Version compatibility**: Ensure all components are compatible with Proxmox VE 9.x requirements

## Quality criteria

- code and automation work with the intended Proxmox version
- VM/LXC definitions are valid and aligned with PVE schema
- storage and network configuration choices are explicit and documented
- API or CLI calls are tested and error-handled
- documentation includes examples and expected outcomes
- migration guidance identifies equivalent Proxmox constructs
- configuration file changes are validated and include rollback procedures
- VM/container configs follow Proxmox naming conventions and parameter standards
- package selections consider Proxmox-specific vs Debian Trixie differences, especially for KVM/QEMU
- compatibility verified with Proxmox VE 9.x features and requirements

## Example prompts

- `Use the proxmox skill to design a migration path from a QEMU/KVM YAML workflow to Proxmox VM definitions.`
- `Use the proxmox skill to add Proxmox cluster storage recommendations to the README.`
- `Use the proxmox skill to create an automation checklist for provisioning VMs and LXCs on Proxmox.`
- `Use the proxmox skill to troubleshoot and document a Proxmox network bridge configuration.`
- `Use the proxmox skill to explain Proxmox VM configuration file structure and common parameters.`
- `Use the proxmox skill to document how to safely modify Proxmox storage.cfg for new storage pools.`
- `Use the proxmox skill to compare Proxmox-specific KVM packages vs Debian Trixie generic packages.`
- `Use the proxmox skill to document Proxmox VE 9.x new features and migration considerations.`

## Proxmox Configuration Files

### VM and Container Definitions
- **KVM VMs**: `/etc/pve/qemu-server/<VMID>.conf` - Key-value format with VM parameters
- **LXC Containers**: `/etc/pve/lxc/<VMID>.conf` - Similar format with container-specific options
- **Common parameters**: `name`, `memory`, `cores`, `net0`, `scsi0`, `boot`, `ostype`
- **Editing**: Use `qm set <VMID> -key value` or `pct set <VMID> -key value` for safe changes

### Storage Configuration
- **File**: `/etc/pve/storage.cfg` - Defines all storage pools and their properties
- **Format**: INI-style sections with storage type, path, and options
- **Types**: `dir`, `lvm`, `zfs`, `ceph`, `nfs`, `cifs`, `glusterfs`, etc.
- **Management**: Use web UI or `pvesm` commands for storage pool management

### Network Configuration
- **Host networking**: `/etc/network/interfaces` - Standard Debian networking
- **PVE bridges**: `vmbr0`, `vmbr1`, etc. for VM networking
- **VLANs**: Configured via bridge interfaces or Open vSwitch
- **Management**: Use `ifupdown` or `systemd-networkd` for host networking

### Cluster Configuration
- **Corosync**: `/etc/pve/corosync.conf` - Cluster membership and communication
- **Cluster config**: `/etc/pve/cluster.conf` - Additional cluster settings
- **HA**: `/etc/pve/ha/` directory for High Availability configuration
- **Management**: Use `pvecm` commands for cluster operations

### Best Practices
- Always backup configuration files before manual edits
- Use Proxmox tools (`qm`, `pct`, `pvesm`) for configuration changes when possible
- Test configuration changes in a non-production environment first
- Document custom configurations and their purposes
- Keep configuration files version-controlled where appropriate
- Keep configuration files version-controlled where appropriate

## ezkvm Proxmox Import Approach

When working on the ezkvm Proxmox import feature, follow these principles derived from ADR-0002:

### Single Defaults Principle

There is exactly one set of defaults — the ezkvm data model and its profiles. The import mapper must **not** hardcode Proxmox runtime values. Values not stored in Proxmox `.conf` files (e.g. boot menu, `kvm-pit.lost_tick_policy`, drive cache tuning, HV CPU flags) belong in dedicated Proxmox profiles:

| Profile | Assigned when | Covers |
|---|---|---|
| `etc/profiles.d/proxmox-base.yaml` | Always (all Proxmox imports) | Boot menu, `kvm-pit.lost_tick_policy=discard`, SCSI `cache=none/aio=io_uring/detect-zeroes=unmap`, SCSI id placement, tap network vhost/queue sizes/scripts |
| `etc/profiles.d/proxmox-windows.yaml` | OS type `win10` or `win11` | HV CPU flags (`hv_ipi`, `hv_spinlocks=0x1fff`, `kvm=off`, etc.) |
| `etc/profiles.d/proxmox-q35-uefi.yaml` | Machine type Q35 + UEFI | `hpet=off`, UEFI firmware path |

To add a new Proxmox runtime default: update the relevant profile, not the mapper.

### Device ID Preservation

Drive and network IDs **must** be set from the Proxmox source key (e.g. `"scsi0"`, `"net0"`). This preserves boot-order lookup semantics at import time. Do not renumber or rename these IDs.

### Import Compactness

Mapper output should be compact (omit fields equal to profile defaults). A field may only be omitted when the assigned profile guarantees its restoration at runtime. Compaction is handled by the `compact_profile_owned_fields` post-processing pass - the mapper should emit correct values first, not pre-empty fields.

### Correctness Validation

Validate an import by running a dry-run diff against a captured Proxmox reference command:

```sh
./target/debug/ezkvm start <vm.yaml> --dry-run | diff - input/<host>/<id>.qemu.cmd
```

Expected intentional differences: QMP sockets, `-daemonize`, binary path (`/usr/bin/kvm` vs `qemu-system-x86_64`), runtime-generated tap `ifname`, and argument ordering. Any other substantive difference is a potential bug in the mapper or profiles.
