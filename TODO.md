# ezkvm TODO List

## Active Backlog

### Profile System Delivery
- [ ] Support multiple profile search directories in priority order

### Review Comments Remediation Plan (Open)

#### Phase 1: Section 13 Structural Compliance (High Priority)
- [x] Split implementation out of `src/qemu/mod.rs` so `mod.rs` contains only module declarations and re-exports
- [x] Create focused QEMU modules for manager lifecycle and argument builders (for example `manager.rs`, `command_builder.rs`, `boot_args.rs`) and move logic from `src/qemu/mod.rs`
- [x] Move schema conversion impls into the same file as source struct definitions in `src/config/vm_schema.rs` (`From<DriveConfig>`, `From<NetworkConfig>`, `From<DisplayConfig>`, `From<SerialConfig>`)
- [x] Move `impl From<SystemConfig>` into the same file as `SystemConfig` and keep source-type behavior co-located
- [ ] Refactor config schema layout so large types and their impl blocks are co-located per type (especially `VmConfig`, `BootConfig`, `DriveConfig`, `HypervConfig`)
- [x] Group loader-family files into a `src/config/loader/` subdirectory and keep `mod.rs` as the family entrypoint

#### Phase 2: Section 2 Threshold Remediation (Medium Priority)
- [x] Re-baseline current Section 2 outliers and lock the target list in TODO (updated from latest line-count audit)
- [x] Split `src/qemu/args.rs` into focused submodules (`src/qemu/args/mod.rs`, `basic.rs`, `storage.rs`, `devices.rs`, `display.rs`, `system.rs`, `tests.rs`)
- [x] Split oversized production modules (>250 lines): `src/cli/runtime/auxiliary/swtpm.rs`, `src/cli.rs`, `src/config/platform.rs`, `src/config/vm_schema.rs`, `src/qemu/process.rs`, `src/network.rs`, `src/storage.rs`, `src/state.rs` into focused submodules/directories (`src/cli/runtime/auxiliary/swtpm/`, `src/cli/`, `src/config/platform/`, `src/config/vm_schema/`, `src/qemu/process/`, `src/network/`, `src/storage/`, `src/state/`)
- [x] Split `src/cli/runtime.rs` into focused submodules (`src/cli/runtime/mod.rs`, `src/cli/runtime/start.rs`, `src/cli/runtime/ops.rs`, `src/cli/runtime/inspect.rs`, `src/cli/runtime/auxiliary/mod.rs`, `src/cli/runtime/auxiliary/launch.rs`, `src/cli/runtime/auxiliary/swtpm/`)
- [x] Split `src/qemu/command_builder.rs` into focused submodules (`src/qemu/command_builder/mod.rs` + `src/qemu/command_builder/composition.rs`) and move `build_command` to orchestration style
- [x] Split `src/config/validation/platform.rs` into focused submodules (`src/config/validation/platform/mod.rs`, `audio.rs`, `core.rs`, `usb.rs`, `devices.rs`, `helpers.rs`)
- [x] Split `src/config/validation/devices.rs` into focused submodules (`src/config/validation/devices/mod.rs`, `drive.rs`, `network.rs`, `display.rs`, `serial.rs`)
- [x] Split oversized test-support modules in `src` (>250 lines): `src/config/tests.rs`, `src/qemu/args/tests.rs`, `src/cli/tests.rs` (move helper fixtures/assertions into submodules)
- [x] Refactor long command-construction functions to orchestration style (<=35 lines where practical): `build_boot_args`, `build_option_args`, `add_usb_host`
- [x] Refactor `build_command`, `add_tpm`, and `add_spice` to orchestration style with extracted helpers
- [x] Refactor validation paths `validate_drive_config` and `validate_audio_devices` to orchestration style with focused helpers
- [x] Refactor long runtime command handlers to orchestration style (<=35 lines where practical): `handle_start`, `start_swtpm_if_configured`, `handle_storage`, and follow-on helpers in `src/cli/runtime/start.rs`, `src/cli/runtime/auxiliary/swtpm/startup.rs`, and `src/cli/commands.rs`
- [x] Refactor long validation paths to orchestration style (<=35 lines where practical): remaining validator outliers found during implementation
- [x] Re-run function-length scan for `src/**` and create a short residual-outlier list directly in TODO before Phase 2 closeout
	Residual function-length outliers (current shortlist):
	- none currently from the shortlist (all identified function-length outliers refactored to orchestration style)
- [x] Struct-size follow-up: split or justify >35-line structs with concise comments (`VmConfig`, `BootConfig`, `DriveConfig`, `HypervConfig`, `QemuArgs`)

