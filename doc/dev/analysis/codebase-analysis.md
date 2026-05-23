# ezkvm Codebase Analysis

**Project:** Easy KVM Virtual Machine Manager  
**Language:** Rust (2024 edition)  
**Purpose:** A direct QEMU/KVM alternative to libvirt using YAML configuration files

---

## 1. Core Architecture Description

### Module Structure

ezkvm follows a **layered modular monolith** architecture with distinct responsibilities:

```
src/
├── main.rs          # Binary entrypoint, parses CLI and delegates to execute()
├── lib.rs           # Public module exports
├── cli/             # Command parsing and orchestration boundary
├── config/          # Schema definitions, YAML loading, profile merging, validation
├── qemu/            # QEMU command generation and process lifecycle management
├── device.rs        # Hot-plug and device passthrough runtime operations (QMP-based)
├── network/         # Network helpers, bridge management, firewall rules
├── storage/         # Disk operations, snapshots via qemu-img
└── state/           # Runtime state: PID files, logs, config caching
```

### Design Patterns & Principles

**1. Layered Flow (Unidirectional)**
- `CLI` → `Config` → `QEMU`/`Device`/`Storage`/`Network` → OS/QEMU
- Each layer exposes typed, coarse-grained interfaces
- No upward call-backs; side effects isolated at component boundaries

**2. Strong Typing Over Maps**
- Configuration types (`VmConfig`, `DriveConfig`, etc.) use serde-based YAML deserialization
- `QemuArgs` newtype wrapper enforces type safety for command building
- Explicit domain types reduce runtime errors and improve IDE discoverability

**3. Fail-Fast Validation**
- Configuration is validated immediately after deserialization
- QEMU binary availability checked before process execution
- Path creation and socket setup validated in dry-run mode

**4. Deterministic Command Generation**
- QEMU commands generated from typed config structs via trait `From<T> for QemuArgs`
- Enables regression testing against expected command strings

**5. Builder & Generic From/Into Patterns**
- `SystemConfig` → `QemuArgs` via `From` trait
- `DriveConfig`, `NetworkConfig`, `DisplayConfig` each implement `From<T> for QemuArgs`
- Composable and chainable argument construction

**6. Trait-Based Abstraction**
- Serde traits for YAML round-tripping
- Custom trait implementations for command emission (e.g., `From<DriveConfig>`)
- Error handling using `anyhow::Result<T>` throughout

---

## 2. Configuration System

### Two-Tier Config Model

#### **Central Configuration** (`/etc/ezkvm/ezkvm.yaml` or `$EZKVM_CONFIG`)

Defines tool locations and runtime defaults:

```yaml
tools:
  swtpm: /usr/bin/swtpm
  remote_viewer: /usr/bin/remote-viewer
  looking_glass: /opt/looking-glass-client

locations:
  run_dir: /var/run/ezkvm
  ovmf_dir: /usr/share/OVMF
  vm_dir: /var/lib/ezkvm
  profile_dir: /etc/ezkvm/profiles.d
```

**Loading:** Via `CentralConfig::load()` → checks `EZKVM_CONFIG` env var, then default paths  
**Fallback:** Defaults to empty config if no file found (tolerant)

#### **VM Configuration** (Per-VM YAML file)

```yaml
name: "ubuntu-22.04"
backend: "qemu"
profiles: ["gpu-passthrough", "looking-glass"]
system: { ... }
devices: { ... }
controllers: { ... }
host: { ... }
spice: { ... }
options: { ... }
```

### Profile System

**Profile Resolution:**
1. Extract `profiles: [name1, name2]` from VM config
2. Resolve each profile from `profile_dir` (default `/etc/ezkvm/profiles.d`)
3. Load and merge profiles in order with deep YAML merge
4. Overlay VM config on top of merged profiles
5. Apply merge policies and validate final config

**Profile Merge Logic** (`src/config/loader/merge.rs`):
- **ID-based merge** (lists): `host.pci`, `host.usb`, `controllers.scsi/xhci`, `devices.audio`
  - Matches items by `id` field, updates or appends
