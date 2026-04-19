# ezkvm - Easy KVM Virtual Machine Manager

A simple alternative to libvirt and virt-manager that uses YAML configuration files to manage QEMU/KVM virtual machines directly.

## Features

- **YAML Configuration**: Human-readable configuration files instead of XML
- **Direct QEMU Integration**: No libvirt abstraction layer
- **Type Safety**: Rust compiler ensures configuration correctness
- **Simple CLI**: Easy-to-use command-line interface
- **KVM Optimized**: Built specifically for KVM with QEMU
- **Environment Variable Substitution**: Support for `${VAR_NAME}` syntax in configs

## Installation

### Prerequisites

- Rust 1.70+ (2021 edition)
- QEMU with KVM support
- Linux kernel with KVM module

### Build from Source

```bash
git clone <repository-url>
cd ezkvm
cargo build --release
```

## Quick Start

1. Create a VM configuration file (see `examples/basic-vm.yaml`)

2. Validate the configuration:
```bash
ezkvm validate examples/basic-vm.yaml
```

3. Start the VM:
```bash
ezkvm start examples/basic-vm.yaml
```

4. Start in dry-run mode to see the QEMU command:
```bash
ezkvm start examples/basic-vm.yaml --dry-run
```

Both `start` and `start --dry-run` now run the same portable-runtime preflight checks before launching or rendering commands, including firmware discovery validation for UEFI/OVMF paths.

## Configuration

VMs are configured using YAML files with the following structure:

```yaml
name: "my-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048          # MiB
  vcpus: 2
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk", "cdrom"]

devices:
  drives:
    - id: "root"
      path: "/path/to/disk.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"

  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "user"
      mac: "52:54:00:12:34:56"

  displays:
    - type: "virtio-gpu"
      vram: 256

options:
  enable_kvm: true
  daemonize: false
```

### Environment Variable Substitution

Configuration files support environment variable substitution using `${VAR_NAME}` or `$VAR_NAME` syntax:

```yaml
system:
  memory: ${VM_MEMORY}  # Will be replaced with environment variable
  vcpus: 2

devices:
  drives:
    - path: "${HOME}/vms/disk.qcow2"  # Uses $HOME environment variable
```

## Examples

See the `examples/` directory for complete configuration examples:
- `basic-vm.yaml` - Full Ubuntu VM configuration
- `test-vm.yaml` - Minimal test configuration

## Commands

### VM Management
- `ezkvm create <config.yaml>` - Create and validate a VM configuration
- `ezkvm start <config.yaml>` - Start a VM from configuration
- `ezkvm start <config.yaml> --dry-run` - Show the QEMU command without executing
- `ezkvm start ...` and `ezkvm start ... --dry-run` run the same deterministic preflight checks in the same order before execution/preview
- Required preflight failures stop startup with actionable diagnostics (QEMU/swtpm binaries, OVMF availability for UEFI, bridge helper paths when configured, and runtime/socket directory access)
- Bridge backends auto-downgrade to user-mode with deterministic warnings when bridge-helper resolution fails or host policy prefers user networking
- TPM capability checks honor `host_capabilities.tpm.placement_mode`: `socket` mode validates swtpm binary discovery while `state-file` mode validates TPM state directory resolution
- Looking Glass honors `options.looking_glass.mode`: `explicit` fails fast when the client is missing, `auto` degrades silently, and `disabled` suppresses launch
- Optional integrations (remote-viewer, Looking Glass) emit deterministic warnings and degrade without blocking VM start
- `ezkvm start <config.yaml> --daemon` - Start VM in background
- `ezkvm start <config.yaml> --run-dir <path> --swtpm-binary <path> --tpm-socket-path <path> --remote-viewer-program <path> --looking-glass-program <path> --ovmf-dir <path>` - Override runtime host defaults used by preflight capability checks and runtime resolution
- `ezkvm stop <config.yaml>` - Stop a VM gracefully
- `ezkvm stop <config.yaml> --force` - Force stop a VM
- `ezkvm kill <config.yaml>` - Force kill a VM
- `ezkvm list` - List running VMs
- `ezkvm status <config.yaml>` - Show VM status
- `ezkvm console <config.yaml>` - Attach to VM console
- `ezkvm validate <config.yaml>` - Validate configuration file

### Storage Management
- `ezkvm storage create <name> --size <GB>` - Create a QCOW2 disk image
- `ezkvm storage list` - List available disk images
- `ezkvm storage info <disk>` - Show disk image information
- `ezkvm storage resize <disk> --size <GB>` - Resize a disk image
- `ezkvm storage snapshot <disk> --name <snapshot>` - Create a disk snapshot

### Device Management
- `ezkvm device usb list` - List available USB devices
- `ezkvm device pci list` - List available PCI devices

### Network Management
- `ezkvm network bridge <name>` - Create a network bridge

## Architecture

