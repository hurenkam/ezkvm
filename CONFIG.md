# Virtual Machine Configuration Reference

This document provides a comprehensive reference for the YAML configuration schema used by ezkvm. Configuration files define virtual machine specifications that are translated into QEMU commands.

## Overview

ezkvm uses YAML configuration files to define virtual machines in a human-readable format. The configuration is validated at load time and supports environment variable substitution.

## Central Configuration

ezkvm supports a central configuration file (`/etc/ezkvm.yaml`) that stores global tool paths and directory locations. This allows VM configurations to be more concise by referencing central settings implicitly.

### Loading Central Configuration

- **Default location**: `/etc/ezkvm.yaml`
- **Environment override**: Set `EZKVM_CONFIG` to specify a custom path
- **Fallback**: If the file doesn't exist, default values are used

### Central Configuration Schema

```yaml
tools:
  swtpm: "/usr/bin/swtpm"
  remote_viewer: "/usr/bin/remote-viewer"
  looking_glass: "/usr/bin/looking-glass-client"

locations:
  run_dir: "/var/run/ezkvm"
  ovmf_dir: "/usr/share/OVMF"
  vm_dir: "/etc/ezkvm/vms"
  template_dir: "/etc/ezkvm/templates"
```

### Central Configuration Fields

#### `tools`
Tool executable paths.

- `tools.swtpm` (optional): Path to the TPM emulator executable
- `tools.remote_viewer` (optional): Path to the remote viewer executable
- `tools.looking_glass` (optional): Path to the Looking Glass client executable

#### `locations`
Directory paths for runtime and configuration files.

- `locations.run_dir` (optional): Directory for runtime files (PID files, sockets)
- `locations.ovmf_dir` (optional): Directory containing OVMF firmware files
- `locations.vm_dir` (optional): Default directory for VM configuration files
- `locations.template_dir` (optional): Directory for VM templates

### Runtime Automation

When `tools.swtpm` is configured and the VM uses `tpm.backend: "emulator"`, ezkvm will start `swtpm` automatically before launching QEMU and will wire the configured socket path into the VM.

When `tools.remote_viewer` is configured and SPICE is enabled, ezkvm will launch `remote-viewer spice://ADDR:PORT` to attach automatically.

When `tools.looking_glass` is configured and `ivshmem.enabled` is true, ezkvm will launch `looking-glass-client` automatically after startup.

### Usage in VM Configurations

Central configuration values are used as defaults when VM-specific values are not provided. For example:

- TPM socket paths default to `{run_dir}/tpm` if `run_dir` is set
- UEFI firmware paths default to `{ovmf_dir}/OVMF.fd` if `ovmf_dir` is set

VM configurations can still override these values for specific cases.

## Basic Structure

```yaml
name: "my-vm"
backend: "qemu"

system:
  # System configuration (required)

boot:
  # Boot configuration (optional)

devices:
  # Device configuration (optional)

options:
  # Additional options (optional)
```

## Top-Level Fields

### `name` (required)
- **Type**: String
- **Description**: Unique identifier for the virtual machine
- **Example**: `"ubuntu-server"`

### `backend` (required)
- **Type**: String
- **Description**: Virtualization backend to use
- **Allowed values**: `"qemu"` (currently the only supported backend)
- **Example**: `"qemu"`

## System Configuration

The `system` section defines the core virtual machine hardware specifications.

### `system.architecture` (required)
- **Type**: String
- **Description**: Target CPU architecture
- **Allowed values**: `"x86_64"`, `"aarch64"`, `"x86"`, `"ppc64"`, `"riscv64"`
- **Example**: `"x86_64"`

### `system.machine` (required)
- **Type**: String
- **Description**: Machine type/emulation model
- **Common values**:
  - `"q35"` - Modern Q35 chipset (recommended for x86_64)
  - `"pc"` - Legacy PC chipset
  - `"virt"` - ARM virtual machine
- **Example**: `"q35"`

### `system.memory` (required)
- **Type**: Integer
- **Description**: RAM size in MiB (Mebibytes)
- **Minimum**: 128 MiB
- **Maximum**: 1 TiB (1048576 MiB)
- **Example**: `2048` (2 GiB)

### `system.vcpus` (required)
- **Type**: Integer
- **Description**: Number of virtual CPU cores
- **Minimum**: 1
- **Maximum**: 1024
- **Example**: `2`

