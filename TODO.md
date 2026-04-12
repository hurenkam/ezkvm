# ezkvm TODO List

## Active Backlog

### QMP Device Hotplug
- [ ] Replace the print-only helpers in `src/device.rs` with real QMP `device_add` and `device_del` flows for disks and network devices
- [ ] Stop using the unused `vm_pid` placeholder in hotplug helpers and resolve a real QMP socket or monitor endpoint from VM state/config
- [ ] Add command/response handling and error reporting for QMP hotplug failures instead of always printing success
- [ ] Add tests around QMP request generation and response parsing for hot-add/hot-remove paths

### Network Tooling Completion
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI

### Profiles Feature Plan

#### Phase 1: Schema And Loader Wiring (MVP)
- [x] Add optional `profiles: Vec<String>` to VM config parsing input (without breaking existing files)
- [x] Add `locations.profile_dir: Option<String>` to central config
- [x] Use default profile directory `/etc/ezkvm/profiles.d` when `locations.profile_dir` is unset
- [x] Load VM YAML as raw value, then resolve and merge referenced profile files before `VmConfig` deserialization
- [x] Resolve profile names to files (for example `windows_11` -> `<profile_dir>/windows_11.yaml`)
- [x] Return clear errors for unknown profile names, missing files, unreadable files, or non-map YAML roots

#### Phase 2: Merge Semantics
- [x] Implement deterministic merge order: base -> profiles in listed order -> VM file
- [x] Implement scalar replace and deep map merge
- [x] Implement MVP list behavior as full replacement
- [x] Document merge precedence and list semantics in user docs

#### Phase 3: Validation, CLI Visibility, And Tests
- [x] Keep existing `VmConfig` validation unchanged after merge, adding only pre-deserialize profile resolution checks
- [x] Add `ezkvm config resolve <vm.yaml>` or `--show-resolved-config` to inspect final merged config
- [x] Add unit tests for profile load/merge order, VM override precedence, and error paths
- [x] Add integration tests proving profile-based configs and legacy non-profile configs both work

#### Phase 4: Optional Enhancements
- [ ] Add id-based merge for selected object lists (`devices.drives`, `devices.networks`, `hostpci`, `usb_devices`, `scsi_controllers`, `xhci_controllers`, `audio_devices`)
- [ ] Add append-unique behavior for option/feature lists (`system.cpu_features`, `system.machine_options`, `options.global_options`)
- [ ] Consider supporting multiple profile search directories in priority order

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