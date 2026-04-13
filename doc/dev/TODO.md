# ezkvm TODO List

## Active Backlog

### VM Config Schema Restructure
- [x] Design and document the target schema move for `boot`, `tpm`, and `smbios` under `system`, `guest_agent` and `qmp` under `options`, `scsi_controllers` to `controllers.scsi`, `xhci_controllers` to `controllers.xhci`, `hostpci` to `host.pci`, `usb_devices` to `host.usb`, `input_devices` to `devices.input`, `audio_devices` to `devices.audio`, `system.memory` to `system.memory.size`, `ballooning` to `system.memory.ballooning`, and `ivshmem` to `system.memory.ivshmem`, including exact precedence and merge semantics
- [x] Introduce `system.cpu` as a new subsection and move CPU-related fields under it: `model`, `features`, `vcpus`, and `numa`
- [x] Change `system.cpu.features` from list-of-objects (`name`) to list-of-strings, while preserving existing feature ordering and command emission behavior
- [x] Implement loader normalization/compatibility mapping from legacy top-level and legacy nested fields into the new structure before validation (including scalar-to-object memory migration for `system.memory`)
- [x] Keep backward compatibility for existing configs during migration and define clear conflict-resolution rules when both legacy and new paths are present (including mixed old/new lists for controllers, host passthrough, device input/audio, and memory/ballooning/ivshmem fields)
- [x] Update validation logic and error messages for the new field locations, CPU feature representation, and nested memory/ballooning/ivshmem schema
- [x] Update QEMU argument composition to read from the new normalized schema paths only (`system.*`, `options.*`, `controllers.*`, `host.*`, `devices.input`, `devices.audio`, `system.memory.*`)
- [x] Add unit tests for parsing, merge behavior, list append semantics, and precedence across legacy vs new schema layouts
- [x] Add integration tests for command regression parity and mixed legacy/new config compatibility across all moved sections
- [x] Update `doc/user/CONFIG.md` and migration notes to reflect new hierarchy and legacy compatibility behavior
- [x] Update all files under `examples/` to the new schema layout (`system.boot`, `system.tpm`, `system.smbios`, `system.cpu`, `system.memory.size`, `system.memory.ballooning`, `system.memory.ivshmem`, `options.guest_agent`, `options.qmp`, `controllers.scsi`, `controllers.xhci`, `host.pci`, `host.usb`, `devices.input`, `devices.audio`)
- [x] Update packaged files under `etc/` (including profile files) to the new schema layout and verify they remain policy-compatible and profile-merge-safe
- [x] Run full validation gates (`cargo fmt --all --check`, strict clippy, full tests) and perform a final guideline audit on changed files

Schema design reference: [doc/dev/IMPROVED_TARGET_SCHEMA.md](doc/dev/IMPROVED_TARGET_SCHEMA.md)

## Postponed Items

### Profile System
- [ ] Support multiple profile search directories in priority order

### Network Tooling
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network/stats.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` in `src/network/firewall.rs` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI

## Completed Items Summary

### Profile System
- [x] Split `devices.networks[].mode` into structured backend properties and keep a compatibility path for legacy string parsing
- [x] Introduce selector-based profile defaults for drives and networks matched by interface/type/model/backend instead of `id`
- [x] Extend selector-based profile defaults to additional device families beyond drives and networks
- [x] Rework profile list behavior so concrete device instances append by declaration order while defaults come from policy rules rather than id-based patching
- [x] Add placement policies for `addr`, `scsi_id`, and similar counters with configurable start/step values and per-bus or per-controller scopes
- [x] Add collision detection and validation for explicit versus auto-assigned placement values
- [x] Document profile policy precedence, migration rules, and legacy compatibility behavior
- [x] Add unit and integration tests for selector defaults, structured network backends, auto-placement, and legacy compatibility

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