### `system.cpu_model` (required)
- **Type**: String
- **Description**: CPU model to emulate
- **Common values**:
  - `"host"` - Use host CPU features (requires KVM)
  - `"qemu64"` - Basic x86_64 emulation
  - `"cortex-a72"` - ARM Cortex-A72 (for aarch64)
- **Example**: `"host"`

### `system.cpu_features` (optional)
- **Type**: Array of objects
- **Description**: CPU feature flags to enable/disable
- **Default**: Empty array

Each feature object has:
- `name` (string): Feature specification (e.g., `"+vmx"`, `"-avx"`)

**Example**:
```yaml
cpu_features:
  - name: "+vmx"    # Enable Intel VT-x
  - name: "-avx"    # Disable AVX instructions
```

## Boot Configuration

The `boot` section controls how the virtual machine starts up.

### `boot.firmware` (optional)
- **Type**: String
- **Description**: Firmware type for the virtual machine
- **Allowed values**: `"uefi"`, `"bios"`
- **Default**: Not specified (QEMU default)
- **Example**: `"uefi"`

### `boot.boot_order` (optional)
- **Type**: Array of strings
- **Description**: Boot device priority order
- **Allowed values**: `"disk"`, `"cdrom"`, `"network"`
- **Default**: QEMU default order
- **Example**: `["disk", "cdrom"]`

### `boot.kernel` (optional)
- **Type**: String
- **Description**: Path to kernel image for direct kernel boot
- **Example**: `"/boot/vmlinuz-linux"`

### `boot.initrd` (optional)
- **Type**: String
- **Description**: Path to initial ramdisk for direct kernel boot
- **Example**: `"/boot/initramfs-linux.img"`

### `boot.cmdline` (optional)
- **Type**: String
- **Description**: Kernel command line parameters for direct kernel boot
- **Example**: `"console=ttyS0 root=/dev/vda1"`

## Device Configuration

The `devices` section defines hardware devices attached to the VM.

### Storage Devices (`devices.drives`)

Each drive object supports:

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the drive
- **Example**: `"root"`

#### `path` (required)
- **Type**: String
- **Description**: Path to the disk image file
- **Example**: `"/var/lib/ezkvm/ubuntu.qcow2"`

#### `interface` (required)
- **Type**: String
- **Description**: Storage interface/controller type
- **Allowed values**: `"virtio"`, `"scsi"`, `"ide"`, `"nvme"`
- **Recommended**: `"virtio"` for modern VMs
- **Example**: `"virtio"`

#### `type` (required)
- **Type**: String
- **Description**: Drive type
- **Allowed values**: `"disk"`, `"cdrom"`
- **Example**: `"disk"`

#### `format` (required)
- **Type**: String
- **Description**: Disk image format
- **Common values**: `"qcow2"`, `"raw"`, `"vmdk"`
- **Example**: `"qcow2"`

#### `readonly` (optional)
- **Type**: Boolean
- **Description**: Whether the drive is read-only
- **Default**: `false`
- **Example**: `true`

**Example**:
```yaml
drives:
  - id: "root"
    path: "/var/lib/ezkvm/ubuntu.qcow2"
    interface: "virtio"
    type: "disk"
    format: "qcow2"
  - id: "cdrom"
    path: "/path/to/installer.iso"
    interface: "ide"
    type: "cdrom"
    format: "raw"
    readonly: true
```

### Network Devices (`devices.networks`)

Each network object supports:

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the network device
- **Example**: `"net0"`

#### `model` (required)
- **Type**: String
- **Description**: Network device model
- **Common values**: `"virtio-net"`, `"e1000"`, `"rtl8139"`
- **Recommended**: `"virtio-net"` for modern VMs
- **Example**: `"virtio-net"`

#### `mode` (required)
- **Type**: String
- **Description**: Network backend mode
- **Allowed values**: `"user"`, `"bridge"`, `"socket"`
- **Example**: `"user"`

#### `mac` (optional)
- **Type**: String
- **Description**: MAC address for the virtual NIC
- **Format**: Standard MAC address (e.g., `"52:54:00:12:34:56"`)
- **Default**: Auto-generated by QEMU
- **Example**: `"52:54:00:12:34:56"`

**Example**:
```yaml
networks:
  - id: "net0"
    model: "virtio-net"
    mode: "user"
    mac: "52:54:00:12:34:56"
```