#### Phase 3: Tests, Verification, And Reporting
- [ ] Keep test files maintainable by splitting oversized suites: `tests/config_tests.rs`, `tests/integration_tests.rs`, `src/config/tests.rs`
- [ ] Add/adjust regression tests around refactored QEMU command building and config conversion behavior to preserve output parity
- [x] Run and keep green: `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --quiet`
- [ ] Re-run Section 2 and Section 13 audit across all Rust files and update `REVIEW_COMMENTS.md` with residual items only

### QMP Device Hotplug
- [ ] Replace the print-only helpers in `src/device.rs` with real QMP `device_add` and `device_del` flows for disks and network devices
- [ ] Stop using the unused `vm_pid` placeholder in hotplug helpers and resolve a real QMP socket or monitor endpoint from VM state/config
- [ ] Add command/response handling and error reporting for QMP hotplug failures instead of always printing success
- [ ] Add tests around QMP request generation and response parsing for hot-add/hot-remove paths

### Network Tooling Completion
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network/stats.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` in `src/network/firewall.rs` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI


## Completed Work (Consolidated) ✅

### Core VM Platform
- [x] Basic VM configuration and QEMU command generation
- [x] YAML configuration schema
- [x] CLI interface with start/stop/status commands
- [x] UEFI boot support
- [x] TPM emulator support
- [x] VFIO PCI passthrough
- [x] Network configuration (user/bridge/tap modes)
- [x] SCSI storage with pvscsi controller
- [x] Guest agent support
- [x] Memory ballooning
- [x] SMBIOS system information
- [x] Hyper-V enlightenments
- [x] QMP monitoring

### Tooling And Runtime Orchestration
- [x] Central tool configuration
- [x] Automatic auxiliary startup (TPM, remote-viewer, Looking Glass)
- [x] VM-specific logging with log rotation support
- [x] Custom PID file management with lifecycle cleanup

### Device Modeling And Placement
- [x] Full SPICE display setup with QXL, vdagent, virtio input devices, and ivshmem
- [x] SPICE audio integration
- [x] Intel HDA/SPICE audio controller config with bus addressing and multiple codecs
- [x] Schema-backed `audio_devices` and `input_devices`
- [x] XHCI USB controller support and USB host passthrough
- [x] USB device validation and controller/device enumeration
- [x] Dedicated XHCI config with Proxmox-style `hostbus`/`hostport` support
- [x] Non-VFIO device placement support for guest agent, balloon, SCSI controller, and richer drive attachments
- [x] `ivshmem` bus placement and configurable `mem_path` with Proxmox-like emission
- [x] Advanced network device options (queue sizing, boot index, bus/address placement)
- [x] Advanced drive options (`cache`, `aio`, `detect-zeroes`)
- [x] iSCSI initiator IQN and authentication support

### Parity And Quality
- [x] Advanced QEMU machine, boot, global, RTC, and `nodefaults` options
- [x] Reuse of a single `virtio-serial-pci` controller for guest agent and SPICE vdagent
- [x] Display and serial argument completion with backend-specific validation and tests
- [x] Looking Glass client integration with validation and dry-run visibility
- [x] Regression comparison against key fragments from `input/wakiza/108.cmd`
- [x] Dry-run and validation coverage for `examples/wakiza.yaml`

### Profiles System Delivery
- [x] Profile references in VM config (`profiles`) with central profile directory support (`locations.profile_dir`)
- [x] Default profile directory fallback to `/etc/ezkvm/profiles.d`
- [x] Deterministic merge order: base -> profiles in listed order -> VM file
- [x] Merge semantics: scalar replace, deep map merge, path-aware list behaviors
- [x] ID-based list merges for `devices.drives`, `devices.networks`, `hostpci`, `usb_devices`, `scsi_controllers`, `xhci_controllers`, `audio_devices`
- [x] Append-unique list merges for `system.cpu_features`, `system.machine_options`, `options.global_options`
- [x] Validation/UX additions: `validate --show-resolved-config`
- [x] Unit and integration coverage for profile loading, precedence, error handling, and compatibility

### Review Concerns Remediation (Phases A-D)
- [x] Correctness and safety hardening completed across process control, network isolation handling, storage error propagation, IOMMU detection robustness, and TPM error flow
- [x] Documentation and test hygiene updates completed (`CONFIG.md` sync, tmp artifact isolation, environment mutation locking)
- [x] Quality gates stabilized (`cargo fmt --all --check`, strict clippy, full test suite)
- [x] Major modularization completed for config and CLI surfaces, including split validators and config loader helper modules