- **Append-unique** (lists): `system.cpu.features`, `system.machine_options`
  - Appends new items, removes duplicates
- **Append-all** (lists): `devices.drives`, `devices.networks`, `devices.displays`
  - Concatenates all items from profile and VM config
- **Recursive merge** (mappings): All other YAML maps
  - Deep merge with VM config values overriding profile values

**Example Flow:**
```yaml
# Profile 1: gpu-passthrough.yaml
host:
  pci:
    - id: gpu
      device: "0000:03:00.0"
      x_vga: true

# VM config
profiles: ["gpu-passthrough"]
host:
  pci:
    - id: gpu2
      device: "0000:04:00.0"

# Result after merge
host:
  pci:
    - id: gpu
      device: "0000:03:00.0"
      x_vga: true
    - id: gpu2
      device: "0000:04:00.0"
```

### YAML Schema Organization

**Canonical (Authoritative) Paths:**
- `system.cpu.*` (model, vcpus, features, numa)
- `system.memory.size`, `system.memory.ballooning`, `system.memory.ivshmem`
- `system.boot` (firmware, boot_order, kernel, initrd, cmdline, uefi_vars)
- `system.tpm`, `system.smbios`
- `devices.{drives, networks, displays, serials, input, audio}`
- `controllers.{scsi, xhci}`
- `host.{pci, usb}`
- `options.{guest_agent, qmp, looking_glass}`

**Top-Level Optional:**
- `spice` (SPICE display configuration)
- `hyperv` (Hyper-V enlightenments)
- `iscsi_disks` (iSCSI storage)

### Configuration Loading Pipeline

```rust
VmConfig::from_file(path)
  ├─ load_vm_value_from_file(path)
  │   ├─ Read YAML file
  │   └─ substitute_env_vars() - expand ${VAR_NAME} syntax
  ├─ extract_profile_names(vm_value) - get profiles list
  ├─ resolve_profile_dir() - check PROFILE_DIR
  ├─ build_merged_vm_value()
  │   ├─ Load and merge each profile in order
  │   └─ Merge VM config on top
  ├─ deserialize_and_validate(merged_value)
  │   ├─ apply_profile_policies() - post-merge transforms
  │   ├─ serde_yaml::from_value() - deserialize to VmConfig
  │   ├─ assign_default_device_ids() - auto-generate missing IDs
  │   └─ validate_config() - comprehensive validation
  └─ Return VmConfig
```

**Environment Variable Substitution:**
- Format: `${VAR_NAME}` or `$VAR_NAME`
- Applied to entire config text before YAML parsing
- Example: `memory: ${VM_MEMORY}` → `memory: 2048` (if `VM_MEMORY=2048`)

### Validation System

**Comprehensive Validation** (`src/config/validation.rs`):

1. **Backend validation** - Only "qemu" supported
2. **System config** - Architecture, machine type, memory, CPU
3. **Boot config** - Firmware type, boot order
4. **Device config** - Drives, networks, displays, serials
5. **Platform features** - TPM, SPICE, Hyper-V, ivshmem, guest agent, QMP
6. **Collections** - PCI/USB passthrough, XHCI controllers, SCSI controllers, audio/input devices
7. **VM options** - PID file, log directories, KVM flags

Returns descriptive `anyhow::Error` with context about validation failures.

---

## 3. VM Device Support

### Storage Devices (Drives)

**Interfaces:** `virtio`, `scsi`, `ide`, `nvme`  
**Types:** `disk`, `cdrom`  
**Formats:** `qcow2`, `raw`, others supported by QEMU

**Configuration Fields:**
- `id`, `path`, `interface`, `type`, `format`
- `readonly`, `discard` (TRIM), `ssd`
- Advanced: `cache` (none/writethrough/writeback), `aio` (threads/native/io_uring)
- Advanced: `detect_zeroes` (off/on/unmap)
- Attachment: `controller`, `scsi_id`, `boot_index`, `bus`, `unit`