### Display Devices (`devices.displays`)

Each display object supports:

#### `type` (required)
- **Type**: String
- **Description**: Display device type
- **Common values**: `"virtio-gpu"`, `"qxl"`, `"cirrus"`
- **Recommended**: `"virtio-gpu"` for modern VMs
- **Example**: `"virtio-gpu"`

#### `vram` (optional)
- **Type**: Integer
- **Description**: Video RAM in MiB
- **Default**: QEMU default (usually 16 MiB)
- **Example**: `256`

**Example**:
```yaml
displays:
  - type: "virtio-gpu"
    vram: 256
```

### Serial Devices (`devices.serials`)

Each serial object supports:

#### `type` (required)
- **Type**: String
- **Description**: Serial device type
- **Allowed values**: `"pty"`, `"file"`, `"socket"`, `"stdio"`
- **Example**: `"pty"`

#### `port` (optional)
- **Type**: Integer
- **Description**: Serial port number (for multi-port setups)
- **Default**: `0`
- **Example**: `1`

**Example**:
```yaml
serials:
  - type: "pty"
    port: 0
```

## Options Configuration

The `options` section provides additional VM configuration flags.

### `options.enable_kvm` (optional)
- **Type**: Boolean
- **Description**: Enable KVM hardware acceleration
- **Default**: `true`
- **Note**: Requires KVM support in kernel and appropriate permissions

### `options.daemonize` (optional)
- **Type**: Boolean
- **Description**: Run QEMU in daemon mode (background)
- **Default**: `false`

### `options.uefi_vars` (optional)
- **Type**: String
- **Description**: Path to UEFI variables file (for UEFI firmware)
- **Example**: `"/var/lib/ezkvm/uefi-vars.fd"`

## Environment Variable Substitution

Configuration files support environment variable substitution using `${VAR_NAME}` or `$VAR_NAME` syntax:

```yaml
system:
  memory: ${VM_MEMORY}
  vcpus: 2

devices:
  drives:
    - path: "${HOME}/vms/disk.qcow2"
```

Environment variables are substituted before YAML parsing. Missing variables cause configuration loading to fail.

## Complete Example

```yaml
name: "ubuntu-server"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 4096
  vcpus: 4
  cpu_model: "host"
  cpu_features:
    - name: "+vmx"

boot:
  firmware: "uefi"
  boot_order: ["disk", "cdrom"]
  kernel: "/boot/vmlinuz"
  initrd: "/boot/initrd.img"
  cmdline: "console=ttyS0 root=/dev/vda1"

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

  serials:
    - type: "pty"

options:
  enable_kvm: true
  daemonize: false
  uefi_vars: "/var/lib/ezkvm/uefi-vars.fd"
```

## TPM Configuration

The `tpm` section configures Trusted Platform Module support for enhanced security.

### `tpm.version` (required)
- **Type**: String
- **Description**: TPM specification version
- **Allowed values**: `"1.2"`, `"2.0"`
- **Example**: `"2.0"`

### `tpm.backend` (required)
- **Type**: String
- **Description**: TPM backend implementation
- **Allowed values**: `"emulator"`, `"passthrough"`
- **Example**: `"emulator"`

### `tpm.state_path` (optional)
- **Type**: String
- **Description**: Path to TPM state file (for emulator backend)
- **Default**: `"/var/run/qemu-server/tpm"`
- **Example**: `"/var/lib/ezkvm/tpm/my-vm-tpm.state"`

### `tpm.model` (optional)
- **Type**: String
- **Description**: TPM device model
- **Allowed values**: `"tpm-tis"`, `"tpm-crb"`
- **Default**: `"tpm-tis"`
- **Example**: `"tpm-tis"`

**Example TPM Configuration:**
```yaml
tpm:
  version: "2.0"
  backend: "emulator"
  state_path: "/var/lib/ezkvm/tpm/win11-tpm.state"
  model: "tpm-tis"
```

## Guest Agent Configuration

The `guest_agent` section configures QEMU Guest Agent for enhanced VM management.

### `guest_agent.enabled` (optional)
- **Type**: Boolean
- **Description**: Enable QEMU Guest Agent
- **Default**: `true`
- **Example**: `true`

### `guest_agent.socket_path` (optional)
- **Type**: String
- **Description**: Path to guest agent socket
- **Default**: `"/var/run/qemu-server/qga.sock"`
- **Example**: `"/var/lib/ezkvm/qga/my-vm.sock"`

