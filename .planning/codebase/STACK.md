# Technology Stack

**Analysis Date:** 2026-07-22

## Languages

**Primary:**
- Rust 1.94.0 - Full system implementation, VM runtime configuration and schema parsing

**Build Edition:**
- Rust Edition 2024 - Latest edition features and stability guarantees

## Runtime

**Environment:**
- Cargo 1.94.0 - Package manager and build system for Rust

**Package Manager:**
- Cargo (Rust native)
- Lockfile: `Cargo.lock` (present)

## Frameworks

**Serialization/Deserialization:**
- serde 1.0.228 - Framework for serializing and deserializing Rust data structures
  - Features: `derive` - Provides procedural macros for auto-implementing Serialize/Deserialize traits

**YAML Processing:**
- saphyr 0.0.11 - YAML parser and emitter with serde integration
  - Provides: `LoadableYamlNode`, `YamlEmitter`, `YamlOwned` types
  - Replaces standard `serde_yaml` to avoid JSON intermediate formats

**Custom Implementation:**
- serde_yaml replacement - Drop-in YAML serialization built with saphyr (see `src/serde_yaml/`)
  - Functions: `from_str()`, `to_string()`, `from_value()`, `to_value()`
  - Deserializer at `src/serde_yaml/de.rs` (760 LOC)
  - Serializer at `src/serde_yaml/ser.rs` (332 LOC)

## Key Dependencies

**Critical:**
- serde 1.0.228 - Core serialization framework with derive macros
  - Dependencies: serde_core, serde_derive
  - Enables: Automatic struct serialization/deserialization to YAML

- saphyr 0.0.11 - YAML parsing and emission
  - Dependencies: encoding_rs (UTF-8/encoding support), hashlink, ordered-float, saphyr-parser, thiserror
  - Enables: Type-safe YAML document handling without JSON intermediates

**Procedural Macros:**
- derive-getters 0.5.0 - Generates getter methods from struct fields
  - Dependencies: proc-macro2, quote, syn
  - Usage: Applied to schema types in `src/config/ezkvm/schema/`
  - Applied to: MemorySchema, ChipsetSchema, CpuSchema, etc.

- derive-new 0.7.0 - Generates `new()` constructor methods
  - Dependencies: proc-macro2, quote, syn
  - Usage: Applied to configuration schema types

**Data Structures:**
- hashlink 0.12.1 - HashMap with insertion order preservation
  - Dependencies: hashbrown (0.17.1)
  - Usage: Ordered collection handling for configuration mappings

- ordered-float 5.3.0 - Totally ordered floating-point types
  - Dependencies: num-traits
  - Usage: Floating-point calculations with strict ordering

**Error Handling:**
- thiserror 2.0.18 - Procedural macro for error type definitions
  - Dependencies: thiserror-impl
  - Transitive Dependencies: proc-macro2, quote, syn, unicode-ident

**Support Libraries:**
- encoding_rs 0.8.35 - Character encoding support
- num-traits 0.2.19 - Generic numeric traits
- proc-macro2 1.0.106 - Macros 2.0 support
- quote 1.0.46 - Macro quoting utilities
- syn 2.0.119 - Rust syntax parsing
- unicode-ident 1.0.16 - Unicode identifier definitions

## Configuration

**Environment:**
- No environment variables required for core functionality
- Configuration provided via YAML files (see `wakiza.yaml` example in codebase)

**Build:**
- `Cargo.toml` - Project manifest defining dependencies and metadata
- Edition: 2024
- Package name: ezkvm
- Package version: 0.1.0

## Platform Requirements

**Development:**
- Rust toolchain 1.94.0+
- Cargo 1.94.0+
- Standard C/C++ build environment (for proc-macro compilation)

**Production:**
- Linux, macOS, or Windows with Rust runtime
- No external runtime dependencies
- Binary deployable as standalone executable

---

*Stack analysis: 2026-07-22*