**Multi-Controller Support:**
- Drives can attach to specific SCSI controllers by name
- IDE, SATA, NVMe placement via bus/addr fields
- Auto-generated IDs if not provided

### Network Devices

**Models:** `virtio-net`, `e1000`, `rtl8139`, others  
**Backend Types:** `user` (NAT), `tap`, `bridge`, `socket`, `fd`

**Backend Configuration:**
```yaml
networks:
  - id: net0
    model: virtio-net
    mac: "52:54:00:12:34:56"
    backend:
      type: user
      hostfwd:
        - "tcp:127.0.0.1:22-:22"
      listen: 127.0.0.1
```

**Features:**
- Port forwarding (user-mode)
- vhost acceleration (tap/bridge backends)
- Multi-queue support
- Custom bridge/script configuration
- Boot index support for PXE

### Display Devices

**Types:** `virtio-gpu`, `qxl`, `cirrus`, `vmware-svga`  
**VRAM:** Configurable per device type

### Serial Devices

**Types:**
- `pty` - Pseudo-terminal
- `unix` - Unix socket
- `tcp` - TCP socket (with host/localaddr/localport)
- `stdio`, `file`, others

### GPU Passthrough

**PCI Passthrough Configuration:**
```yaml
host:
  pci:
    - device: "0000:03:00.0"
      id: gpu0
      x_vga: true                # VGA passthrough flag
      pcie: true                 # PCIe topology
      romfile: /path/to/rom      # Optional ROM file
      bus: pcie.0                # Guest bus placement
      multifunction: false       # Multifunction slots
```

**Features:**
- IOMMU requirement detection (checks `/proc/cmdline`)
- x-vga flag for primary GPU
- ROM file loading
- Multifunction device grouping
- PCIe topology support

### USB Passthrough

**Two Configuration Styles:**

1. **QEMU-style:**
   ```yaml
   host:
     usb:
       - id: usb0
         host: "1:1"  # bus:device format
   ```

2. **Proxmox-style:**
   ```yaml
   host:
     usb:
       - id: usb0
         hostbus: "1"
         hostport: "1"
   ```

**XHCI Controller Support:**
```yaml
controllers:
  xhci:
    - id: xhci0
      p2: 4        # USB 2.0 ports
      p3: 4        # USB 3.0 ports
      bus: pcie.0
```

### Audio Devices

**Configuration:**
```yaml
devices:
  audio:
    - type: hda-duplex
      id: audio0
      audiodev: audiodev0  # References backend
```

**Additional Audio Support:**
- SPICE audio backend (when SPICE enabled)
- Input devices for tablet/keyboard

### Input Devices

**Types:** `tablet`, `keyboard`, `mouse`

```yaml
devices:
  input:
    - type: tablet
    - type: keyboard
```

### Advanced Features

**TPM (Trusted Platform Module):**
- Version: 1.2 or 2.0
- Backend: emulator (swtpm) or passthrough
- Device: tpm-tis or tpm-crb
- State persistence via state_dir or state_backend_uri

**Memory Ballooning:**
- Models: virtio-balloon-pci, virtio-balloon-ccw
- Free page reporting support
- Optional device ID and bus placement

**ivshmem (Looking Glass Shared Memory):**
- Size: default 32 MiB
- Device file: default `/dev/kvmfr0`
- Vectors configurable for QEMU version compatibility

**Hyper-V Enlightenments (Windows):**
- Flags: relaxed, vapic, time, crash, reset
- Frequencies, reenlightenment, tlbflush, IPI, spinlock

---

## 4. Feature List

### Core VM Operations

- ✅ **Dry-run mode** (`--dry-run`) - Display QEMU command without execution
- ✅ **Validation** (`validate` command) - Check config syntax and compatibility
- ✅ **Config caching** (`create` command) - Store VM config for later reference
- ✅ **Profile system** - Layer config overrides via named profiles
- ✅ **Daemon mode** (`start --daemon`) - Background VM execution
- ✅ **Interactive mode** - Foreground VM with TTY attachment
- ✅ **VM state tracking** - PID files, log rotation
- ✅ **Graceful shutdown** - SIGTERM, forceful kill (SIGKILL)