### `guest_agent.freeze_cpu` (optional)
- **Type**: Boolean
- **Description**: Freeze CPU on suspend
- **Default**: `false`
- **Example**: `true`

**Example Guest Agent Configuration:**
```yaml
guest_agent:
  enabled: true
  socket_path: "/var/lib/ezkvm/qga/ubuntu-server.sock"
  freeze_cpu: false
```

## Memory Ballooning Configuration

The `ballooning` section configures memory ballooning for dynamic memory management.

### `ballooning.enabled` (optional)
- **Type**: Boolean
- **Description**: Enable memory ballooning
- **Default**: `true`
- **Example**: `true`

### `ballooning.free_page_reporting` (optional)
- **Type**: Boolean
- **Description**: Enable free page reporting for better performance
- **Default**: `false`
- **Example**: `true`

### `ballooning.model` (optional)
- **Type**: String
- **Description**: Balloon device model
- **Default**: `"virtio-balloon-pci"`
- **Example**: `"virtio-balloon-pci"`

**Example Ballooning Configuration:**
```yaml
ballooning:
  enabled: true
  free_page_reporting: true
  model: "virtio-balloon-pci"
```

## Enhanced Boot Configuration

The boot section has been enhanced with additional UEFI options.

### `boot.uefi_code` (optional)
- **Type**: String
- **Description**: Path to UEFI firmware code file
- **Default**: `"/usr/share/ovmf/OVMF.fd"`
- **Example**: `"/usr/share/ovmf/OVMF_CODE.fd"`

### `boot.uefi_vars` (optional)
- **Type**: String
- **Description**: Path to UEFI variables file
- **Default**: Uses system default
- **Example**: `"/var/lib/ezkvm/uefi/win11-vars.fd"`

### `boot.secure_boot` (optional)
- **Type**: Boolean
- **Description**: Enable UEFI Secure Boot
- **Default**: `false`
- **Example**: `true`

**Example Enhanced UEFI Configuration:**
```yaml
boot:
  firmware: "uefi"
  uefi_code: "/usr/share/ovmf/OVMF_CODE.secboot.fd"
  uefi_vars: "/var/lib/ezkvm/uefi/win11-vars.fd"
  secure_boot: true
  boot_order: ["disk", "network"]
```

## Complete Example Configuration

```yaml
name: "windows-11-gaming"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 16384
  vcpus: 8
  cpu_model: "host"
  cpu_features:
    - "+vmx"
    - "+svm"

boot:
  firmware: "uefi"
  uefi_code: "/usr/share/ovmf/OVMF_CODE.secboot.fd"
  uefi_vars: "/var/lib/ezkvm/uefi/win11-vars.fd"
  secure_boot: true
  boot_order: ["disk"]

devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/win11.qcow2"
      interface: "virtio"
      format: "qcow2"
  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "bridge=br0"
      mac: "52:54:00:12:34:56"
  displays:
    - type: "virtio-gpu"
      vram: 128

tpm:
  version: "2.0"
  backend: "emulator"
  state_path: "/var/lib/ezkvm/tpm/win11-tpm.state"

guest_agent:
  enabled: true
  socket_path: "/var/lib/ezkvm/qga/win11.sock"

ballooning:
  enabled: true
  free_page_reporting: true

options:
  enable_kvm: true
  daemonize: false
```

## Hardware Passthrough Configuration

### VFIO-PCI Device Passthrough (`hostpci`)

The `hostpci` section configures PCI device passthrough using VFIO (Virtual Function I/O). This allows direct hardware access for high-performance devices like GPUs.

#### `device` (required)
- **Type**: String
- **Description**: PCI device address in the format `XXXX:XX:XX.X`
- **Example**: `"0000:03:00.0"`

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the device
- **Example**: `"hostpci0"`

#### `pcie` (optional)
- **Type**: Boolean
- **Description**: Enable PCIe configuration
- **Default**: `false`
- **Example**: `true`

#### `x_vga` (optional)
- **Type**: Boolean
- **Description**: Enable VGA passthrough (for GPU devices)
- **Default**: `false`
- **Example**: `true`

#### `romfile` (optional)
- **Type**: String
- **Description**: Path to ROM file for the device
- **Example**: `"/var/lib/ezkvm/vbios/gpu.rom"`

