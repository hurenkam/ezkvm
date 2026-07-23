# Codebase Structure

**Analysis Date:** 2026-07-22

## Directory Layout

```
ezkvm_v4/
├── src/                                  # All Rust source code
│   ├── main.rs                          # Entry point: demonstrates Runtime/Schema/YAML round-trip
│   ├── runtime.rs                       # Runtime trait/struct definitions and RuntimeBuilder
│   ├── config.rs                        # Config layer module re-exports
│   ├── serde_yaml.rs                    # Custom YAML serde adapter via saphyr
│   │
│   ├── runtime/                         # In-memory VM topology layer
│   │   ├── memory.rs                    # Memory root device
│   │   ├── chipset.rs                   # Chipset enum (Q35/I440FX)
│   │   ├── q35.rs                       # Q35Chipset + Q35ChipsetBuilder
│   │   ├── pcie.rs                      # PCIe bus address/device trait
│   │   ├── pci.rs                       # PCI bus address/device trait
│   │   ├── sata.rs                      # SATA bus address/device trait
│   │   ├── scsi.rs                      # SCSI bus address/device trait
│   │   ├── ide.rs                       # IDE bus address/device trait
│   │   ├── usb.rs                       # USB bus address/device trait
│   │   ├── isa.rs                       # ISA device trait (minimal)
│   │   ├── storage.rs                   # Ssd, Hdd, Cdrom impl; StorageDevice trait
│   │   ├── devices.rs                   # Device enum re-exports
│   │   └── devices/                     # Specific device implementations
│   │       ├── pvscsi.rs                # PvScsi SCSI controller + builder
│   │       ├── pcie_net.rs              # VirtioNetPcie network device
│   │       ├── pci_generic.rs           # Generic PCI device
│   │       └── usb_generic.rs           # Generic USB device
│   │
│   ├── config/                          # Config container layer (re-exports)
│   │   ├── ezkvm.rs                     # ezkvm sub-module re-exports
│   │   ├── qemu.rs                      # QEMU backend (stub)
│   │   ├── proxmox.rs                   # Proxmox backend (stub)
│   │   │
│   │   └── ezkvm/                       # Main ezkvm config implementation
│   │       ├── ezkvm.rs                 # ConfigSchema re-exports
│   │       ├── runtime.rs               # EzkvmConfigSchema runtime handlers
│   │       ├── file.rs                  # ConfigFileStore trait re-exports
│   │       │
│   │       ├── schema/                  # YAML-serializable VM config structs
│   │       │   ├── config.rs            # ConfigSchema (top-level container)
│   │       │   ├── meta.rs              # Metadata (schema_version, vm_name)
│   │       │   ├── virtual_machine.rs   # VirtualMachineSchema (guest config)
│   │       │   ├── host.rs              # HostSchema (host resources)
│   │       │   ├── machine.rs           # MachineSchema (chipset/version)
│   │       │   ├── cpu.rs               # CpuSchema (cores, threads, sockets, model)
│   │       │   ├── memory.rs            # MemorySchema (size)
│   │       │   ├── boot.rs              # BootSchema (UEFI, BIOS, boot order)
│   │       │   ├── device.rs            # DeviceSchema (union of all device types)
│   │       │   ├── pcie.rs              # PCIe device schema
│   │       │   ├── pci.rs               # PCI device schema
│   │       │   ├── sata.rs              # SATA device schema
│   │       │   ├── scsi.rs              # SCSI device schema
│   │       │   ├── ide.rs               # IDE device schema
│   │       │   ├── usb.rs               # USB device schema
│   │       │   ├── tpm.rs               # TPM/swtpm schema
│   │       │   ├── guest_agent.rs       # Guest agent schema
│   │       │   ├── resources.rs         # HostResource schema (storage, network)
│   │       │   ├── display.rs           # Display schema
│   │       │   ├── audio.rs             # Audio schema
│   │       │   └── chipset.rs           # ChipsetSchema (machine family, versions)
│   │       │
│   │       ├── runtime/                 # Conversion handlers (Runtime ↔ ConfigSchema)
│   │       │   ├── parser.rs            # FromStr impl for ConfigSchema (YAML → Schema)
│   │       │   └── builder.rs           # Display/to_styled_compact_yaml impls (Schema → YAML)
│   │       │
│   │       └── file/                    # File I/O and serialization
│   │           ├── parser.rs            # YAML string parsing
│   │           ├── builder.rs           # YAML emission and formatting
│   │           ├── compact_yaml.rs      # Custom YAML styling (ToStyledYaml trait)
│   │           └── store.rs             # ConfigFileStore trait (file abstraction)
│   │
│   └── serde_yaml/                      # Custom serde + saphyr YAML adapter
│       ├── de.rs                        # Deserializer impl (YAML AST → serde::Deserialize)
│       ├── ser.rs                       # Serializer impl (serde::Serialize → YAML AST)
│       └── error.rs                     # Error type
│
├── target/                              # Rust build output (gitignored)
├── doc/                                 # Documentation (not analyzed here)
├── diffs/                               # Diff files (not analyzed here)
├── input/                               # Sample input files (not analyzed here)
├── src.org/                             # Backup source (not analyzed here)
│
├── Cargo.toml                           # Rust package manifest
├── Cargo.lock                           # Dependency lockfile
├── wakiza.yaml                          # VM configuration example
├── AGENTS.md                            # GSD agents documentation
├── LICENSE                              # Project license
└── .github/, .vscode/, .planning/       # Configuration and project planning
```

## Directory Purposes

**`src/runtime/`:**
- **Purpose:** In-memory VM topology representation
- **Contains:** Traits (RootDevice, device bus traits), concrete types (Memory, Chipset, Q35, storage types), builders
- **Key files:** `memory.rs`, `q35.rs`, `storage.rs`, `devices/`
- **Dependencies:** Derives only (derive-getters, derive-new, ordered-float for addressing)
- **Used by:** main.rs for building runtime, config layer for conversion

**`src/config/ezkvm/schema/`:**
- **Purpose:** YAML-serializable configuration structures
- **Contains:** Serde structs representing each VM subsystem (CPU, memory, devices, resources)
- **Key files:** `config.rs` (ConfigSchema), `virtual_machine.rs`, `host.rs`, device schemas
- **Dependencies:** serde (derive), derive-getters, derive-new, ordered-float
- **Used by:** File I/O layer for parsing/serialization, Runtime conversion handlers

**`src/config/ezkvm/runtime/`:**
- **Purpose:** Convert between Runtime and ConfigSchema
- **Contains:** Handler traits and implementations (EzkvmMemoryHandler, EzkvmChipsetHandler)
- **Key files:** `builder.rs` (Schema → YAML), `parser.rs` (YAML → Schema)
- **Dependencies:** ConfigSchema, Runtime types
- **Used by:** main.rs for round-trip conversions

**`src/config/ezkvm/file/`:**
- **Purpose:** Parse YAML and serialize ConfigSchema
- **Contains:** FromStr impl, Display impl, compact YAML styler
- **Key files:** `parser.rs`, `builder.rs`, `compact_yaml.rs`
- **Dependencies:** serde_yaml, saphyr
- **Used by:** main.rs for input parsing and output formatting

**`src/serde_yaml/`:**
- **Purpose:** Custom serde adapter using saphyr YAML library
- **Contains:** Deserializer, Serializer, Error type
- **Key files:** `de.rs`, `ser.rs`, `error.rs`
- **Dependencies:** serde, saphyr
- **Used by:** File I/O layer and schema Layer for all YAML ↔ Rust conversions

## Key File Locations

**Entry Points:**
- `src/main.rs`: Application entry; demonstrates all major flows

**Configuration:**
- `Cargo.toml`: Package metadata and dependencies
- `wakiza.yaml`: Example VM configuration file (YAML format)

**Core Logic:**
- `src/runtime.rs`: Runtime trait/struct, RuntimeBuilder, root device registration
- `src/runtime/q35.rs`: Q35 chipset with device bus management
- `src/config/ezkvm/schema/config.rs`: ConfigSchema definition (top-level YAML struct)
- `src/config/ezkvm/file/parser.rs`: YAML → ConfigSchema deserialization
- `src/config/ezkvm/file/builder.rs`: ConfigSchema → YAML serialization
- `src/serde_yaml.rs`: Custom serde adapter (public API)

**Testing:**
- No dedicated test files found (--0 .test.rs files). Tests may be inline or in separate test directory not yet created.

## Naming Conventions

**Files:**
- `{component}.rs` — Single-purpose trait/struct file (e.g., `memory.rs`, `pcie.rs`)
- `mod.rs` — Not used; each file is module definition
- Module directories use snake_case (e.g., `src/config/ezkvm/`, `src/runtime/devices/`)

**Modules:**
- Trait files named after trait (e.g., `pcie.rs` contains PcieDevice trait + PcieAddress)
- Builder files suffixed with `builder` (e.g., `q35.rs` contains Q35Chipset + Q35ChipsetBuilder)
- Schema files named after struct (e.g., `virtual_machine.rs` contains VirtualMachineSchema)
- Re-export files named after component (e.g., `ezkvm.rs` re-exports from schema/, file/, runtime/ submodules)

**Types:**
- Traits: PascalCase, no Trait suffix (e.g., RootDevice, PcieDevice, StorageDevice)
- Concrete types: PascalCase (e.g., Memory, Q35Chipset, ConfigSchema, Ssd)
- Builders: Append "Builder" (e.g., RuntimeBuilder, Q35ChipsetBuilder, PvScsiBuilder)
- Address types: Append "Address" (e.g., PcieAddress, SataAddress, ScsiAddress)
- Schema types: Append "Schema" (e.g., ConfigSchema, VirtualMachineSchema, CpuSchema)
- Enums: PascalCase (e.g., Chipset, StorageDeviceType, DeviceSchema)

**Functions/Methods:**
- Builders: `with_{component}()` (e.g., with_memory, with_pcie_device)
- Getters: Auto-generated by derive-getters macro; no get_ prefix; pub fn {field_name}(&self) -> &{type}
- Constructors: `new()` for simple types; builder pattern for complex ones
- Conversions: Standard Rust traits (TryFrom, FromStr, Display, From)