### Auxiliary Processes

**swtpm (Software TPM) Integration:**
- Auto-spawn external TPM emulator when configured
- Socket coordination with QEMU
- State directory and backend URI support
- Lifecycle management (stops when VM stops)

**Looking Glass Client:**
- Auto-launch from VM startup if configured
- Full-screen, window size, grab keyboard options
- Integrated with central config or per-VM options

**SPICE Client (remote-viewer):**
- Auto-launch SPICE display client
- Port/address configuration
- Integrated with VM audio and vdagent

### Storage Management

- ✅ **QCOW2 creation** - `storage create --size`
- ✅ **Disk info** - `storage info`
- ✅ **Disk resize** - `storage resize --size`
- ✅ **Snapshot management** - `storage snapshot --name`
- ✅ **Image listing** - `storage list`

### Device Operations

- ✅ **USB device listing** - `device usb list` (via lsusb)
- ✅ **PCI device listing** - `device pci list` (via lspci)
- ✅ **Network bridge creation** - `network bridge --name`
- ✅ **Hot-add/remove drives** - QMP-based runtime attachment
- ✅ **Hot-add/remove networks** - QMP-based runtime attachment

### Display & Interaction

- ✅ **SPICE display** - Structured configuration with auth/audio/vdagent
- ✅ **VNC display** - Via QEMU's `-vnc` parameter
- ✅ **Serial console** - `console` command for VM serial port attachment
- ✅ **QMP socket** - Optional QEMU Monitor Protocol access for tooling

### Networking

- ✅ **User-mode networking** - Default NAT backend
- ✅ **Bridge networking** - Attached to host bridges
- ✅ **Tap devices** - Direct interface binding
- ✅ **Socket networking** - For QEMU sandbox scenarios
- ✅ **Host port forwarding** - In user-mode via hostfwd syntax
- ✅ **Multi-queue support** - Improved throughput for virtio-net

### CPU & Memory

- ✅ **CPU model selection** - host, custom models, features
- ✅ **vCPU count** - Symmetric multiprocessing (SMP)
- ✅ **CPU features** - Feature string composition
- ✅ **NUMA topology** - Multi-node configuration
- ✅ **Memory ballooning** - Dynamic RAM adjustment
- ✅ **Free page reporting** - Optimization for host memory pressure

### Boot & Firmware

- ✅ **UEFI boot** - OVMF firmware integration
- ✅ **BIOS boot** - SeaBIOS
- ✅ **Custom boot order** - disk, cdrom, network, etc.
- ✅ **Kernel/initrd direct boot** - `-kernel`, `-initrd`, `-append`
- ✅ **Boot index** - Per-device boot ordering

### System Features

- ✅ **SMBIOS customization** - System manufacturer, version, etc.
- ✅ **RTC configuration** - base, driftfix options
- ✅ **Machine-specific options** - Append to `-machine` parameter
- ✅ **QEMU readconfig** - Load external machine topology files
- ✅ **KVM acceleration** - `-enable-kvm` flag support
- ✅ **Guest agent** (qemu-guest-agent) - Optional virtio-serial device

### Advanced Platform Features

- ✅ **TPM 1.2/2.0** - Software emulator or passthrough
- ✅ **Hyper-V enlightenments** - Windows optimization flags
- ✅ **iSCSI storage** - Network-attached SCSI disks
- ✅ **SCSI controllers** - Configurable controllers with max_targets, iothreads
- ✅ **XHCI controllers** - USB 3 with configurable port counts
- ✅ **PCI topology** - Custom PCI bridge placement
- ✅ **ivshmem** - Looking Glass shared memory support

---

## 5. Command Structure

### Top-Level Commands