**Example VFIO-PCI Configuration:**
```yaml
hostpci:
  - device: "0000:03:00.0"
    id: "gpu0"
    pcie: true
    x_vga: true
    romfile: "/var/lib/ezkvm/vbios/rx7700s.rom"
  - device: "0000:03:00.1"
    id: "gpu0-audio"
    pcie: true
```

### USB Device Passthrough (`usb_devices`)

The `usb_devices` section configures USB device passthrough for peripherals.

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the USB device
- **Example**: `"usb0"`

#### `host` (required)
- **Type**: String
- **Description**: USB device specification (bus-port or vendor:product)
- **Examples**: `"1-2.3"`, `"1234:5678"`

#### `bus` (optional)
- **Type**: String
- **Description**: USB controller bus to attach to
- **Default**: Auto-assigned
- **Example**: `"xhci.0"`

#### `port` (optional)
- **Type**: String
- **Description**: USB port number
- **Example**: `"1"`

**Example USB Device Configuration:**
```yaml
usb_devices:
  - id: "keyboard"
    host: "1-2.1"
  - id: "mouse"
    host: "1-2.2"
  - id: "webcam"
    host: "046d:0825"
```

## Display & Audio Protocols Configuration

### SPICE Display Support (`spice`)

The `spice` section configures SPICE (Simple Protocol for Independent Computing Environments) for remote desktop access and audio streaming.

#### `enabled` (optional)
- **Type**: Boolean
- **Description**: Enable SPICE display server
- **Default**: `true`
- **Example**: `true`

#### `port` (optional)
- **Type**: Integer
- **Description**: SPICE server port number
- **Default**: `5900`
- **Example**: `5901`

#### `addr` (optional)
- **Type**: String
- **Description**: SPICE server listen address
- **Default**: `"127.0.0.1"`
- **Example**: `"0.0.0.0"`

#### `disable_ticketing` (optional)
- **Type**: Boolean
- **Description**: Disable password authentication
- **Default**: `false`
- **Example**: `true`

#### `audio` (optional)
- **Type**: Boolean
- **Description**: Enable SPICE audio streaming
- **Default**: `false`
- **Example**: `true`

#### `vdagent` (optional)
- **Type**: Boolean
- **Description**: Enable vdagent for clipboard sharing
- **Default**: `true`
- **Example**: `true`

**Example SPICE Configuration:**
```yaml
spice:
  enabled: true
  port: 5901
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true
  vdagent: true
```

### Looking Glass Shared Memory (`ivshmem`)

The `ivshmem` section configures shared memory for Looking Glass, a low-latency KVM frame relay for gaming.

#### `enabled` (optional)
- **Type**: Boolean
- **Description**: Enable Looking Glass shared memory
- **Default**: `true`
- **Example**: `true`

#### `size` (optional)
- **Type**: Integer
- **Description**: Shared memory size in MiB
- **Default**: `32`
- **Example**: `64`

#### `vectors` (optional)
- **Type**: Integer
- **Description**: Number of MSI-X vectors
- **Default**: `1`
- **Example**: `2`

#### `id` (optional)
- **Type**: String
- **Description**: Device identifier
- **Default**: `"ivshmem0"`
- **Example**: `"looking-glass"`

**Example ivshmem Configuration:**
```yaml
ivshmem:
  enabled: true
  size: 64
  vectors: 2
  id: "looking-glass"
```

## Complete Remote Gaming VM Example Configuration

```yaml
name: "windows-11-remote-gaming"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 16384
  vcpus: 8
  cpu_model: "host"
  cpu_features:
    - "+vmx"
    - "+svm"

boot:
  firmware: "uefi"
  uefi_code: "/usr/share/ovmf/OVMF_CODE.secboot.fd"
  uefi_vars: "/var/lib/ezkvm/uefi/win11-vars.fd"
  secure_boot: true
  boot_order: ["disk"]

devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/win11.qcow2"
      interface: "virtio"
      format: "qcow2"
  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "bridge=br0"
      mac: "52:54:00:12:34:56"
  displays:
    - type: "qxl"
      vram: 128

# Phase 1 Features
tpm:
  version: "2.0"
  backend: "emulator"
  state_path: "/var/lib/ezkvm/tpm/win11-tpm.state"

guest_agent:
  enabled: true
  socket_path: "/var/lib/ezkvm/qga/win11.sock"

ballooning:
  enabled: true
  free_page_reporting: true

# Phase 2 Features - Hardware Passthrough
hostpci:
  - device: "0000:03:00.0"
    id: "gpu0"
    pcie: true
    x_vga: true
    romfile: "/var/lib/ezkvm/vbios/rx7700s.rom"
  - device: "0000:03:00.1"
    id: "gpu0-audio"
    pcie: true

usb_devices:
  - id: "keyboard"
    host: "1-2.1"
  - id: "mouse"
    host: "1-2.2"
  - id: "controller"
    host: "045e:028e"

```

