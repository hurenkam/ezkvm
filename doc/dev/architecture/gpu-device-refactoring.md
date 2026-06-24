# GPU as Device Refactoring Proposal

**Status:** Proposed  
**Date:** June 24, 2026  
**Purpose:** Move GPU from top-level `VirtualMachine` field to bus-attached device in `devices` section

## Rationale

1. **Consistency**: GPU is hardware attached to a bus, just like every other device → should live in `devices` section
2. **No duplication**: Top-level `gpu` field and bus-attached GPU devices would be redundant
3. **Headless elegance**: Absence of any GPU device naturally represents headless; no need for explicit `Headless` variant
4. **Single source of truth**: Devices defined in one place (`devices` section), not scattered between top-level fields and devices

## Architecture Overview

### Three-Layer Schema Structure

```
host:                                       # Host-level execution policy
  display: [Vnc|Spice|LookingGlass|...]    # Runtime frontend choice
  audio: [Alsa|PulseAudio|PipeWire]        # Runtime audio backend
  resources: [...]                          # Shared resources (passthrough GPU, shared memory, etc.)

virtual_machine:                            # Guest hardware intent
  machine: { ... }
  cpu: { ... }
  memory: { ... }
  boot: { ... }
  tpm: [optional]
  guest_agent: [optional]
  devices: [...]                            # ALL hardware: GPU, audio controllers, storage, network, etc.
```

**Separation Principle:**
- `host.display` = "How will I see the guest console?" (host concern)
- `virtual_machine.devices` = "What hardware will the guest see?" (guest hardware)

### VirtualMachine Structure (Simplified)

```rust
pub struct VirtualMachine {
    pub machine: Machine,
    pub cpu: Option<Cpu>,
    pub memory: Memory,
    pub boot: Boot,
    pub tpm: Option<Tpm>,
    // REMOVED: gpu field (GPU now in devices section)
    pub guest_agent: Option<GuestAgent>,
    pub devices: Vec<Device>,
}
```

## GPU as Bus Devices

### PcieDeviceType Variants

```rust
pub enum PcieDeviceType {
    PvScsiController { ... },
    // GPU variants:
    StandardGpu { },                        // emits: -device VGA
    VirtioGpu { },                          // emits: -device virtio-gpu-pci
    PassthroughGpu { resource: String },    // emits: -device vfio-pci
    // Audio devices:
    Ich9IntelHda { codec: HdaCodec },
    IvshmemPlain { resource: String },
}
```

### PciDeviceType Variants

```rust
pub enum PciDeviceType {
    // GPU variants:
    QxlGpu { },                             // emits: -device qxl
    // Audio device:
    Ac97 { },                               // emits: -device AC97
}
```

## YAML Examples

### Example 1: Standard GPU (PCIe)

```yaml
host:
  vnc:
    port: 5900
    listen: "0.0.0.0"
  resources: []

virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  cpu:
    cores: 8
    threads: 2
  memory:
    value: 16
    unit: "GiB"
  devices:
    - pcie: { standard_gpu: {} }
    - pcie: { ich9_intel_hda: { codec: "Realtek" } }
```

### Example 2: VirtIO GPU with Looking Glass

```yaml
host:
  looking_glass:
    port: 5900
    listen: "127.0.0.1"
    disable_ticketing: true
  resources:
    - name: "lg-shared-mem"
      type: "shared_memory"
      size_mb: 64

virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  devices:
    - pcie: { virtio_gpu: {} }
    - pcie: { ivshmem_plain: { resource: "lg-shared-mem" } }
```

### Example 3: QXL GPU (PCI, legacy)

```yaml
host:
  spice:
    port: 5900
    listen: "127.0.0.1"
    disable_ticketing: true
  resources: []

virtual_machine:
  machine:
    family: "pc"
    chipset: "i440fx"
  devices:
    - pci: { qxl_gpu: {} }
```

### Example 4: Headless (NO GPU device)

```yaml
host:
  resources: []

virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    value: 8
    unit: "GiB"
  devices:
    - pcie: { pv_scsi_controller: {} }
    # NO GPU device = headless
```

### Example 5: Passthrough GPU

```yaml
host:
  resources:
    - name: "nvidia-rtx4090"
      type: "pci_passthrough"
      pci_address: "02:00.0"

virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  devices:
    - pcie: { passthrough_gpu: { resource: "nvidia-rtx4090" } }
```

## Implementation Changes

### Remove From VirtualMachine