**VM Lifecycle:**
- `create CONFIG [--validate-only]` - Validate and cache VM config
- `start CONFIG [--daemon] [--dry-run]` - Start VM (foreground or background)
- `stop CONFIG [--force]` - Stop VM (gracefully or SIGKILL)
- `kill CONFIG` - Force kill VM
- `list` - Show running VMs with PIDs
- `status CONFIG` - Show VM status
- `console CONFIG` - Attach to VM serial console
- `validate CONFIG [--show-resolved-config]` - Validate config + optionally display merged result

**Storage Subcommand:**
- `storage create NAME --size <N>` - Create QCOW2 image
- `storage list` - List available disk images
- `storage info DISK` - Show disk properties
- `storage resize DISK --size <N>` - Resize disk
- `storage snapshot DISK --name <N>` - Create snapshot

**Device Subcommand:**
- `device usb [list]` - List USB devices
- `device pci [list]` - List PCI devices

**Network Subcommand:**
- `network bridge --name <NAME>` - Create bridge (informational, requires sudo)

### Command Execution Model

**Flow:**
```
Cli::parse()                    # Parse args, create Commands enum
  ↓
cli::execute(cli)               # Main dispatch function
  ↓
match cli.command {             # Route to handler
  Commands::Start { config, ... } → runtime::handle_start()
  Commands::Create { config, ... } → commands::handle_create()
  ...
}
```

**Error Handling:**
- All handlers return `Result<()>` (anyhow)
- main() exits with error status if Result is Err
- User-friendly error messages via Display impl

---

## 6. State Management

### Runtime State Organization

**State Directory:** `~/.ezkvm/` (XDG-like fallback to `$HOME/.ezkvm/`)

```
~/.ezkvm/
├── config/           # Cached VM configurations
│   └── {vm-name}.yaml
├── logs/             # VM output logs
│   └── {vm-name}/
│       ├── session-0.log
│       ├── session-1.log
│       └── ...
└── {vm-name}.pid     # PID file for running VM
```

### PID Management

**Function:** `save_pid(vm_name, pid) → Result<()>`
- Creates `~/.ezkvm/{vm-name}.pid` with process ID
- Used to track running VMs for stop/kill/status

**Discovery:** `read_pid(vm_name) → Result<Option<i32>>`
- Reads PID file if it exists
- Returns None if not running or PID stale

**Cleanup:** `delete_pid(vm_name) → Result<()>`
- Removes PID file after VM stops

### Configuration Caching

**Function:** `cache_config(vm_name, config) → Result<()>`
- Serializes VmConfig to YAML
- Saves to `~/.ezkvm/config/{vm-name}.yaml`
- Allows later reference without re-parsing original file

**Function:** `load_cached_config(vm_name) → Result<VmConfig>`
- Loads cached YAML for VM

### Log Management

**Log Files:**
- Location: `~/.ezkvm/logs/{vm-name}/`
- Naming: `session-{index}.log`
- Created per VM startup (if daemon or log_dir configured)

**Log Rotation:**
- `cleanup_old_logs_at(vm_name, log_dir, keep_count)` removes old logs
- Keeps last N log files (configurable via options.log_keep)

### VM Lifecycle Tracking

**Discovery:** `find_qemu_processes() → Result<Vec<ProcessInfo>>`
- Searches `/proc` for QEMU processes
- Matches against configured VM names via PID file lookup

**State Queries:**
- `is_vm_running(vm_name) -> Result<bool>`
- `list_running_vms() -> Result<Vec<(String, i32)>>` - name, PID tuples

**Control Operations:**
- `stop_vm(vm_name) -> Result<()>` - Send SIGTERM
- `kill_vm(vm_name) -> Result<()>` - Send SIGKILL

---

## 7. Notable Design Patterns

### 1. **Trait-Based Type Conversion**

```rust
// Type-safe QEMU argument emission
impl From<DriveConfig> for QemuArgs { ... }
impl From<NetworkConfig> for QemuArgs { ... }
impl From<SystemConfig> for QemuArgs { ... }

// Usage
let args = QemuArgs::from(drive_config);
args.extend(QemuArgs::from(network_config));
```

**Benefit:** Composable, type-checked, decoupled config → command logic

