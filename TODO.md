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
- [x] Schema-backed SPICE audio device generation with `audio_devices`
- [x] Schema-backed SPICE input device generation with `input_devices`
- [x] Dedicated XHCI controller config with Proxmox-style USB `hostbus` and `hostport` support
- [x] `ivshmem` bus placement and configurable `mem_path` with Proxmox-like emission
- [x] Non-VFIO device placement support for guest agent, balloon, SCSI controller, and richer drive attachments
- [x] Regression comparison against key fragments from `input/wakiza/108.cmd`
- [x] Dry-run and validation coverage for `examples/wakiza.yaml`
- [x] Reuse of a single `virtio-serial-pci` controller for guest agent and SPICE vdagent

## Open Items
- Keep VFIO bus/address mapping unchanged; no parity work is planned for that area

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
- [x] Stop dropping `DisplayConfig.vram` in `src/config/devices.rs` and emit display-specific properties for supported GPU/display devices
- [x] Extend `SerialConfig` with the fields needed for real `file` and `socket` backends instead of hard-coded placeholder targets like `/dev/null` and `127.0.0.1:4444`
- [x] Add validation for serial backend-specific requirements such as file paths, socket addresses, and port ranges
- [x] Add tests that verify display and serial config sections produce the expected QEMU arguments

### 10. Looking Glass Client Integration
- [ ] Use `ivshmem` and central tool configuration to build real `looking-glass-client` arguments instead of only spawning the binary
- [ ] Validate that auto-started Looking Glass sessions fail clearly when the shared-memory device or client binary is misconfigured
- [ ] Add a dry-run or logging path for auxiliary tool startup so viewer-launch behavior can be verified without starting the VM