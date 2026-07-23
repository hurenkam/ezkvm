# Coding Conventions

**Analysis Date:** 2026-07-22

## Naming Patterns

**Files:**
- Lowercase with underscores: `memory.rs`, `storage.rs`, `compact_yaml.rs`
- Module types use descriptive names: `builder.rs`, `parser.rs`, `schema.rs`, `runtime.rs`
- Files ending in `_schema.rs` contain serialization-friendly data structures
- Test modules inline within files using `#[cfg(test)]` rather than separate files

**Functions:**
- Lowercase with underscores: `parse_chipset()`, `build_runtime()`, `match_pcie_device()`
- Builder methods use `with_*` prefix: `with_memory()`, `with_chipset()`, `with_sata_device()`
- Conversion/parsing methods use `try_from()`, `parse()`, or prefix naming
- Helper functions use descriptive names: `assert_runtime_has_memory_and_q35_counts()`, `format_root_device()`

**Variables:**
- Lowercase with underscores: `root_devices`, `pcie_bus`, `memory_size`, `expected_pcie`
- Builder result variables: `runtime`, `schema`, `chipset`
- Collection variables use plural: `root_devices`, `devices`, `handlers`, `pcie_devices`

**Types:**
- PascalCase for structs: `Runtime`, `Memory`, `Q35Chipset`, `MemorySchema`, `DeviceSchema`
- PascalCase for enums: `Chipset`, `StorageDeviceType`, `DeviceSchema`
- Type aliases for function pointers: `QemuSchemaHandler`, `ProxmoxSchemaHandler`
- Suffixes for schema types: `*Schema` for serializable variants

## Code Style

**Formatting:**
- No explicit formatter configured in Cargo.toml
- Standard Rust formatting conventions appear to be followed
- 4-space indentation implicit in codebase

**Linting:**
- No clippy configuration files found (`.clippy.toml`)
- Standard Rust warnings and lints are in effect
- Dead code frequently suppressed with `#[allow(dead_code)]` for unused constructors and methods

## Import Organization

**Order:**
1. Standard library imports (`use std::...`)
2. Third-party crate imports (`use serde::...`, `use saphyr::...`)
3. Derive macros (`use derive_getters::Getters`, `use derive_new::new`)
4. Local crate imports (`use crate::...`)

**Path Aliases:**
- Imports use absolute paths from crate root: `use crate::config::ezkvm::schema::...`
- Relative imports used within modules: `use super::...`
- Module re-exports use `pub use` pattern in `mod.rs` or lib entry files:
  ```rust
  pub use {schema::ConfigSchema, file::ConfigFileStore};
  ```

**Patterns:**
- Schema files group related imports at top: device schemas, host schemas, metadata
- Runtime files group runtime types together
- Grouped imports for better readability over wildcard imports

## Error Handling

**Patterns:**
- `Result<T, String>` is the primary error type throughout the codebase
- Error messages are descriptive strings: `"runtime missing memory root device"`
- `?` operator used frequently for error propagation
- `unwrap()` used only in safe contexts:
  - Main function initialization: `.expect("build runtime failed")`
  - Test code: `.unwrap()` on test YAML parsing
  - Mutex locks: `self.root_devices.lock().unwrap()`
- `ok_or_else()` used for converting `Option` to `Result` with lazy evaluation
- `map_err()` used for error type conversion in parsing

**Example Error Handling:**
```rust
// From parser.rs - line 349
impl TryFrom<ConfigSchema> for Runtime {
    type Error = String;
    fn try_from(value: ConfigSchema) -> Result<Self, Self::Error> {
        // Implementation returns Result<Self, String>
    }
}

// From builder.rs - line 36
pub fn build(&self) -> Result<Runtime, String> {
    let chipset = self.build_chipset()?;  // Early return on error
    // ...
}
```

## Logging

**Framework:** `println!()` and `eprintln!()` only

**Patterns:**
- Debug output in main: `println!("runtime: {}\n\n\n", runtime);`
- Error context in handler fallbacks: `println!("No {} handler for device type: {:?}", handler_name, device_type);`
- No structured logging or log crate integration
- Used primarily for diagnostic output, not production logging

**Locations:**
- `src/main.rs`: Runtime and schema debug output
- `src/config/ezkvm.rs`, `qemu.rs`, `proxmox.rs`: Handler missing messages

## Comments

**When to Comment:**
- Module-level documentation using `//!` format in `serde_yaml.rs`
- Function documentation using `///` for public APIs
- Inline comments for non-obvious logic very sparse; code is self-documenting
- Commented-out code blocks preserved in files (e.g., `src/config/ezkvm.rs` lines 8-109)

**Doc Comments:**
```rust
/// Deserialize a value from a YAML string.
pub fn from_str<T: for<'de> serde::Deserialize<'de>>(s: &str) -> Result<T, Error> {
    // ...
}

/// Serialize a value to a YAML string.
pub fn to_string<T: serde::Serialize>(value: &T) -> Result<String, Error> {
    // ...
}
```

**Module Documentation:**
```rust
//! Serde-compatible YAML deserializer and serializer using saphyr.
//!
//! This module provides a drop-in replacement for `serde_yaml` functions, using saphyr's
//! YAML parser and emitter.
```

## Function Design

**Size:** Functions range from 5 to 30+ lines; larger functions decomposed into helpers
- Short accessors: 1-3 lines (generated via `#[derive(Getters)]`)
- Medium parsers: 15-25 lines (e.g., `build_q35_chipset()`, `parse_chipset()`)
- Complex logic delegated to separate functions (e.g., `insert_pvsci_controllers()`)

**Parameters:**
- Self/self for methods with state
- `&self` for queries, `&mut self` for mutations
- Pass owned values for device registration: `Arc<dyn RootDevice>`
- Multiple parameters grouped logically: address, device, storage type
- Mutable collections passed by reference for in-place operations

