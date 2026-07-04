src/README.md

# EZKVM

## High level requirements
- Easily create a qemu based VM on any linux distro.
- Easily convert between ezkvm, proxmox, libvirt and qemu commandline syntax.

## Command syntax requirements

### Importing
1. Import ezkvm config file and run validation:
```ezkvm import --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --validate```

2. Import ezkvm config file and show runtime:
```ezkvm import --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --show-runtime```

3. Import ezkvm config file and export qemu commandline:
```ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --output.type qemu```
This command imports the given ezkvm config file through the `config_format/ezkvm` importer and exports via the `config_format/qemu_cmd` exporter to ```<name>.qemu.cmd```.
Note that the vm input path is provided with ```--input.vm```, and host settings are provided with ```--input.host```.
Current ezkvm schema coverage in this flow includes machine/cpu/memory, boot firmware, TPM, storage/network bus devices, host-level resources, and optional display/audio/guest-agent sections when present in the input YAML. GPU is now modeled as a PCI/PCIe device in `virtual_machine.devices` (headless is represented by omitting GPU devices).

4. Import proxmox config file, and save as ezkvm yaml file:
```ezkvm convert --input.type proxmox --input.storage /etc/pve/storage.cfg --input.vm <name>.conf --output.type ezkvm --output.host /etc/ezkvm/host.yaml --output.vm <name>.yaml```
This command imports the given proxmox config file through the `config_format/proxmox` importer and exports via the `config_format/ezkvm` exporter to ```<name>.yaml```.
Note that ```--input.storage``` is used to locate the proxmox ```storage.cfg``` file which explains how to translate storage paths in the proxmox vm config file to actual device locations.
Current import coverage includes machine/cpu/memory plus baseline UEFI firmware disk, TPM state, SCSI disks, and bridged network adapters where present in the source config.
Current Proxmox mapping also handles `agent` -> guest-agent, `vga` -> runtime GPU device selection, and `spice` -> display when those fields are present and representable.
Current Proxmox mapping also preserves hugepages/NUMA memory policy (`hugepages`, `numa`) into runtime memory backend rendering.
Current Proxmox mapping now models `hostpci*` passthrough devices via `host.resources[].pcie` entries, and links VM-side passthrough devices through `virtual_machine.devices[].pcie.resource`.
Current Proxmox mapping also imports and exports lifecycle/monitoring configuration (`pidfile`, `daemonize`, `no-shutdown`, and QMP monitor socket fields) through the runtime model and QEMU rendering path.

5. Import ezkvm config and export as libvirt xml:
```ezkvm convert --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm <name>.yaml --output.type libvirt --output.vm <name>.xml```

6. Standalone export subcommand:
```ezkvm export --output.type proxmox --output.storage /etc/pve/storage.cfg --output.vm <name>.conf```
This subcommand is currently a placeholder and returns an error. Use `convert` for import+export flow.

### Runtime operations
1. Show runtime:
```ezkvm show-runtime --name <name>```

2. Start vm:
```ezkvm start --name <name>```

3. Stop vm:
```ezkvm stop --name <name>```
Uses QMP `quit` on the configured lifecycle monitor socket (`lifecycle.qmp_socket`).

4. Reset vm:
```ezkvm reset --name <name>```
Uses QMP `system_reset` on the configured lifecycle monitor socket (`lifecycle.qmp_socket`).

5. Shutdown vm:
```ezkvm shutdown --name <name>```
Uses QMP `system_powerdown` on the configured lifecycle monitor socket (`lifecycle.qmp_socket`).

`stop`, `reset`, and `shutdown` require lifecycle configuration with a QMP socket path. If unset, these commands return an error.

## Design

### Config Format Architecture

The config format system uses a **bidirectional stage-based pipeline** enabling lossless conversions between VM specification formats.

#### Stage Pattern

Each config format (ezkvm, Proxmox, libvirt, QEMU) implements a pair of stages:

- **RuntimeBuilder** (`stages/runtime_builder.rs`): Converts format-specific schema → `RuntimeModel`
  - Parses and validates input schema
  - Resolves resources and allocates buses
  - Registers devices across bus topologies
  
- **SchemaBuilder** (`stages/schema_builder.rs`): Converts `RuntimeModel` → format-specific schema
  - Traverses runtime model devices deterministically
  - Synthesizes deterministic resource IDs (e.g., `storage0`, `net0`, `hostpci0`)
  - Reconstructs schema representation for output

#### Ezkvm Bidirectional Flow

The ezkvm format provides **lossless round-trip conversion**:

```
┌─────────────────────────────────────────────────────────────┐
│ ezkvm YAML (schema)                                         │
│ ├─ metadata                                                 │
│ ├─ host: { display, audio, resources[] }                  │
│ └─ virtual_machine: { machine, cpu, memory, devices[] }   │
└─────────────┬───────────────────────────────────────────────┘
              │ RuntimeBuilder (stages/runtime_builder.rs)
              │ - Resolve resources → ResourceMaps
              │ - Initialize chipset → BusRegister
              │ - Register devices across buses
              ↓
┌─────────────────────────────────────────────────────────────┐
│ RuntimeModel (internal representation)                      │
│ ├─ CPUs, memory, chipset, boot firmware                    │
│ ├─ Display, audio, TPM, guest agent                        │
│ └─ Buses: PCIe[], SATA[], IDE[], SCSI[], USB[]           │
└─────────────┬───────────────────────────────────────────────┘
              │ SchemaBuilder (stages/schema_builder.rs)
              │ - Traverse runtime buses deterministically
              │ - Synthesize ResourceIndex (storage0, net0, hostpci0)
              │ - Split display/audio to host vs VM
              ↓
┌─────────────────────────────────────────────────────────────┐
│ ezkvm YAML (schema) — semantically identical to input       │
└─────────────────────────────────────────────────────────────┘
```

