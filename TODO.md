# ezkvm TODO List

## Completed Features ✅
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
- [x] Central tool configuration
- [x] Automatic service startup (TPM, remote-viewer, Looking Glass)
- [x] Intel HDA/SPICE audio controller configuration with bus addressing and multiple codecs
- [x] XHCI USB controller support and USB host passthrough
- [x] USB device validation and controller/device enumeration
- [x] Full SPICE display setup with QXL, vdagent, virtio input devices, and ivshmem
- [x] SPICE audio integration
- [x] Advanced QEMU machine, boot, global, RTC, and `nodefaults` options
- [x] Advanced network device options including queue sizing, boot index, and bus/address placement
- [x] Advanced drive options including `cache`, `aio`, and `detect-zeroes`
- [x] iSCSI initiator IQN and authentication support
- [x] Custom PID file management with lifecycle cleanup
- [x] VM-specific logging with log rotation support

## Open Items
- Implement the remaining Proxmox-parity items that cannot be expressed with YAML alone

## Next Implementation Plan

### 1. Real Audio Device Support
- [x] Replace the current placeholder `spice-audio` path with schema-backed audio device generation
- [x] Make `audio_devices` a real parsed/validated config section instead of an inert manual block
- [x] Emit `ich9-intel-hda`, `hda-micro`, `hda-duplex`, and `-audiodev spice,id=spice-backend0`
- [x] Preserve controller bus/address and codec `cad` assignments from the config
- [x] Add validation and tests that compare emitted audio arguments against the Proxmox pattern

### 2. Real Input Device Support
- [x] Make `input_devices` a real parsed/validated config section instead of an inert top-level block
- [x] Emit `virtio-mouse` and `virtio-keyboard` only when configured
- [x] Ensure SPICE/vdagent setup and input-device emission work together without duplicate devices
- [x] Add tests for parsed input devices and emitted QEMU arguments

### 3. XHCI And USB Parity
- [ ] Extend the XHCI controller model to support `p2`, `p3`, `bus`, and `addr`
- [ ] Add a dedicated XHCI controller config section instead of relying on implicit controller creation
- [ ] Support Proxmox-style USB host emission using `hostbus` and `hostport` in addition to the current host spec
- [ ] Keep existing USB validation, but add normalization/tests for the Proxmox `1-2.2` form

### 4. ivshmem And Looking Glass Parity
- [ ] Extend `ivshmem` config to support `bus` placement and custom `mem_path`
- [ ] Change the current default mem-path handling so `/dev/kvmfr0` can be emitted directly from config
- [ ] Emit `ivshmem-plain` and `memory-backend-file` with Proxmox-like IDs and placement
- [ ] Add tests verifying `bus=pcie.0` and `mem-path=/dev/kvmfr0`

### 5. Non-VFIO Device Placement Parity
- [ ] Add bus/address support for guest agent `virtio-serial`, balloon, SCSI controller, and other non-VFIO devices
- [ ] Add per-drive attachment options for `bootindex`, `scsi-id`, controller bus placement, and CDROM unit/bus layout
- [ ] Support richer drive emission where needed (`if=none` plus matching `-device` attachment) for closer Proxmox parity
- [ ] Do not change VFIO bus/address mapping in this plan

### 6. Validation And Comparison Pass
- [ ] Add targeted tests for each new config section and argument builder path
- [ ] Add a regression test or fixture comparison against the key fragments from `input/wakiza/108.cmd`
- [ ] Re-run `validate` and `start --dry-run` on `examples/wakiza.yaml` after each milestone

## Code Review Follow-Ups

### 7. Real QMP Device Hotplug
- [ ] Replace the print-only helpers in `src/device.rs` with real QMP `device_add` and `device_del` flows for disks and network devices
- [ ] Stop using the unused `vm_pid` placeholder in hotplug helpers and resolve a real QMP socket or monitor endpoint from VM state/config
- [ ] Add command/response handling and error reporting for QMP hotplug failures instead of always printing success
- [ ] Add tests around QMP request generation and response parsing for hot-add/hot-remove paths

### 8. Network Tooling Completion
- [ ] Replace the placeholder `get_network_stats` implementation in `src/network.rs` with real parsing of `ip -s link show` output
- [ ] Add tests for network statistics parsing so byte and packet counters are validated from sample command output
- [ ] Remove the hard-coded `eth0` parent from `setup_network_isolation` and make the uplink/interface configurable
- [ ] Expand the CLI network commands beyond bridge creation so the existing network helper functionality is reachable from the CLI

### 9. Display And Serial Argument Completion
- [ ] Stop dropping `DisplayConfig.vram` in `src/config/devices.rs` and emit display-specific properties for supported GPU/display devices
- [ ] Extend `SerialConfig` with the fields needed for real `file` and `socket` backends instead of hard-coded placeholder targets like `/dev/null` and `127.0.0.1:4444`
- [ ] Add validation for serial backend-specific requirements such as file paths, socket addresses, and port ranges
- [ ] Add tests that verify display and serial config sections produce the expected QEMU arguments

### 10. Looking Glass Client Integration
- [ ] Use `ivshmem` and central tool configuration to build real `looking-glass-client` arguments instead of only spawning the binary
- [ ] Validate that auto-started Looking Glass sessions fail clearly when the shared-memory device or client binary is misconfigured
- [ ] Add a dry-run or logging path for auxiliary tool startup so viewer-launch behavior can be verified without starting the VM