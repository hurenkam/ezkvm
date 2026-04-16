# ezkvm TASK List

## Current Items

## Postponed Items

### Proxmox Parity: Command Diagnostics
- [ ] Add a parity report command to generate detailed diff between imported config's generated QEMU args and a reference Proxmox command output
- [ ] Would help users understand what features are not yet fully preserved during import

### Proxmox Parity: Runtime Path Compatibility
- [ ] Extend import mapping to handle Proxmox runtime paths and environment-specific configurations  
- [ ] Includes path resolution for firmware, storage, and resource references specific to Proxmox deployment

### Proxmox Parity: Compatibility Profile Mode
- [ ] Implement an opt-in `--proxmox-compat` profile mode that prioritizes Proxmox-specific feature preservation over portable defaults
- [ ] Allows users to generate configs more closely matching original Proxmox behavior when needed for specific use cases

### Profile System
- [ ] Support multiple profile search directories in priority order

### Network Tooling
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network/stats.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` in `src/network/firewall.rs` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI
