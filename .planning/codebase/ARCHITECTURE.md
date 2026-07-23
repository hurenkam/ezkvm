<!-- refreshed: 2026-07-22 -->
# Architecture

**Analysis Date:** 2026-07-22

## System Overview

EzKVM is a Rust-based VM configuration framework that translates between multiple abstraction layers:
1. **Runtime Layer** — Direct Rust objects representing VM hardware components
2. **Schema Layer** — YAML-serializable configuration structures
3. **File I/O Layer** — Parser, builder, and YAML emission

The system operates bidirectionally: you can build a Runtime programmatically and export it to ConfigSchema/YAML, or parse YAML input to create a Runtime.

```text
┌────────────────────────────────────────────────────────────┐
│              Entry Point / Main Application                │
│                  `src/main.rs`                             │
│  Demonstrates Runtime → Schema → YAML round-trip           │
└──────────┬─────────────────────────┬───────────────────────┘
           │                         │
           ▼                         ▼
┌──────────────────────────┐  ┌──────────────────────────┐
│    Runtime Layer         │  │    Config Layer          │
│  `src/runtime/`          │  │  `src/config/ezkvm/`     │
│  - Memory                │  │  - ConfigSchema          │
│  - Chipset (Q35/I440FX)  │  │  - Device mappings       │
│  - Devices (PCI/PCIe)    │  │  - Resources/Host info   │
│  - Storage (SSD/HDD)     │  │                          │
└──────────┬───────────────┘  └───────────┬──────────────┘
           │                              │
           └────────────┬─────────────────┘
                        ▼
            ┌────────────────────────────┐
            │   Schema Layer             │
            │ `src/config/ezkvm/schema/` │
            │  - ConfigSchema            │
            │  - VirtualMachineSchema    │
            │  - HostSchema              │
            │  - Metadata                │
            └────────────┬───────────────┘
                         │
                 ▼       ▼      ▼
              ┌─────┐ ┌────┐ ┌──────┐
              │YAML │ │File│ │Build │
              │Parse│ │I/O │ │ Emit │
              └─────┘ └────┘ └──────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| **Runtime** | In-memory VM topology: root devices (Memory, Chipset) | `src/runtime.rs` |
| **Chipset** | Container for bus hierarchies (PCI, PCIe, SATA, IDE, USB) | `src/runtime/chipset.rs` |
| **Q35** | Q35 chipset implementation with builder pattern for device placement | `src/runtime/q35.rs` |
| **Devices** | PCI/PCIe/USB device implementations (PvScsi, VirtioNet, generic) | `src/runtime/devices/` |
| **Storage** | Ssd, Hdd, Cdrom implementations; implements multiple bus traits | `src/runtime/storage.rs` |
| **ConfigSchema** | Top-level YAML-serializable container | `src/config/ezkvm/schema/config.rs` |
| **VirtualMachineSchema** | Guest VM specification (CPU, memory, boot, devices, TPM) | `src/config/ezkvm/schema/virtual_machine.rs` |
| **HostSchema** | Host-side resources (storage, network) | `src/config/ezkvm/schema/host.rs` |
| **File Parser** | YAML → ConfigSchema deserialization | `src/config/ezkvm/file/parser.rs` |
| **File Builder** | ConfigSchema → YAML serialization with styling | `src/config/ezkvm/file/builder.rs` |
| **Serde YAML** | Custom serde adapter using saphyr YAML library | `src/serde_yaml/` |

## Pattern Overview

**Overall:** Builder + Type-driven conversion pattern

**Key Characteristics:**
- **Trait-based polymorphism:** Root devices, bus devices, storage devices all inherit traits (`RootDevice`, `PcieDevice`, `StorageDevice`)
- **Builder pattern:** `RuntimeBuilder`, `Q35ChipsetBuilder`, `PvScsiBuilder` for incremental construction
- **Layered conversion:** Runtime ↔ ConfigSchema ↔ YAML via `TryFrom` impls and `FromStr`
- **Arc-wrapped devices:** Devices wrapped in `Arc<dyn Trait>` for shared ownership across buses
- **Schema-first serialization:** ConfigSchema is the YAML source of truth; Round-trip fidelity maintained via serde attrs

## Layers

**Runtime Layer:**
- **Purpose:** In-memory representation of VM topology; builder-friendly API
- **Location:** `src/runtime/`
- **Contains:** Traits (RootDevice, PcieDevice, PciDevice, SataDevice, etc.), concrete types (Memory, Chipset, Q35Chipset, storage types), and builders
- **Depends on:** Standard library, derive macros (derive-getters, derive-new)
- **Used by:** main.rs entry point, config layer via conversions
- **Key types:**
  - `Runtime` — contains Vec<Arc<dyn RootDevice>> (Memory, Chipset)
  - `Chipset::Q35(Q35Chipset)` — contains nested HashMaps for each bus type
  - Storage types (Ssd, Hdd, Cdrom) — implement multiple bus device traits

**Config/Schema Layer:**
- **Purpose:** YAML-serializable representation of VM config; source of truth for I/O
- **Location:** `src/config/ezkvm/schema/`
- **Contains:** Serde structs (ConfigSchema, VirtualMachineSchema, HostSchema, DeviceSchema, etc.)
- **Depends on:** serde, derive-getters, derive-new
- **Used by:** File parser/builder, Runtime conversions
- **Key types:**
  - `ConfigSchema` — top-level struct with metadata, host, virtual_machine
  - `VirtualMachineSchema` — guest config (machine, CPU, memory, boot, devices, TPM)
  - `HostSchema` — host resources (storage, network, display, audio)

**File I/O Layer:**
- **Purpose:** Parse YAML files into ConfigSchema; serialize ConfigSchema to YAML
- **Location:** `src/config/ezkvm/file/`
- **Contains:** Parser (FromStr impl), builder (Display/to_styled_compact_yaml impl), store (ConfigFileStore trait)
- **Depends on:** serde_yaml module, ConfigSchema
- **Used by:** main.rs for demo input parsing/output formatting

**Serde YAML Layer:**
- **Purpose:** Custom serde adapter replacing standard serde_yaml using saphyr library
- **Location:** `src/serde_yaml/`
- **Contains:** Deserializer, Serializer, Error types
- **Depends on:** saphyr (YAML AST parser/emitter), serde
- **Used by:** File I/O layer for parsing/emitting

## Data Flow

### Primary Request Path: Runtime → Schema → YAML

1. **Build Runtime programmatically** (`src/main.rs:15-31`)
   ```
   RuntimeBuilder::new()
     .with_memory(Memory::new(1024))
     .with_chipset(Q35ChipsetBuilder::new()
       .with_sata_device(...)
       .with_pcie_device(...)
       .build())
     .build()
   ```
   - Result: `Runtime` with root devices in Vec

2. **Convert Runtime → ConfigSchema** (`src/main.rs:35`)
   ```
   EzkvmConfigSchema::try_from(runtime)
   ```
   - Iterates root_devices
   - Downcasts to concrete types (Memory, Chipset)
   - Populates ConfigSchema fields
   - Uses handlers (EzkvmMemoryHandler, EzkvmChipsetHandler)

3. **Serialize ConfigSchema → YAML** (`src/main.rs:110-111`)
   ```
   schema.to_styled_compact_yaml()
   ```
   - ConfigSchema serializes via serde → saphyr Yaml AST
   - Custom compact_yaml formatter optimizes presentation
   - YamlEmitter emits to string

### Secondary Path: YAML → Schema → Runtime

1. **Parse YAML string** (`src/main.rs:107`)
   ```
   EzkvmConfigSchema::from_str(input)
   ```
   - Parses via serde_yaml::from_str
   - saphyr loads YAML → YamlOwned AST
   - Deserializer walks AST, populates ConfigSchema

2. **Convert ConfigSchema → Runtime** (`src/main.rs:38`)
   ```
   Runtime::try_from(schema)
   ```
   - Reads virtual_machine, host, metadata from schema
   - Reconstructs Runtime's root devices
   - Reverse of Runtime → Schema conversion

### State Management

- **No persistent state:** Each operation is stateless conversion
- **Builder accumulation:** RuntimeBuilder and Q35ChipsetBuilder accumulate state in Mutex/Vec, then finalize via `.build()`
- **Arc sharing:** Devices shared across multiple bus references via Arc<dyn Trait>
- **No circular refs:** Devices don't reference parents; containment is one-way (Chipset → buses → devices)

## Key Abstractions

**RootDevice Trait:**
- **Purpose:** Polymorphic container for Memory and Chipset
- **Examples:** `src/runtime/memory.rs`, `src/runtime/chipset.rs`
- **Pattern:** Trait object downcasting via `as_any()` method + TypeId

**Bus Device Traits (PcieDevice, PciDevice, SataDevice, IdeDevice, UsbDevice):**
- **Purpose:** Polymorphic containers for devices attachable to specific buses
- **Examples:** `src/runtime/pcie.rs`, `src/runtime/sata.rs`, `src/runtime/scsi.rs`
- **Pattern:** Each bus trait has minimal interface; devices impl multiple traits based on multi-bus compatibility

**StorageDevice Trait:**
- **Purpose:** Characterize storage options (HDD vs SSD vs CDROM)
- **Examples:** `src/runtime/storage.rs`
- **Pattern:** `storage_options()` returns StorageOptions with device_type, read_only, cache_mode

**Builder Pattern:**
- **Purpose:** Incremental construction with optional address specification
- **Examples:** RuntimeBuilder, Q35ChipsetBuilder, PvScsiBuilder
- **Pattern:** `with_X_device(Option<Address>, Arc<Device>)` → Self, `.build()` finalizes

**Address Types:**
- **Purpose:** Uniquely identify device placement on each bus
- **Examples:** PcieAddress(device, function), SataAddress(bus, port), ScsiAddress(target, lun)
- **Pattern:** Small struct with Copy semantics, Display for rendering

## Entry Points

**`src/main.rs` (Application Entry):**
- **Location:** Line 14
- **Triggers:** Executed when binary runs
- **Responsibilities:**
  1. Demonstrates Runtime builder API
  2. Shows Runtime → Schema → YAML round-trip
  3. Parses YAML string input to Schema/Runtime
  4. Pretty-prints output at each step

**`Runtime::try_from<ConfigSchema>` (Schema → Runtime):**
- **Location:** Impl blocks in schema conversion code (not shown in snippets, but referenced in main)
- **Triggers:** Explicit conversion call
- **Responsibilities:** Reconstruct Runtime from ConfigSchema by reading metadata, virtual_machine, host sections

**`EzkvmConfigSchema::try_from<Runtime>` (Runtime → Schema):**
- **Location:** Handlers in `src/config/ezkvm/runtime/` (referenced in ezkvm.rs)
- **Triggers:** Explicit conversion call
- **Responsibilities:** Downcast root devices, use handlers to populate Schema fields

**`FromStr for ConfigSchema` (YAML → Schema):**
- **Location:** `src/config/ezkvm/file/parser.rs`
- **Triggers:** `ConfigSchema::from_str(yaml_string)`
- **Responsibilities:** Delegate to serde_yaml::from_str

**`Display for ConfigSchema` (Schema → YAML):**
- **Location:** `src/config/ezkvm/file/builder.rs`
- **Triggers:** `format!("{}", schema)` or `.to_styled_compact_yaml()`
- **Responsibilities:** Serialize via serde_yaml, optionally apply compact styling

## Architectural Constraints

- **Trait object sizes:** Devices wrapped in Arc<dyn Trait> because size unknown at compile time; enables runtime polymorphism
- **Downcast pattern:** RootDevice and bus devices use `.as_any()` + TypeId for type checking; no true downcasting; must manually check each concrete type
- **No nested builders:** Builders flatten the hierarchy (e.g., Q35ChipsetBuilder creates device maps, but doesn't nest further)
- **Single generic YAML path:** All YAML parsing goes through serde_yaml/saphyr; no format variants (e.g., no JSON support in schema layer)
- **Optional device addresses:** Builder pattern defaults to (0, 0) if address not provided; no address validation or conflict detection
- **Copy-on-serialize:** Storage devices (Ssd, Hdd, Cdrom) are zero-sized types; serialization is metadata-only

## Error Handling

**Strategy:** Early termination with Result propagation

**Patterns:**
- Builder `.build()` returns `Result<T, ()>` — Mutex poisoning treated as fatal
- `try_from` impls return `Result` — Downcasting failures or missing schema fields are fatal
- `from_str` returns `Result<T, String>` — YAML parse errors propagated with message
- `to_styled_compact_yaml` returns `Result<String, String>` — Serialization errors caught
- No panics in normal paths (except in runtime builders' `.expect()` calls in main.rs for demo purposes)

## Cross-Cutting Concerns

**Logging:** 
- None. System is functional/pure; main.rs uses println! for demo output

**Validation:**
- Minimal. No address space conflict detection, bus capacity checks, or device compatibility validation
- Schema parsing relies on serde type validation only

**Serialization Round-Trip:**
- ConfigSchema is designed for lossless round-trip (YAML → Schema → YAML yields identical output)
- Custom compact_yaml formatter preserves structure while optimizing presentation

## Type Hierarchy

```
RootDevice (Trait)
├─ Memory
├─ Chipset
   └─ Chipset::Q35(Q35Chipset)

PcieDevice (Trait)
├─ PvScsi
├─ VirtioNetPcie
├─ GenericPciDevice

PciDevice (Trait)
├─ PvScsi
├─ GenericPciDevice

SataDevice (Trait)
├─ Ssd
├─ Hdd
├─ Cdrom

ScsiDevice (Trait)
├─ Ssd
├─ Hdd
├─ Cdrom

IdeDevice (Trait)
├─ Ssd
├─ Hdd
├─ Cdrom

UsbDevice (Trait)
├─ GenericUsbDevice

StorageDevice (Trait)
├─ Ssd
├─ Hdd
├─ Cdrom
```

---

*Architecture analysis: 2026-07-22*
