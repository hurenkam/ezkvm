# ezkvm Project Plan

## **Project Overview**
- **Name**: ezkvm (Easy KVM)
- **Language**: Rust (stable toolchain)
- **Configuration**: YAML files translated to direct QEMU commands
- **Scope**: QEMU/KVM only, no libvirt abstraction layer
- **Architecture**: CLI tool with modular design for future expansion

## **Phase 1: Project Setup & Core Infrastructure (1-2 weeks)**

### 1.1 Initialize Rust Project
```bash
cargo new ezkvm --bin
cd ezkvm
```

### 1.2 Add Core Dependencies
Update `Cargo.toml` with essential crates:
- `serde` + `serde_yaml` for configuration parsing
- `clap` for CLI argument parsing  
- `anyhow` for error handling
- `tokio` for async operations (future-proofing)
- `nix` for system calls

### 1.3 Project Structure
```
ezkvm/
├── src/
│   ├── config/
│   │   ├── mod.rs           # Config structs and serde
│   │   ├── system.rs        # CPU, memory, machine config
│   │   ├── devices.rs       # Drives, networks, displays
│   │   └── validation.rs    # Config validation
│   ├── qemu/
│   │   ├── builder.rs       # QEMU argument generation
│   │   ├── executor.rs      # Process management
│   │   └── args.rs          # Argument structures
│   ├── cli.rs               # Command-line interface
│   └── main.rs
├── examples/
│   └── basic-vm.yaml        # Example configuration
├── Cargo.toml
└── README.md
```

## **Phase 2: YAML Configuration Schema & Parsing (2-3 weeks)**

### 2.1 Define YAML Schema
Create a simplified, human-readable schema covering essential VM components:

```yaml
name: "ubuntu-server"
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
      path: "/var/lib/ezkvm/ubuntu.qcow2"
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

### 2.2 Implement Configuration Types
- Define Rust structs with `#[derive(Serialize, Deserialize)]`
- Add validation logic for architecture compatibility
- Support optional fields with defaults

### 2.3 Configuration Loading
- File path resolution (absolute/relative)
- Environment variable substitution
- Schema validation before VM creation

## **Phase 3: QEMU Command Generation (2-3 weeks)**

### 3.1 Argument Builder Pattern
- Type-safe QEMU argument construction
- Prevent invalid combinations (e.g., incompatible CPU models)
- Support for all major QEMU options

### 3.2 Command Execution
- Spawn `qemu-system-*` processes
- Handle stdout/stderr redirection
- Process lifecycle management (start/stop/status)

### 3.3 Error Handling
- Parse QEMU error messages
- Provide meaningful error messages to users
- Graceful cleanup on failures

## **Phase 4: CLI Interface & VM Management (2 weeks)**

### 4.1 Core Commands
```bash
ezkvm create <config.yaml>    # Validate and prepare VM
ezkvm start <vm-name>         # Launch VM
ezkvm stop <vm-name>          # Graceful shutdown
ezkvm kill <vm-name>          # Force stop
ezkvm list                    # Show running VMs
ezkvm status <vm-name>        # VM details
ezkvm console <vm-name>       # Attach to VM console
```

### 4.2 VM State Management
- Track running VMs (PID files or process monitoring)
- Configuration caching for quick restarts
- Log rotation for VM output

## **Phase 5: Advanced Features (3-4 weeks)**

### 5.1 Device Management
- Hot-plug support for drives/networks
- USB device passthrough
- SPICE/VNC remote access

### 5.2 Networking
- Bridge mode configuration
- Port forwarding
- Network isolation

### 5.3 Storage
- QCOW2 image creation/management
- Snapshot support
- Live migration preparation

## **Phase 6: Testing & Documentation (2 weeks)**

### 6.1 Testing Strategy
- Unit tests for configuration parsing
- Integration tests with QEMU mock
- End-to-end tests with minimal VMs

### 6.2 Documentation
- README with installation and usage
- Configuration reference
- Architecture diagrams
- Migration guide from virt-manager

## **Technical Decisions**

### **QEMU Integration Approach**
- **Phase 1-2**: Generate command-line arguments (simple, reliable)
- **Phase 3+**: Direct KVM API usage via `kvm-ioctls` crate (performance, control)

### **Configuration Philosophy**
- **Minimal but extensible**: Start with essential options
- **Validation-first**: Prevent invalid configurations at parse time
- **Human-readable**: YAML over XML for better developer experience

### **Error Handling**
- Use `anyhow` for ergonomic error propagation
- Custom error types for VM-specific failures
- User-friendly error messages with suggestions

## **Risks & Mitigations**

### **QEMU API Stability**
- **Risk**: QEMU command-line changes break compatibility
- **Mitigation**: Version detection, fallback modes, comprehensive testing

### **Security**
- **Risk**: Direct QEMU execution could expose host
- **Mitigation**: Input validation, sandboxing, minimal privilege execution

### **Performance**
- **Risk**: Rust overhead vs libvirt C performance  
- **Mitigation**: Profile early, optimize hot paths, consider direct KVM integration

## **Success Metrics**
- ✅ Parse and validate YAML configs without errors
- ✅ Generate correct QEMU commands for basic VMs
- ✅ Start/stop VMs reliably
- ✅ Handle common error scenarios gracefully
- ✅ Configuration simpler than virt-manager XML
- ✅ Faster startup than libvirt (target: <500ms overhead)

## **Timeline & Milestones**
- **Month 1**: Project setup, YAML parsing, basic QEMU execution
- **Month 2**: CLI interface, VM lifecycle management, testing
- **Month 3**: Advanced features, documentation, performance optimization