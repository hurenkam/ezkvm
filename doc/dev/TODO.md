# ezkvm TODO List

## Active Backlog

### Profile System
- [ ] Support multiple profile search directories in priority order

### Network Tooling
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network/stats.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` in `src/network/firewall.rs` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI


## Completed Snapshot

### Architecture Conformance
- [x] Converted targeted production `mod.rs` files to wiring-only module entrypoints
- [x] Extracted moved implementations into focused sibling modules
- [x] Added explicit `mod.rs` wiring-only checklist guidance in `doc/dev/CODING_GUIDELINES.md`
- [x] Co-located config schema types with their impl blocks for `VmConfig`, `BootConfig`, `DriveConfig`, and `HypervConfig`

### Platform And Feature Delivery
- [x] Core VM platform, CLI lifecycle commands, and QEMU command generation
- [x] Profiles merge/load system with validation and test coverage
- [x] QMP hotplug flows for disk and network device add/remove
- [x] Device modeling parity improvements (SPICE/audio/input/USB/XHCI/ivshmem/iSCSI)

### Quality And Parity
- [x] Command/config parity coverage against `input/wakiza/108.cmd`
- [x] Test suite modularization under `tests/integration` and `tests/config`
- [x] Quality gate stabilization (`cargo fmt --all --check`, strict clippy, full test suite)

