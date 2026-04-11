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
- Replace the current placeholder `spice-audio` path with schema-backed audio device generation
- Make `audio_devices` a real parsed/validated config section instead of an inert manual block
- Emit `ich9-intel-hda`, `hda-micro`, `hda-duplex`, and `-audiodev spice,id=spice-backend0`
- Preserve controller bus/address and codec `cad` assignments from the config
- Add validation and tests that compare emitted audio arguments against the Proxmox pattern

### 2. Real Input Device Support
- Make `input_devices` a real parsed/validated config section instead of an inert top-level block
- Emit `virtio-mouse` and `virtio-keyboard` only when configured
- Ensure SPICE/vdagent setup and input-device emission work together without duplicate devices
- Add tests for parsed input devices and emitted QEMU arguments

### 3. XHCI And USB Parity
- Extend the XHCI controller model to support `p2`, `p3`, `bus`, and `addr`
- Add a dedicated XHCI controller config section instead of relying on implicit controller creation
- Support Proxmox-style USB host emission using `hostbus` and `hostport` in addition to the current host spec
- Keep existing USB validation, but add normalization/tests for the Proxmox `1-2.2` form

### 4. ivshmem And Looking Glass Parity
- Extend `ivshmem` config to support `bus` placement and custom `mem_path`
- Change the current default mem-path handling so `/dev/kvmfr0` can be emitted directly from config
- Emit `ivshmem-plain` and `memory-backend-file` with Proxmox-like IDs and placement
- Add tests verifying `bus=pcie.0` and `mem-path=/dev/kvmfr0`

### 5. Non-VFIO Device Placement Parity
- Add bus/address support for guest agent `virtio-serial`, balloon, SCSI controller, and other non-VFIO devices
- Add per-drive attachment options for `bootindex`, `scsi-id`, controller bus placement, and CDROM unit/bus layout
- Support richer drive emission where needed (`if=none` plus matching `-device` attachment) for closer Proxmox parity
- Do not change VFIO bus/address mapping in this plan

### 6. Validation And Comparison Pass
- Add targeted tests for each new config section and argument builder path
- Add a regression test or fixture comparison against the key fragments from `input/wakiza/108.cmd`
- Re-run `validate` and `start --dry-run` on `examples/wakiza.yaml` after each milestone