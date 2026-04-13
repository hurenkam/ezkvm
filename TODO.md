# ezkvm TODO List

## Active Backlog

### Profile System Delivery
- [ ] Support multiple profile search directories in priority order

### Review Concerns Remediation Plan

#### Phase A: Correctness And Safety
- [x] Rework VM process discovery/stop/kill in `src/qemu/process.rs` to avoid regex pattern ambiguity and prefer exact process targeting
- [x] Add targeted tests for VM stop/kill process selection, including overlapping names and unsafe-character name cases
- [x] Harden `setup_network_isolation` in `src/network.rs` to ignore only the explicit "already exists" case and error on other failures
- [x] Replace production-path `unwrap` usage in `src/storage.rs` path handling with fallible error propagation and contextual `anyhow` errors
- [x] Replace fragile IOMMU detection shell-grep logic in `src/device.rs` with robust lowercase token checks and non-panicking flow
- [x] Prepare proposal for changing `add_tpm` panic behavior in `src/qemu/args.rs` to `Result` (signature changed, all integrated, tests updated)

#### Phase B: Documentation And Test Hygiene
- [x] Update `CONFIG.md` to reflect current profile merge behavior exactly (id-based list merges, append-unique list merges, and list-replace fallback paths)
- [x] Replace fixed `/tmp` test artifacts with unique temporary paths in integration/config tests
- [x] Apply consistent environment-variable locking strategy across tests that mutate process environment

#### Phase C: Quality Gate Cleanup
- [x] Make `cargo fmt --all --check` pass across repository
- [x] Reduce and resolve current `cargo clippy --all-targets --all-features -- -D warnings` failures in staged batches
- [x] Re-run full `cargo test` after lint/format remediations and keep suite green

#### Phase D: Module Refactor Program
- [x] Split `src/config/mod.rs` into focused modules (schema types, profile loading/merge, env substitution, entrypoint wiring) — complete (`src/config/loader.rs` orchestrator with `src/config/loader_env.rs` and `src/config/loader_merge.rs`, plus `src/config/central.rs`, `src/config/vm_options.rs`, `src/config/platform.rs`, `src/config/vm_schema.rs`, `src/config/entrypoint.rs`, and `src/config/tests.rs` extracted)
- [x] Split `src/config/validation.rs` into domain validators (system, boot, devices, vm options) with small function surfaces
- [x] Split `src/cli.rs` command handlers into submodules by command group while keeping top-level dispatch minimal
- [x] Review `src/qemu/builder.rs` usage and either integrate it as the command-building path or remove/deprecate it

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