## Storage Enhancements Configuration

### Enhanced Drive Configuration

The drive configuration has been enhanced with advanced storage options.

#### `discard` (optional)
- **Type**: Boolean
- **Description**: Enable discard (TRIM) support for SSD storage optimization
- **Default**: `false`
- **Example**: `true`

#### `ssd` (optional)
- **Type**: Boolean
- **Description**: Enable SSD emulation for better performance characteristics
- **Default**: `false`
- **Example**: `true`

#### `controller` (optional)
- **Type**: String
- **Description**: SCSI controller to attach the drive to (for SCSI interface)
- **Example**: `"scsi0"`

**Example Enhanced Drive Configuration:**
```yaml
devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/win11.qcow2"
      interface: "scsi"
      format: "qcow2"
      discard: true
      ssd: true
      controller: "scsi0"
```

### SCSI Controller Configuration (`scsi_controllers`)

The `scsi_controllers` section configures advanced SCSI controllers for better storage performance.

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the controller
- **Example**: `"scsi0"`

#### `type` (optional)
- **Type**: String
- **Description**: Controller type
- **Allowed values**: `"virtio-scsi-pci"`, `"pvscsi"`, `"lsi"`, `"lsi53c895a"`, `"megasas"`, `"megasas-gen2"`
- **Default**: `"virtio-scsi-pci"`
- **Example**: `"pvscsi"`

#### `iothread` (optional)
- **Type**: String
- **Description**: I/O thread for virtio-scsi controllers
- **Example**: `"iothread0"`

#### `max_targets` (optional)
- **Type**: Integer
- **Description**: Maximum number of SCSI targets
- **Default**: Controller default
- **Example**: `256`

**Example SCSI Controller Configuration:**
```yaml
scsi_controllers:
  - id: "scsi0"
    type: "virtio-scsi-pci"
    iothread: "iothread0"
    max_targets: 256
  - id: "scsi1"
    type: "pvscsi"
```

### iSCSI Disk Configuration (`iscsi_disks`)

The `iscsi_disks` section configures iSCSI storage for enterprise storage connectivity.

#### `id` (required)
- **Type**: String
- **Description**: Unique identifier for the disk
- **Example**: `"iscsi0"`

#### `portal` (required)
- **Type**: String
- **Description**: iSCSI target portal in format `host:port`
- **Example**: `"192.168.1.100:3260"`

#### `target` (required)
- **Type**: String
- **Description**: iSCSI target IQN
- **Example**: `"iqn.2020-01.com.example:storage.target1"`

#### `lun` (optional)
- **Type**: Integer
- **Description**: Logical Unit Number
- **Default**: `0`
- **Example**: `1`

#### `initiator` (optional)
- **Type**: String
- **Description**: Initiator IQN for CHAP authentication
- **Example**: `"iqn.2020-01.com.example:initiator"`

#### `username` (optional)
- **Type**: String
- **Description**: Username for CHAP authentication
- **Example**: `"myuser"`

#### `password` (optional)
- **Type**: String
- **Description**: Password for CHAP authentication
- **Example**: `"mypass"`

#### `controller` (optional)
- **Type**: String
- **Description**: SCSI controller to attach the disk to
- **Example**: `"scsi0"`

**Example iSCSI Disk Configuration:**
```yaml
iscsi_disks:
  - id: "iscsi-data"
    portal: "192.168.1.100:3260"
    target: "iqn.2020-01.com.example:storage.target1"
    lun: 0
    initiator: "iqn.2020-01.com.example:initiator"
    username: "myuser"
    password: "mypass"
    controller: "scsi0"
```

## Complete Enterprise Storage VM Example Configuration