### 2. **Newtype Pattern for domain safety**

```rust
pub struct QemuArgs(Vec<String>);
impl QemuArgs {
    pub fn push_str(&mut self, arg: &str) { ... }
    pub fn extend(&mut self, args: impl IntoIterator<Item = String>) { ... }
}
```

**Benefit:** Prevents accidentally passing generic Vec<String>; centralizes argument construction semantics

### 3. **Builder Pattern (Deprecated but available)**

```rust
#[deprecated]
pub struct QemuCommandBuilder {
    args: QemuArgs,
}
impl QemuCommandBuilder {
    pub fn new() -> Self { ... }
    pub fn from_config(config: &VmConfig) -> Self { ... }
    pub fn name(mut self, name: &str) -> Self { ... }
    // ... chainable methods
}
```

**Current Status:** Marked deprecated; runtime path uses `QemuManager::build_command()`

### 4. **Lazy Enum Dispatch**

```rust
pub enum Commands {
    Create { config: String, validate_only: bool },
    Start { config: String, daemon: bool, dry_run: bool },
    Stop { config: String, force: bool },
    // ...
}

match cli.command {
    Commands::Create { config, validate_only } => handle_create(config, validate_only).await,
    // ...
}
```

**Benefit:** Type-safe command parsing; compiler enforces all variants handled

### 5. **Deep YAML Merge with Path-Aware Policies**

```rust
// Profile merge logic (src/config/loader/merge.rs)
match path_depth {
    ["host", "pci"] => merge_by_id(),       // ID-based merge
    ["devices", "drives"] => merge_append_all(),  // Append all
    ["system", "cpu", "features"] => merge_append_unique(),  // Deduplicate
    _ => merge_recursive(),                 // Deep merge
}
```

**Benefit:** Profiles can layer config without losing items; explicit merge semantics

### 6. **Composition Over Inheritance**

```rust
// VmConfig aggregates sub-configs
pub struct VmConfig {
    pub system: SystemConfig,
    pub devices: DeviceConfig,
    pub controllers: ControllersConfig,
    pub host: HostConfig,
    // ... no inheritance hierarchy
}
```

**Benefit:** Flat, discoverable schema; easier to validate and serialize

### 7. **Fail-Fast Validation Gateway**

```rust
pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    // ... load, merge, deserialize
    validation::validate_config(&config)?;  // Early validation
    Ok(config)
}
```

**Benefit:** Bad configs caught immediately, before runtime

### 8. **Auxiliary Process Pattern**

```rust
// swtpm, looking-glass, remote-viewer started separately
prepare_auxiliary_runtime(config, central_config, dry_run)?;
// ... then start main QEMU process
```

**Benefit:** Decoupled lifecycle; can debug QEMU independently

### 9. **Deterministic Command Generation via Typed Configs**

```rust
impl From<DriveConfig> for QemuArgs {
    fn from(drive: DriveConfig) -> Self {
        // Deterministic ordering of -drive args
        // Predictable for testing and debugging
    }
}
```

**Benefit:** Same config → identical command every time; regression testable

### 10. **Centralized Error Context with anyhow**

```rust
use anyhow::{Result, anyhow, Context};

fn validate_config(config: &VmConfig) -> Result<()> {
    validate_system_config(&config.system)
        .context("Failed to validate system config")?;
    // ...
}
```

**Benefit:** Rich error chain visible to user; no silent failure modes

---

## 8. Key Architectural Files Reference

### Configuration & Schema

- [src/config/vm_schema/vm_config.rs](../../../src/config/vm_schema/vm_config.rs) - Main VmConfig with profile loading
- [src/config/vm_schema/system.rs](../../../src/config/vm_schema/system.rs) - SystemConfig, CPU, memory, boot
- [src/config/vm_schema/device.rs](../../../src/config/vm_schema/device.rs) - Device device structure
- [src/config/vm_schema/drive.rs](../../../src/config/vm_schema/drive.rs) - Drive config & QEMU args
- [src/config/vm_schema/network.rs](../../../src/config/vm_schema/network.rs) - Network backends
- [src/config/vm_schema/display.rs](../../../src/config/vm_schema/display.rs) - Display devices
- [src/config/platform/](../../../src/config/platform/) - TPM, audio, GPU, SPICE, Hyper-V, ivshmem
- [src/config/loader/merge.rs](../../../src/config/loader/merge.rs) - Profile merge logic
- [src/config/validation.rs](../../../src/config/validation.rs) - Comprehensive config validation