**Variables:**
- snake_case for bindings (e.g., runtime, schema, builder)
- SCREAMING_SNAKE_CASE for constants (e.g., EZKVM_CONFIG_SCHEMA_VERSION)

## Where to Add New Code

**New Device Type (e.g., GPU, custom PCI device):**
- **Runtime implementation:** Create `src/runtime/devices/{device_name}.rs`
  - Define struct (e.g., `struct Gpu { ... }`)
  - Implement relevant bus trait(s) (PcieDevice, PciDevice, etc.)
  - If device has sub-devices, create a builder (e.g., GpuBuilder)
  - Export in `src/runtime/devices.rs` via `pub use`
- **Schema representation:** Create `src/config/ezkvm/schema/{device_name}.rs`
  - Define #[derive(Serde, Getters, new)] struct with config fields
  - Add variant to `DeviceSchema` enum in `src/config/ezkvm/schema/device.rs`
  - Export in `src/config/ezkvm/schema.rs`
- **Conversion handler:** Add to `src/config/ezkvm/runtime/builder.rs`
  - Implement handler to convert Runtime device → Schema device
  - Implement reverse in schema → runtime

**New Bus Type (e.g., NVMe namespace):**
- Create `src/runtime/{bus_name}.rs` with trait + address type
  - Define trait (e.g., `pub trait NvmeDevice: ...`)
  - Define address struct (e.g., `#[derive(...)] pub struct NvmeAddress { namespace: u32 }`)
- Add to Q35ChipsetBuilder in `src/runtime/q35.rs`
  - Add HashMap field to Q35ChipsetBuilder
  - Add `with_{bus_name}_device()` method
- Create schema in `src/config/ezkvm/schema/{bus_name}.rs`
- Update ConfigSchema handlers in `src/config/ezkvm/runtime/`

**New Schema Field (e.g., VNC display config):**
- Add field to schema struct in `src/config/ezkvm/schema/{component}.rs`
  - Use `#[serde(default, skip_serializing_if = "Option::is_none")]` for optional fields
- Update VirtualMachineSchema, HostSchema, or top-level ConfigSchema as appropriate
- Update corresponding runtime type if behavioral impact
- Update conversion handlers if round-trip conversion needed

**YAML Custom Serialization (e.g., compact format):**
- Add methods to `src/config/ezkvm/file/compact_yaml.rs`
  - Implement ToStyledYaml trait on ConfigSchema variants
  - Custom formatting in `emit_styled_yaml()`
- Call from `src/config/ezkvm/file/builder.rs` in `to_styled_compact_yaml()` method

**YAML Parsing Enhancement (e.g., macro expansion):**
- Modify `src/config/ezkvm/file/parser.rs` parsing logic
  - Pre-process input string before serde_yaml::from_str
  - Post-process deserialized ConfigSchema to fill in defaults

## Special Directories

**`target/`:**
- **Purpose:** Rust build artifacts (binaries, deps, etc.)
- **Generated:** Yes (by cargo build)
- **Committed:** No (in .gitignore)

**`doc/`:**
- **Purpose:** Documentation (not analyzed)
- **Generated:** Possibly (README, design docs)
- **Committed:** Yes

**`input/`:**
- **Purpose:** Sample YAML/configuration files for testing
- **Generated:** No (manually created)
- **Committed:** Yes

**`.planning/`:**
- **Purpose:** GSD project planning and analysis artifacts
- **Generated:** Yes (by GSD tools)
- **Committed:** Yes (planning history)

**`.github/`:**
- **Purpose:** GitHub-specific configs (Actions, issue templates, etc.)
- **Generated:** No
- **Committed:** Yes

## Module Dependency Graph

```
main.rs
  ├─ config
  │   └─ ezkvm
  │       ├─ schema
  │       │   ├─ config.rs (ConfigSchema)
  │       │   ├─ virtual_machine.rs
  │       │   ├─ host.rs
  │       │   └─ [other schema modules]
  │       ├─ file
  │       │   ├─ parser.rs
  │       │   ├─ builder.rs
  │       │   └─ compact_yaml.rs
  │       └─ runtime
  │           ├─ builder.rs
  │           └─ parser.rs
  │
  ├─ runtime
  │   ├─ memory.rs (Memory root device)
  │   ├─ chipset.rs (Chipset enum)
  │   ├─ q35.rs (Q35Chipset + Q35ChipsetBuilder)
  │   ├─ devices.rs
  │   │   └─ devices/
  │   │       ├─ pvscsi.rs (PvScsi)
  │   │       └─ [other device impls]
  │   ├─ storage.rs (Ssd, Hdd, Cdrom)
  │   └─ [bus trait files]
  │
  └─ serde_yaml
      ├─ de.rs
      ├─ ser.rs
      └─ error.rs
```

No circular dependencies detected. Layers are unidirectional:
- `main` uses `config` and `runtime`
- `config` uses `runtime` for conversions
- `runtime` has no upward dependencies

---

*Structure analysis: 2026-07-22*
