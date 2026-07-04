# Runtime model

## Requirements

- Represent a runnable, bus-oriented VM model derived from canonical runtime config.
- Keep conversion deterministic: the same runtime config must produce the same runtime topology.
- Resolve resource references (`storage`, `network`, `usb_device`) during model construction.
- Fail fast with actionable errors when required referenced resources are missing.
- Keep bus/device registration explicit per bus family (PCIe, PCI, USB, SATA, IDE, SCSI).

## Design

`RuntimeModel` is built from `RuntimeConfig` via `TryFrom<RuntimeConfig>` in [src/runtime_model/model.rs](src/runtime_model/model.rs).

High-level flow:

1. Collect top-level resources into lookup maps keyed by `id`.
2. Instantiate chipset and register foundational buses.
3. Build `BootModel` using `BootModelBuilder`.
4. Build optional top-level runtime feature models (`Gpu`, `Display`, `Audio`, `GuestAgent`, `TPM`).
5. Iterate `virtual_machine.devices` and register each device on its target bus.
6. For resource-backed devices, resolve `resource` ids through builder helpers.

### Builder-based resource resolution

Runtime model uses dedicated builders to keep device/resource binding logic localized:

- [src/runtime_model/boot.rs](src/runtime_model/boot.rs): `BootModelBuilder`
- [src/runtime_model/display.rs](src/runtime_model/display.rs): `DisplayModelBuilder`
- [src/runtime_model/gpu.rs](src/runtime_model/gpu.rs): `GpuModelBuilder`
- [src/runtime_model/audio.rs](src/runtime_model/audio.rs): `AudioModelBuilder`
- [src/runtime_model/guest_agent.rs](src/runtime_model/guest_agent.rs): `GuestAgentModelBuilder`
- [src/runtime_model/tpm.rs](src/runtime_model/tpm.rs): `TpmModelBuilder`
- [src/runtime_model/sata.rs](src/runtime_model/sata.rs): `SataDeviceBuilder`
- [src/runtime_model/ide.rs](src/runtime_model/ide.rs): `IdeDeviceBuilder`
- [src/runtime_model/scsi.rs](src/runtime_model/scsi.rs): `ScsiDeviceBuilder`
- [src/runtime_model/usb.rs](src/runtime_model/usb.rs): `UsbDeviceBuilder`

Each builder accepts the parsed device/boot payload plus the relevant resource map and returns either:

- a concrete runtime model object (`BootModel`), or
- a bus device API object (`Arc<dyn ...DeviceApi>`), optionally wrapped in `Result` when resolution can fail.

### Resource binding behavior

- SATA/IDE/SCSI device variants carry storage resource ids and are resolved against storage resources.
- PCIe `virtio_net` optionally carries a network resource id and resolves against network resources.
- PCIe `virtio_net` additionally supports optional metadata for MAC address, queue sizing (`rx_queue_size`, `tx_queue_size`), and `vhost` offload preference.
- USB devices are constructed through `UsbDeviceBuilder`; usb resource map is already threaded through for feature growth.
- Boot model is built from boot config and storage resources.
- Gpu/display/audio/guest-agent are top-level optional runtime features; they are assembled independently from bus registration and contribute command-line arguments when present.

Missing referenced resources return descriptive errors during `RuntimeModel::try_from(...)` construction.

### Module boundaries

- [src/runtime_model/model.rs](src/runtime_model/model.rs): orchestration, bus registration, runtime assembly.
- [src/runtime_model/gpu.rs](src/runtime_model/gpu.rs): GPU model (`standard`, `qxl`, `virtio`, `headless`, `passthrough`).
- [src/runtime_model/display.rs](src/runtime_model/display.rs): display frontend model (`gtk`, `sdl`, `vnc`, `spice`, `looking_glass`).
- [src/runtime_model/audio.rs](src/runtime_model/audio.rs): audio backend and controller model.
- [src/runtime_model/guest_agent.rs](src/runtime_model/guest_agent.rs): guest agent channel model.
- [src/runtime_model/devices](src/runtime_model/devices): concrete device implementations (`Hdd`, `Ssd`, `Cdrom`, `VirtioNetController`, `PvScsiController`).
- Bus-family modules (`pcie.rs`, `pci.rs`, `usb.rs`, `sata.rs`, `ide.rs`, `scsi.rs`) define device/controller APIs, addresses, and builder logic.

### Class diagram: runtime assembly structure