- Delete `gpu: Option<Gpu>` field from [src/config_format/ezkvm/virtual_machine.rs](../../config_format/ezkvm/virtual_machine.rs)

### Add GPU Variants To

- [src/runtime_model/pcie.rs](../../runtime_model/pcie.rs) - `StandardGpu`, `VirtioGpu`, `PassthroughGpu`
- [src/runtime_model/pci.rs](../../runtime_model/pci.rs) - `QxlGpu`

### Update Device Rendering In

- [src/runtime_model/pcie.rs](../../runtime_model/pcie.rs) - GPU device name mappings to qemu args
- [src/runtime_model/pci.rs](../../runtime_model/pci.rs) - GPU device name mappings to qemu args
- [src/config_format/ezkvm/builder.rs](../../config_format/ezkvm/builder.rs) - remove `gpu` field wiring
- [src/config_format/ezkvm/renderer.rs](../../config_format/ezkvm/renderer.rs) - remove `gpu` field extraction
- [src/config_format/qemu_cmd/schema_builder.rs](../../config_format/qemu_cmd/schema_builder.rs) - GPU args now via device rendering

### May Deprecate

- [src/runtime_model/gpu.rs](../../runtime_model/gpu.rs) - no longer needed (GPU variants now inline as device types)

## Builder Logic

```rust
pub fn build_virtual_machine(vm: &VirtualMachine) -> Result<RuntimeModel, String> {
    // GPU is now handled as part of device tree building
    let devices = build_devices_from_schema(&vm.devices)?;
    
    // Validate: at most one GPU device
    let gpu_device_count = devices.iter()
        .filter(|d| d.is_gpu_device())
        .count();
    if gpu_device_count > 1 {
        return Err("Only one GPU device allowed per VM".to_string());
    }
    
    RuntimeModel {
        devices,
        guest_agent: vm.guest_agent.clone(),
        // ... other fields
    }
}

// Helper
impl Device {
    fn is_gpu_device(&self) -> bool {
        matches!(self,
            Device::Pcie { pcie } if pcie.is_gpu(),
            Device::Pci { pci } if pci.is_gpu(),
        )
    }
}
```

## Renderer Logic

```rust
pub fn render_to_schema(model: &RuntimeModel) -> VirtualMachine {
    VirtualMachine {
        devices: render_all_devices(&model.devices),
        // No separate gpu field to extract
        guest_agent: model.guest_agent.clone(),
        machine: render_machine(&model.machine),
        // ...
    }
}
```

## QEMU Command Generation

GPU args are now emitted as part of device rendering:

```rust
pub fn render_device_to_args(device: &Device) -> Vec<String> {
    match device {
        Device::Pcie { pcie } => match pcie {
            PcieDeviceType::StandardGpu {} => vec!["-device".to_string(), "VGA".to_string()],
            PcieDeviceType::VirtioGpu {} => vec!["-device".to_string(), "virtio-gpu-pci".to_string()],
            // ... other devices
        }
        Device::Pci { pci } => match pci {
            PciDeviceType::QxlGpu {} => vec!["-device".to_string(), "qxl".to_string()],
            // ... other devices
        }
    }
}
```

## Migration from Previous Schema

### Old Structure

```yaml
virtual_machine:
  gpu: { standard: {} }
  devices: [...]
```

### New Structure

```yaml
virtual_machine:
  devices:
    - pcie: { standard_gpu: {} }
    - ... (other devices)
```

### Headless Migration

```yaml
# OLD
virtual_machine:
  gpu: { headless: {} }
  devices: [...]

# NEW
virtual_machine:
  devices: [...]  # Just omit GPU device
```

## Validation Rules

1. **At most one GPU device**: Scan `devices` for GPU entities; error if count > 1
2. **GPU passthrough resource exists**: `PassthroughGpu.resource` must reference `host.resources` entry
3. **Chipset GPU compatibility**:
   - Q35: Standard, Virtio, Passthrough, QXL (via PCIe bridge)
   - I440FX: Standard, QXL (PCI)
4. **Audio device compatibility**: Validated as part of device tree
5. **Ivshmem resource exists**: `IvshmemPlain.resource` must reference `host.resources` entry

## Benefits

- ✅ Single hardware definition location (devices section)
- ✅ Headless is natural (no GPU device)
- ✅ Consistent with all other devices (no special top-level fields)
- ✅ Clearer separation: host runtime policy (host section) vs guest hardware (virtual_machine)
- ✅ Simpler builder and renderer logic
- ✅ Fewer edge cases in validation