### QEMU Integration

- [src/qemu/manager.rs](../../../src/qemu/manager.rs) - QemuManager, command building interface
- [src/qemu/executor.rs](../../../src/qemu/executor.rs) - Process execution, dry-run, logging
- [src/qemu/types.rs](../../../src/qemu/types.rs) - QemuArgs newtype and utilities
- [src/qemu/process/](../../../src/qemu/process/) - VM discovery, stop, kill

### Runtime & CLI

- [src/cli/types.rs](../../../src/cli/types.rs) - All command enums and clap definitions
- [src/cli/commands.rs](../../../src/cli/commands.rs) - Command handlers (create, storage, device, network)
- [src/cli/runtime/start.rs](../../../src/cli/runtime/start.rs) - VM startup orchestration
- [src/cli/runtime/auxiliary/](../../../src/cli/runtime/auxiliary/) - swtpm, Looking Glass, SPICE launches

### State & Metadata

- [src/state/](../../../src/state/) - PID files, logs, config cache, path conventions
- [doc/user/config/](../../user/config/) - User-facing YAML config documentation

---

## 9. Data Flow Example: Starting a VM

1. **Input:** `ezkvm start examples/ubuntu.yaml --daemon`

2. **Parsing (CLI layer):**
   - clap parses args → Commands::Start { config: "examples/ubuntu.yaml", daemon: true, dry_run: false }

3. **Config Loading (Config layer):**
   - Load ubuntu.yaml
   - Extract ENV vars (${VAR_NAME})
  - Resolve profile names: ["gpu-passthrough", "looking-glass"]
   - Load and merge profiles
   - Deserialize to VmConfig
   - Validate config
   - Auto-assign device IDs

4. **Central Config (Config → Runtime):**
   - Load central config (swtpm path, tools locations)

5. **Auxiliary Setup (Runtime layer):**
   - Create run-dir socket directories
   - Start swtpm process if TPM enabled
   - (dry-run skips this step)

6. **Command Building (QEMU layer):**
   - Create QemuManager(config, central_config)
   - Call build_command() → QemuArgs
     - System config → machine, CPU, memory args
     - Device config → drive, network, display args
     - Platform features → TPM, SPICE, guest-agent args
   - Result: `qemu-system-x86_64 -machine q35 -cpu host ... -enable-kvm`

7. **Process Execution (QEMU executor):**
   - If dry_run: print command, exit
   - If daemon: spawn async, save PID, print "VM started"
   - If interactive: execute sync, inherit TTY

8. **Auxiliary Clients (Runtime layer, post-VM-start):**
   - If looking-glass configured: spawn looking-glass-client
   - If SPICE configured: spawn remote-viewer

9. **State Tracking (State layer):**
   - Save PID file
   - Create/rotate log file
   - Cache VmConfig for reference

10. **Return to user:**
    - VM running in background (daemon) or foreground (interactive)

---

## 10. Testing Strategy

### Unit Tests
- Located alongside source modules (`src/**/*.rs` with `#[test]` blocks)
- Test config parsing, validation, command generation

### Integration Tests (`tests/` directory)
- [tests/integration_tests.rs](../../../tests/integration_tests.rs) - Integration test entrypoint
- [tests/config_tests.rs](../../../tests/config_tests.rs) - Config parsing tests
- Subdirectories organize tests by domain:
  - `tests/config/` - Config loading, profiles, validation
  - `tests/integration/` - Command workflows, end-to-end scenarios

