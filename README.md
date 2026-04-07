# ezkvm - Easy KVM Virtual Machine Manager

A simple alternative to libvirt and virt-manager that uses YAML configuration files to manage QEMU/KVM virtual machines directly.

## Features

- **YAML Configuration**: Human-readable configuration files instead of XML
- **Direct QEMU Integration**: No libvirt abstraction layer
- **Type Safety**: Rust compiler ensures configuration correctness
- **Simple CLI**: Easy-to-use command-line interface
- **KVM Optimized**: Built specifically for KVM with QEMU

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

3. Create the VM:
```bash
ezkvm create examples/basic-vm.yaml
```

4. Start the VM:
```bash
ezkvm start basic-ubuntu
```

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

## Commands

- `ezkvm create <config.yaml>` - Create and validate a VM
- `ezkvm start <name>` - Start a VM
- `ezkvm stop <name>` - Stop a VM gracefully
- `ezkvm kill <name>` - Force kill a VM
- `ezkvm list` - List running VMs
- `ezkvm status <name>` - Show VM status
- `ezkvm console <name>` - Attach to VM console
- `ezkvm validate <config.yaml>` - Validate configuration

## Architecture

- **config/**: YAML parsing and validation
- **qemu/**: QEMU command generation and process management
- **cli/**: Command-line interface

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running

```bash
cargo run -- validate examples/basic-vm.yaml
```

## License

MIT License

## Contributing

Contributions welcome! Please see the project plan in `ProjectPlan.md` for roadmap and development guidelines.