```yaml
name: "enterprise-storage-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 8192
  vcpus: 4
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk"]

devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/enterprise.qcow2"
      interface: "virtio"
      format: "qcow2"
  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "bridge=br0"

# Phase 4 Features - Storage Enhancements
scsi_controllers:
  - id: "scsi0"
    type: "virtio-scsi-pci"
    iothread: "iothread0"
    max_targets: 256

iscsi_disks:
  - id: "iscsi-data1"
    portal: "192.168.1.100:3260"
    target: "iqn.2020-01.com.example:storage.target1"
    lun: 0
    controller: "scsi0"
  - id: "iscsi-data2"
    portal: "192.168.1.100:3260"
    target: "iqn.2020-01.com.example:storage.target2"
    lun: 0
    controller: "scsi0"

options:
  enable_kvm: true
  daemonize: false
```

# Phase 3 Features - Display & Audio Protocols
spice:
  enabled: true
  port: 5901
  addr: "0.0.0.0"
  disable_ticketing: true
  audio: true
  vdagent: true

ivshmem:
  enabled: true
  size: 64
  vectors: 2
  id: "looking-glass"

options:
  enable_kvm: true
  daemonize: false
```

# Phase 5 Features - System Management

## QMP Monitoring Configuration (`qmp`)

The `qmp` section configures QEMU Machine Protocol monitoring for advanced VM management.

#### `enabled` (optional)
- **Type**: Boolean
- **Description**: Enable QMP monitoring interface
- **Default**: `true`
- **Example**: `true`

#### `socket_path` (optional)
- **Type**: String
- **Description**: Path for the QMP socket connection
- **Default**: `"/var/run/qemu-monitor.sock"` (Unix) or `"127.0.0.1:4444"` (TCP)
- **Example**: `"/var/run/ezkvm/my-vm-monitor.sock"`

#### `socket_type` (optional)
- **Type**: String
- **Description**: Type of socket to use for QMP
- **Allowed values**: `"unix"`, `"tcp"`
- **Default**: `"unix"`
- **Example**: `"tcp"`

**Example QMP Configuration:**
```yaml
qmp:
  enabled: true
  socket_path: "/var/run/ezkvm/my-vm-monitor.sock"
  socket_type: "unix"
```

## SMBIOS System Information Configuration (`smbios`)

The `smbios` section configures System Management BIOS information for system identification.

#### `manufacturer` (optional)
- **Type**: String
- **Description**: System manufacturer name
- **Example**: `"QEMU"`

#### `product` (optional)
- **Type**: String
- **Description**: Product name
- **Example**: `"Standard PC (Q35 + ICH9, 2009)"`

#### `version` (optional)
- **Type**: String
- **Description**: System version
- **Example**: `"1.0"`

#### `serial` (optional)
- **Type**: String
- **Description**: System serial number
- **Example**: `"ABC123"`

#### `uuid` (optional)
- **Type**: String
- **Description**: System UUID in format XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
- **Example**: `"12345678-1234-1234-1234-123456789abc"`

#### `sku` (optional)
- **Type**: String
- **Description**: System SKU number
- **Example**: `"SKU123"`

#### `family` (optional)
- **Type**: String
- **Description**: System family name
- **Example**: `"Virtual Machine"`

#### `vm_generation_id` (optional)
- **Type**: String
- **Description**: VM generation ID for Windows Server 2016+ in UUID format
- **Example**: `"87654321-4321-4321-4321-cba987654321"`

**Example SMBIOS Configuration:**
```yaml
smbios:
  manufacturer: "QEMU"
  product: "Enterprise Server"
  version: "2.0"
  serial: "ES-001"
  uuid: "12345678-1234-1234-1234-123456789abc"
  sku: "ENT-SRV-001"
  family: "Virtual Machine"
  vm_generation_id: "87654321-4321-4321-4321-cba987654321"
```

## NUMA Topology Configuration (`numa`)

The `numa` section configures Non-Uniform Memory Access topology for high-performance systems.

#### `id` (required)
- **Type**: Integer
- **Description**: Unique NUMA node identifier
- **Example**: `0`

#### `memory` (required)
- **Type**: Integer
- **Description**: Memory size for this NUMA node in MiB
- **Example**: `4096`

#### `cpus` (required)
- **Type**: Array of Integers
- **Description**: List of CPU cores assigned to this NUMA node
- **Example**: `[0, 1, 2, 3]`

#### `host_node` (optional)
- **Type**: Integer
- **Description**: Host NUMA node to bind this VM node to (for host-passthrough)
- **Example**: `0`

**Example NUMA Configuration:**
```yaml
numa:
  - id: 0
    memory: 4096
    cpus: [0, 1]
    host_node: 0
  - id: 1
    memory: 4096
    cpus: [2, 3]
    host_node: 1