ezkvm follows a modular architecture with clear separation of concerns:

For repository architecture standards and packaging/layering guidance, see [doc/dev/ARCHITECTURE_GUIDELINES.md](doc/dev/ARCHITECTURE_GUIDELINES.md).

- **config/**: YAML parsing, validation, and configuration structures
- **qemu/**: QEMU command generation and process management
- **cli/**: Command-line interface and command dispatch
- **state/**: VM state management and persistence
- **storage/**: Storage device management utilities
- **device/**: Hardware device management (USB, PCI passthrough)
- **network/**: Network configuration and bridge management

### Design Principles

- **Type Safety**: Rust's type system ensures configuration correctness
- **Direct QEMU Integration**: No libvirt abstraction layer for maximum control
- **YAML Configuration**: Human-readable configs instead of XML
- **Modular Design**: Easy to extend with new features
- **Error Handling**: Comprehensive error messages with actionable suggestions

### Architecture Diagram

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Layer     │    │  Config Layer   │    │  QEMU Layer     │
│                 │    │                 │    │                 │
│ • Command       │    │ • YAML Parsing  │    │ • Command Gen   │
│   parsing       │◄──►│ • Validation    │◄──►│ • Process Mgmt  │
│ • Help/usage    │    │ • Env vars      │    │ • Monitoring    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   State Layer   │
                    │                 │
                    │ • PID tracking  │
                    │ • Config cache  │
                    │ • Logs          │
                    └─────────────────┘
```

## Development

## Rust Guidelines Monitor Agent

Use the repository coding standard in [doc/dev/CODING_GUIDELINES.md](doc/dev/CODING_GUIDELINES.md) together with the review agent in [.github/rust-guidelines-monitor.agent.md](.github/rust-guidelines-monitor.agent.md).

When to run it:
- Before opening or merging a pull request.
- After refactors that touch multiple modules.
- After schema, merge, or CLI behavior changes.
- When a file grows significantly and maintainability is a concern.

What to ask it to review:
- Correctness and behavior regressions.
- Error handling and actionable failure messages.
- API clarity and maintainability.
- Test and documentation coverage gaps.

Suggested prompt template:

```text
Review the current changes against doc/dev/CODING_GUIDELINES.md.

Scope:
- Focus on Rust files and affected tests/docs.
- Prioritize correctness, safety, and regression risks.

Output:
- Findings first, ordered by severity.
- For each finding: file path, issue, and concrete fix suggestion.
- Then assumptions/open questions.
- Then a short summary.

Validation:
- Run cargo fmt --all
- Run cargo clippy --all-targets --all-features -- -D warnings
- Run cargo test
```

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

The test suite includes:
- Unit tests for configuration parsing and validation
- Integration tests for file I/O and environment variable substitution
- End-to-end tests for CLI functionality

### Running

```bash
cargo run -- validate examples/basic-vm.yaml
```

## Migration from virt-manager/libvirt

### Key Differences

| Feature | virt-manager/libvirt | ezkvm |
|---------|---------------------|-------|
| Configuration | XML files | YAML files |
| Backend | libvirt abstraction | Direct QEMU |
| Learning Curve | Steep | Gentle |
| Flexibility | Limited by libvirt | Full QEMU control |
| Dependencies | Many packages | Minimal (QEMU + Rust) |

### Converting a Basic VM

**virt-manager XML:**
```xml
<domain type='kvm'>
  <name>ubuntu-vm</name>
  <memory unit='MiB'>2048</memory>
  <vcpu>2</vcpu>
  <os>
    <type arch='x86_64' machine='pc-q35-6.2'>hvm</type>
    <boot dev='hd'/>
  </os>
  <devices>
    <disk type='file' device='disk'>
      <driver name='qemu' type='qcow2'/>
      <source file='/var/lib/libvirt/images/ubuntu.qcow2'/>
      <target dev='vda' bus='virtio'/>
    </disk>
  </devices>
</domain>
```

**Equivalent ezkvm YAML:**
```yaml
name: "ubuntu-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 2048
  vcpus: 2
  cpu_model: "host"

devices:
  drives:
    - id: "root"
      path: "/var/lib/libvirt/images/ubuntu.qcow2"
      interface: "virtio"
      type: "disk"
      format: "qcow2"
```

### Command Equivalents

| virt-manager/libvirt | ezkvm |
|---------------------|-------|
| `virsh define config.xml` | `ezkvm validate config.yaml` |
| `virsh start vm-name` | `ezkvm start config.yaml` |
| `virsh shutdown vm-name` | `ezkvm stop config.yaml` |
| `virsh destroy vm-name` | `ezkvm kill config.yaml` |
| `virsh list --all` | `ezkvm list` |
| `virt-viewer vm-name` | `ezkvm console config.yaml` |

MIT License

## Contributing

Contributions welcome! Please see the project plan in `ProjectPlan.md` for roadmap and development guidelines.