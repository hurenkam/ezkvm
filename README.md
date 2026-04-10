# EZKVM

## TLDR

EZKVM is a wrapper around qemu, mainly intended to run proxmox VM's from other
distro's (arch, lmde, ubuntu).
VM's can be configured using yaml, config files reside in /etc/ezkvm
Packages for arch can be built from source, see the arch directory.
Packages for debian/ubuntu can be built from source, see the debian directory.

Pre-built packages for arch and debian can be found for released versions in the releases section

## Proxmox Import

The Proxmox importer defaults remote-viewer style guests to SPICE endpoints.
When storage metadata is not specified explicitly, the importer also assumes `/etc/pve/storage.cfg`.

Example:

```bash
ezkvm --import-proxmox /etc/pve/qemu-server/100.conf --dry-run
```

Override storage metadata explicitly when needed:

```bash
ezkvm --import-proxmox /etc/pve/qemu-server/100.conf --proxmox-storage /path/to/storage.cfg --dry-run
```

## Documentation

- [Status](doc/STATUS.md)
- [Roadmap](doc/ROADMAP.md)

### For users

- [Installation](doc/INSTALLATION.md)
- [Network Bridge](doc/NETWORK_BRIDGE.md)
- [Configuration](doc/CONFIGURATION.md)
- [User Manual](doc/USER_MANUAL.md)

### For developers

- [Building](doc/BUILDING.md)
- [Contributing](doc/CONTRIBUTING.md)