### Test Examples
- Profile merging: [tests/integration/profile_compat.rs](../../../tests/integration/profile_compat.rs)
- Config parsing with environment variables: [tests/config/config_parsing_env.rs](../../../tests/config/config_parsing_env.rs)

---

## 11. Documentation Structure

### User-Facing Docs (`doc/user/config/`)
- [vm-structure.md](../../user/config/vm-structure.md) - Overall YAML schema
- [central-config.md](../../user/config/central-config.md) - Tool paths, locations
- [profiles-and-merge.md](../../user/config/profiles-and-merge.md) - Profile system
- [system-and-boot.md](../../user/config/system-and-boot.md) - CPU, mem, boot config
- [devices.md](../../user/config/devices.md) - Drives, networks, displays
- [platform-features.md](../../user/config/platform-features.md) - TPM, SPICE, audio, GPU
- [examples.md](../../user/config/examples.md) - Configuration examples

### Developer Docs (`doc/dev/`)
- [ARCHITECTURE_GUIDELINES.md](../ARCHITECTURE_GUIDELINES.md) - Modular design, layering
- [CODING_GUIDELINES.md](../CODING_GUIDELINES.md) - Rust style, error handling
- [IMPROVED_PROFILES.md](./IMPROVED_PROFILES.md) - Profile system design notes
- [IMPROVED_TARGET_SCHEMA.md](./IMPROVED_TARGET_SCHEMA.md) - Schema evolution notes
- [TODO.md](../../backlog/TODO.md) - Known limitations and roadmap

### Examples (`examples/`)
- [basic-vm.yaml](../../../examples/basic-vm.yaml) - Minimal working config
- [ezkvm-profiles.yaml](../../../examples/ezkvm-profiles.yaml) - Profile naming examples
- [profiles/](../../../examples/profiles/) - Reusable profile templates

---

## 12. Notable Implementation Details

### ID Assignment

- Drives: auto-assigned as `{interface}{index}` (e.g., `virtio2`)
- Networks: auto-assigned as `net{index}`
- Audio devices: require explicit ID
- Reserved IDs tracked during generation to avoid conflicts

### Device Attachment Logic

- Simple drives (no boot, bus, controller) → uses `-drive if={interface}`
- Advanced drives (boot_index, controller, etc.) → uses `-drive if=none` + separate `-device`
- Example: SCSI drive with controller attachment uses both drive node and device specifications

### Network Backend Serialization

- NetworkBackendConfig uses custom `to_netdev_spec()` to emit `-netdev` arguments
- Handles bridge name key variations (`br` vs `bridge`)
- Accumulates hostfwd rules as separate parts

### QEMU Version Compatibility

- ivshmem vectors configurable for different QEMU versions
- BIOS/UEFI selection via system.boot.firmware
- Read-config support for Proxmox machine topology files

---

## 13. Dependencies

**Key Crates:**
- `clap` (4.0) - CLI argument parsing with derive macros
- `tokio` (1.0) - Async runtime for background processes
- `serde` + `serde_yaml` - YAML deserialization
- `anyhow` - Ergonomic error handling
- `nix` (0.29) - Unix signal handling for VM termination
- `regex` (1.10) - Pattern matching for environment variable substitution

---

## Summary

**ezkvm** is a **modular, type-safe, YAML-driven KVM front-end** that:

1. **Uses strong typing** to prevent configuration errors at compile and validation time
2. **Layers cleanly** with unidirectional dependencies: CLI → Config → QEMU/Runtime → OS
3. **Supports profiles** with intelligent merging (ID-based, append-all, append-unique semantics)
4. **Handles complex device topologies** (PCI/PCIe, SCSI controllers, XHCI, GPU/USB passthrough)
5. **Integrates auxiliary processes** (swtpm, Looking Glass, SPICE cleanly)
6. **Provides fail-fast validation** followed by deterministic command generation
7. **Tracks runtime state** via PID files, logs, and config caching
8. **Follows Rust idioms** (traits, error types, composable APIs)

The architecture prioritizes **discoverability, modularity, and testability** while maintaining a **stable YAML contract** for users.