#### Resource Binding Pattern

Resources use a **dual-reference pattern** for safe sharing:

1. **Resource Declaration** (`host.resources[]`):
   ```yaml
   resources:
     - storage: { file: "/var/vm/disk.qcow2" }  # synthesized as storage0
     - network: { name: "eth0", bridge: "br0" }  # synthesized as net0
   ```

2. **Device Reference** (`virtual_machine.devices[].*.resource`):
   ```yaml
   devices:
     - sata:
         address: 0
         type: { ssd: { resource: "storage0" } }  # reference to declared resource
   ```

This pattern ensures:
- Resources are defined once, referenced many times
- Deterministic ID synthesis across render passes
- Schema validation against resource availability

#### Bus Topology

Buses are created by the chipset and populated during device registration:

- **PCIe Buses**: Primary mesh for compute devices
  - Bus 0: Root complex, houses passthrough controllers, NICs, GPUs, storage controllers
  - Bus N: Additional endpoint buses (extensible)
  - Supports hotplug (reserved in runtime)

- **Storage Buses** (SATA, IDE, SCSI):
  - Backed by controllers registered on PCIe
  - SCSI: Dynamic controller creation if referenced bus has no controller

- **USB Buses**:
  - Separate topology from PCIe
  - Host and device ports

Device registration is explicit per-bus to avoid ambiguity.

#### Conversion Examples

**Import ezkvm, validate, and show runtime:**
```bash
ezkvm import --input.type ezkvm \
  --input.host /etc/ezkvm/host.yaml \
  --input.vm myvm.yaml \
  --show-runtime
```
(Uses `EzkvmRuntimeBuilder::build()` to parse and construct `RuntimeModel`)

**Convert ezkvm to QEMU command line:**
```bash
ezkvm convert \
  --input.type ezkvm --input.host /etc/ezkvm/host.yaml --input.vm myvm.yaml \
  --output.type qemu --output.vm myvm.qemu.cmd
```
(Chains: `EzkvmRuntimeBuilder` → schema→runtime + `QemuCmdSchemaBuilder` → runtime→schema)

**Convert Proxmox to ezkvm:**
```bash
ezkvm convert \
  --input.type proxmox --input.storage /etc/pve/storage.cfg --input.vm myvm.conf \
  --output.type ezkvm --output.host /etc/ezkvm/host.yaml --output.vm myvm.yaml
```
(Chains: `ProxmoxRuntimeBuilder` → schema→runtime + `EzkvmSchemaBuilder` → runtime→schema)

```plantuml
@startuml
title Config Format Pipeline: Bidirectional Conversions via RuntimeModel

package "Input Formats" {
  component Ezkvm
  component Proxmox
  component Libvirt
  component QemuCmd
}

package "Stage Builders (RuntimeBuilder)" {
  component "EzkvmRuntimeBuilder"
  component "ProxmoxRuntimeBuilder"
  component "LibvirtRuntimeBuilder"
  component "QemuCmdRuntimeBuilder"
}

package "Internal Representation" {
  component RuntimeModel
}

package "Stage Builders (SchemaBuilder)" {
  component "EzkvmSchemaBuilder"
  component "ProxmoxSchemaBuilder"
  component "LibvirtSchemaBuilder"
  component "QemuCmdSchemaBuilder"
}

package "Output Formats" {
  component EzkvmOut [Output Ezkvm]
  component ProxmoxOut [Output Proxmox]
  component LibvirtOut [Output Libvirt]
  component QemuCmdOut [Output QemuCmd]
}

Ezkvm --> EzkvmRuntimeBuilder : schema->runtime
ProxmoxSchemaBuilder --> ProxmoxOut : runtime->schema
Libvirt --> LibvirtRuntimeBuilder : schema->runtime
QemuCmd --> QemuCmdRuntimeBuilder : schema->runtime

EzkvmRuntimeBuilder --> RuntimeModel : build
ProxmoxRuntimeBuilder --> RuntimeModel : build
LibvirtRuntimeBuilder --> RuntimeModel : build
QemuCmdRuntimeBuilder --> RuntimeModel : build

RuntimeModel --> EzkvmSchemaBuilder
RuntimeModel --> ProxmoxSchemaBuilder
RuntimeModel --> LibvirtSchemaBuilder
RuntimeModel --> QemuCmdSchemaBuilder

EzkvmSchemaBuilder --> EzkvmOut
ProxmoxSchemaBuilder --> ProxmoxOut
LibvirtSchemaBuilder --> LibvirtOut
QemuCmdSchemaBuilder --> QemuCmdOut

note right of RuntimeModel
  Canonical intermediate representation
  - CPUs, memory, boot, TPM
  - Display, audio, guest agent
  - Bus topology (PCIe, SATA, IDE, SCSI, USB)
  - Device attachment points
end note

note bottom of ProxmoxSchemaBuilder
  Deterministic resource ID synthesis:
  storage0, net0, hostpci0, etc.
  Ensures consistent YAML across renders
end note
@enduml
```

```
```