# ezkvm TODO List

## Active Backlog

### Profile System Delivery (Remaining)
- [ ] Support multiple profile search directories in priority order

### Review Comments Remediation Plan (Remaining)
- [ ] Refactor config schema layout so large types and their impl blocks are co-located per type (especially `VmConfig`, `BootConfig`, `DriveConfig`, `HypervConfig`)

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

### Review Concerns Remediation
- [x] Correctness and safety hardening completed across process control, network isolation handling, storage error propagation, IOMMU detection robustness, and TPM error flow
- [x] Documentation and test hygiene updates completed (`doc/user/CONFIG.md` sync, tmp artifact isolation, environment mutation locking)
- [x] Quality gates stabilized (`cargo fmt --all --check`, strict clippy, full test suite)
- [x] Major modularization completed for config and CLI surfaces, including split validators and config loader helper modules
- [x] Section 13 cleanup substantially completed: `src/qemu/mod.rs` split, loader files grouped, and schema conversion impls co-located with source types where targeted
- [x] Section 2 threshold remediation completed for the planned scope: oversized modules split, orchestration refactors applied, and large structs justified where kept intact
- [x] Test-maintainability and parity follow-up completed: oversized suites split into focused modules, regression coverage added for command/config parity, and residual audit findings captured in `REVIEW_COMMENTS.md`

### QMP Device Hotplug
- [x] Implemented real QMP hotplug flows for disks and network devices (`device_add`, `device_del`, `blockdev-add`, `blockdev-del`, `netdev_add`, `netdev_del`)
- [x] Replaced placeholder `vm_pid` usage with PID-based QMP socket resolution
- [x] Added QMP command/response error handling and request/response parser tests