```plantuml
@startuml
left to right direction
skinparam classAttributeIconSize 0

class RuntimeConfig
class RuntimeModel
class BusRegister
class BootModel

class BootModelBuilder {
	+build(boot, storage_resources) : Result<BootModel, String>
}
class SataDeviceBuilder {
	+build(device_type, storage_resources) : Result<Arc<SataDeviceApi>, String>
}
class IdeDeviceBuilder {
	+build(device_type, storage_resources) : Result<Arc<IdeDeviceApi>, String>
}
class ScsiDeviceBuilder {
	+build(device_type, storage_resources) : Result<Arc<ScsiDeviceApi>, String>
}
class UsbDeviceBuilder {
	+build(device_type, usb_resources) : Arc<UsbDeviceApi>
}

interface PcieDeviceApi
interface PciDeviceApi
interface UsbDeviceApi
interface SataDeviceApi
interface IdeDeviceApi
interface ScsiDeviceApi

class Hdd
class Ssd
class Cdrom
class VirtioNetController
class PvScsiController

RuntimeConfig --> RuntimeModel : TryFrom
RuntimeModel *-- BusRegister
RuntimeModel *-- BootModel

RuntimeModel ..> BootModelBuilder
RuntimeModel ..> SataDeviceBuilder
RuntimeModel ..> IdeDeviceBuilder
RuntimeModel ..> ScsiDeviceBuilder
RuntimeModel ..> UsbDeviceBuilder

SataDeviceBuilder ..> Hdd
SataDeviceBuilder ..> Ssd
SataDeviceBuilder ..> Cdrom
IdeDeviceBuilder ..> Hdd
IdeDeviceBuilder ..> Ssd
IdeDeviceBuilder ..> Cdrom
ScsiDeviceBuilder ..> Hdd
ScsiDeviceBuilder ..> Ssd
ScsiDeviceBuilder ..> Cdrom
RuntimeModel ..> VirtioNetController
RuntimeModel ..> PvScsiController

Hdd ..|> SataDeviceApi
Ssd ..|> SataDeviceApi
Cdrom ..|> SataDeviceApi
Hdd ..|> IdeDeviceApi
Ssd ..|> IdeDeviceApi
Cdrom ..|> IdeDeviceApi
Hdd ..|> ScsiDeviceApi
Ssd ..|> ScsiDeviceApi
Cdrom ..|> ScsiDeviceApi
VirtioNetController ..|> PcieDeviceApi
PvScsiController ..|> PcieDeviceApi
@enduml
```

### Sequence diagram: successful runtime model construction

Typical use case: storage-backed IDE/SATA/SCSI devices plus optional network-backed PCIe virtio-net.

```plantuml
@startuml
actor Caller
participant "RuntimeModel::try_from" as RM
participant "BootModelBuilder" as BootB
participant "SataDeviceBuilder" as SataB
participant "IdeDeviceBuilder" as IdeB
participant "ScsiDeviceBuilder" as ScsiB
participant "UsbDeviceBuilder" as UsbB

Caller -> RM : try_from(RuntimeConfig)
RM -> RM : Build resource maps by id
RM -> BootB : build(vm.boot, storage_resources)
BootB --> RM : BootModel

loop for each vm device
	alt PCIe pv_scsi
		RM -> RM : create PvScsiController
		RM -> RM : register_pcie_device(...)
	else PCIe virtio_net
		RM -> RM : resolve optional network resource id
		RM -> RM : create VirtioNetController(resolved?)
		RM -> RM : register_pcie_device(...)
	else SATA
		RM -> SataB : build(sata.type, storage_resources)
		SataB --> RM : Arc<SataDeviceApi>
		RM -> RM : register_sata_device(...)
	else IDE
		RM -> IdeB : build(ide.type, storage_resources)
		IdeB --> RM : Arc<IdeDeviceApi>
		RM -> RM : register_ide_device(...)
	else SCSI
		RM -> ScsiB : build(scsi.type, storage_resources)
		ScsiB --> RM : Arc<ScsiDeviceApi>
		RM -> RM : register_scsi_device(...)
	else USB
		RM -> UsbB : build(usb.type, usb_resources)
		UsbB --> RM : Arc<UsbDeviceApi>
		RM -> RM : register_usb_device(...)
	else PCI
		RM -> RM : register_pci_device(...)
	end
end

RM --> Caller : Ok(RuntimeModel)
@enduml
```

### Sequence diagram: missing resource failure path

Typical failure: a storage-backed device references a non-existent storage resource id.

```plantuml
@startuml
actor Caller
participant "RuntimeModel::try_from" as RM
participant "IdeDeviceBuilder" as IdeB

Caller -> RM : try_from(RuntimeConfig)
RM -> RM : Build storage resource map
RM -> IdeB : build(ide.type{resource:"diskX"}, storage_resources)
IdeB -> IdeB : lookup storage_resources["diskX"]
IdeB --> RM : Err("missing storage resource 'diskX' ...")
RM --> Caller : Err(String)
@enduml
```

### Current implementation notes

- Lifecycle actions (`start`, `stop`, `reset`, `shutdown`) are scaffolded and currently log intent.
- `gpu`, `display`, `audio`, and `guest_agent` are represented as optional top-level runtime features.
- Runtime model is qemu-agnostic. QEMU command rendering is owned by `src/config_format/qemu_cmd/runtime_render.rs`.
- Validation of duplicate/missing resource references lives in runtime config validation, while runtime model conversion performs final lookup enforcement.