**Return Values:**
- Builder methods return `&mut self` for chaining
- Conversions return `Result<T, String>`
- Construction methods return `Self` or `Self::Builder`
- Traits implement standard patterns (Display, Debug, From, TryFrom)

**Example Function:**
```rust
pub fn build(self) -> Result<Runtime, ()> {
    let root_devices = self
        .root_devices
        .into_inner()
        .expect("Root devices mutex should not be poisoned");
    Ok(Runtime { root_devices })
}
```

## Module Design

**Exports:**
- `pub use` statements in module headers re-export important types
- Each module typically has a `mod.rs` or inline module declarations
- Hierarchy: `config/` → `ezkvm/` → `schema/`, `runtime/`, `file/`

**Barrel Files:**
- `src/config/ezkvm/schema.rs` (line 23): `pub use {schema::ConfigSchema, file::ConfigFileStore};`
- `src/runtime.rs` (lines 14-24): Re-exports core runtime types via `pub use`
- `src/config/ezkvm.rs` (line 6): Re-exports ConfigSchema and ConfigFileStore

**Module Structure:**
```
src/
├── main.rs              # Entry point, demonstrates builder and conversion patterns
├── config.rs            # Config module root (proxmox, qemu, ezkvm variants)
│   ├── ezkvm.rs        # EZKVM schema root
│   │   ├── schema/     # Data structures for serialization
│   │   ├── runtime/    # Builder and parser implementations
│   │   └── file/       # YAML formatting and storage
│   ├── qemu.rs         # QEMU schema stub
│   └── proxmox.rs      # Proxmox schema stub
├── runtime.rs          # Runtime model root
│   ├── memory.rs       # Memory configuration
│   ├── storage.rs      # Storage devices
│   ├── chipset.rs      # Chipset selection
│   └── ... (other device types)
└── serde_yaml.rs       # Custom YAML serialization
```

**Re-export Pattern:**
- Submodules declare internal types with `pub` visibility
- Parent module selectively re-exports via `pub use` in root file
- Allows clean public API without exposing implementation details

## Derive Macro Usage

**Standard Derives:**
- `Debug`: Nearly universal for types (aids in error messages and logging)
- `Clone`: Used on data structures that need to be copied
- `Serialize, Deserialize`: Applied to schema types via serde
- `Getters`: Custom derive from `derive-getters` crate to generate field accessors
- `new`: Custom derive from `derive-new` crate to generate constructors
- `Default`: Implemented manually or derived on schema types

**Example:**
```rust
#[derive(Debug, Clone, Deserialize, Serialize, Getters, new)]
pub struct Q35ChipsetSchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}
```

**Serde Attributes:**
- `#[serde(untagged)]`: For enum variants without wrapper (DeviceSchema)
- `#[serde(tag = "type", rename_all = "snake_case")]`: Tagged enums
- `#[serde(flatten)]`: Inline struct fields in parent
- `#[serde(default)]`: Use Default impl if field missing
- `#[serde(skip_serializing_if)]`: Omit from serialized output if predicate true

## Trait Implementations

**Common Patterns:**
- `impl Display for AddressSchema`: Manual string formatting for network addresses
- `impl Default for SchemaType`: Default values for optional configs
- `impl TryFrom<SourceType> for TargetType`: Fallible type conversions
- `impl Display for Runtime`: Human-readable output format

**Trait Objects:**
- `Arc<dyn RootDevice>`: Dynamic dispatch for polymorphic device storage
- `Arc<dyn PcieDevice>`: Enables heterogeneous device collections
- Extensive use of `dyn Trait` for flexible architecture

## Builder Pattern

**Implementation:**
- `RuntimeBuilder` collects configuration via mutable methods
- Methods like `with_memory()`, `with_chipset()` return `&mut self`
- `.build()` terminal method returns `Result<Runtime, ErrorType>`
- Uses `Mutex<Vec<T>>` to safely accumulate state

**Usage:**
```rust
let runtime = RuntimeBuilder::new()
    .with_memory(Memory::new(1024))
    .with_chipset(Chipset::Q35(Q35ChipsetBuilder::new()...))
    .build()
    .expect("build runtime failed");
```

## Anti-Patterns

### Widespread Use of `#[allow(dead_code)]`

**What happens:** Nearly every module has multiple `#[allow(dead_code)]` attributes, indicating unused code paths remain in production.

**Why it's wrong:** Maintains unused API surface that may confuse consumers and increase maintenance burden. Makes it unclear what's actually needed vs. experimental.

**Do this instead:** Audit modules marked with `#[allow(dead_code)]` in `src/config/qemu.rs`, `src/config/proxmox.rs`, and remove unused variants. Use feature gates for experimental implementations instead.

### Comments-Out Large Code Blocks

**What happens:** `src/config/ezkvm.rs` (lines 8-109) contains an entire commented-out SchemaBuilder implementation.

**Why it's wrong:** Dead code takes up space and can confuse developers. Version control should handle history.

**Do this instead:** Remove the commented block and rely on git history. If the pattern is worth keeping, extract into a feature-gated module or document the reason for deferral in a CONCERN.

### Manual `Option` to `Result` Conversions

**What happens:** Frequent use of `ok_or_else()` and `ok_or()` to convert Option types, especially in parsing.

**Why it's wrong:** Verbose and repetitive. Wastes tokens on error construction.

**Do this instead:** Define helper trait or use `anyhow`/`eyre` for more ergonomic error handling. Example from `parser.rs:50`:
```rust
// Current:
let memory = memory.ok_or_else(|| "runtime missing memory root device".to_string())?;

// Better:
let memory = memory.context("runtime missing memory root device")?;
```