```

## Complete Enterprise Management VM Example Configuration

```yaml
name: "enterprise-management-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 8192
  vcpus: 4
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk"]

devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/management.qcow2"
      interface: "virtio"
      format: "qcow2"
  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "bridge=br0"

# Phase 5 Features - System Management
qmp:
  enabled: true
  socket_path: "/var/run/ezkvm/management-monitor.sock"
  socket_type: "unix"

smbios:
  manufacturer: "QEMU"
  product: "Enterprise Management Server"
  version: "1.0"
  serial: "EMS-001"
  uuid: "12345678-1234-1234-1234-123456789abc"
  sku: "ENT-MGMT-001"
  family: "Virtual Machine"
  vm_generation_id: "87654321-4321-4321-4321-cba987654321"

numa:
  - id: 0
    memory: 4096
    cpus: [0, 1]
  - id: 1
    memory: 4096
    cpus: [2, 3]

options:
  enable_kvm: true
  daemonize: false
```

# Phase 6 Features - KVM Optimizations

## Hyper-V Enlightenments Configuration (`hyperv`)

The `hyperv` section configures Hyper-V enlightenments for improved Windows VM performance on KVM.

#### `enabled` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V enlightenments
- **Default**: `true`
- **Example**: `true`

#### `relaxed` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V relaxed timing
- **Default**: `true`
- **Example**: `true`

#### `vapic` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V virtual APIC
- **Default**: `true`
- **Example**: `true`

#### `time` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V time reference counter
- **Default**: `true`
- **Example**: `true`

#### `crash` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V crash MSRs
- **Default**: `false`
- **Example**: `true`

#### `reset` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V reset MSR
- **Default**: `false`
- **Example**: `true`

#### `vendor_id` (optional)
- **Type**: String
- **Description**: Hyper-V vendor ID spoofing (max 12 characters)
- **Example**: `"Microsoft Hv"`

#### `frequencies` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V frequency MSRs
- **Default**: `false`
- **Example**: `true`

#### `reenlightenment` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V reenlightenment MSRs
- **Default**: `false`
- **Example**: `true`

#### `tlbflush` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V TLB flush
- **Default**: `false`
- **Example**: `true`

#### `ipi` (optional)
- **Type**: Boolean
- **Description**: Enable Hyper-V IPI optimization
- **Default**: `false`
- **Example**: `true`

#### `spinlock_retry` (optional)
- **Type**: Integer
- **Description**: Hyper-V spinlock retry count (1-4294967295)
- **Example**: `8191`

**Example Hyper-V Configuration:**
```yaml
hyperv:
  enabled: true
  relaxed: true
  vapic: true
  time: true
  crash: true
  vendor_id: "Microsoft Hv"
  frequencies: true
  reenlightenment: true
  tlbflush: true
  ipi: true
  spinlock_retry: 8191
```

## Complete Windows Optimization VM Example Configuration

```yaml
name: "windows-optimized-vm"
backend: "qemu"

system:
  architecture: "x86_64"
  machine: "q35"
  memory: 8192
  vcpus: 4
  cpu_model: "host"

boot:
  firmware: "uefi"
  boot_order: ["disk"]

devices:
  drives:
    - id: "boot-disk"
      path: "/var/lib/ezkvm/images/windows.qcow2"
      interface: "virtio"
      format: "qcow2"
  networks:
    - id: "net0"
      model: "virtio-net"
      mode: "bridge=br0"

# Phase 6 Features - KVM Optimizations
hyperv:
  enabled: true
  relaxed: true
  vapic: true
  time: true
  crash: true
  vendor_id: "Microsoft Hv"
  frequencies: true
  reenlightenment: true
  tlbflush: true
  ipi: true
  spinlock_retry: 8191

options:
  enable_kvm: true
  daemonize: false
```

## Validation

Configuration files are validated at load time. Common validation rules include:

- Required fields must be present
- Architecture must be supported
- Memory and CPU limits are enforced
- Device IDs must be unique
- File paths are checked for accessibility (when possible)
- Network modes are validated
- CPU models are verified against architecture

Invalid configurations will produce detailed error messages indicating the specific validation failure.