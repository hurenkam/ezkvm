# External Integrations

**Analysis Date:** 2026-07-22

## APIs & External Services

**Not detected:** This project does not integrate with external APIs or third-party services. It operates as a standalone utility for VM configuration management.

## Data Storage

**Databases:**
- Not used - No database integration

**File Storage:**
- Local filesystem only - YAML configuration file I/O
  - Client: Standard Rust `std::fs` module
  - Implementation: `src/config/ezkvm/file/store.rs` (ConfigFileStore)
  - Operations: Read/write YAML configuration files to disk
  - File format: `.yaml` extension (e.g., `{vm_name}.yaml`)

**Caching:**
- Not used - No caching layer

## Authentication & Identity

**Auth Provider:**
- None - This is a local configuration tool with no authentication requirements

## Monitoring & Observability

**Error Tracking:**
- Not integrated - Application exits with Result types

**Logs:**
- No logging framework integrated
- Output via standard Rust `println!()` macros in main (`src/main.rs`)
- Example output: Runtime configuration, schema parsing results

**Metrics:**
- Not collected

## CI/CD & Deployment

**Hosting:**
- Not applicable - Standalone CLI utility

**CI Pipeline:**
- Not detected - No CI configuration in repository

**Build Process:**
- Cargo build system manages all compilation and dependency resolution
- See `Cargo.toml` and `Cargo.lock` for complete dependency specifications

## Environment Configuration

**Required env vars:**
- None - Project requires no environment variables

**Config Files:**
- YAML configuration files loaded from local filesystem
- Example: `wakiza.yaml` - Sample VM configuration with host resources, machine specs, and device definitions
- Schema: Defined in `src/config/ezkvm/schema/` modules
- Parsing: Uses custom serde_yaml implementation via saphyr

**Secrets location:**
- Not applicable - No secret management
- Configuration is stored in plaintext YAML files

## File I/O Operations

**Configuration Loading:**
- Input: YAML files from filesystem
- Method: `ConfigFileStore::load_config()` at `src/config/ezkvm/file/store.rs`
- Error handling: Returns `std::io::Error` with InvalidData variant for parse failures
- Usage: `std::fs::read_to_string()` followed by schema parsing

**Configuration Saving:**
- Output: YAML files to filesystem
- Method: `ConfigFileStore::save_config()` at `src/config/ezkvm/file/store.rs`
- Format: Styled compact YAML via `config.to_styled_compact_yaml()`
- Error handling: Catches serialization errors and maps to InvalidData I/O errors

**Working with Content:**
- Uses custom serde_yaml implementation (`src/serde_yaml/`)
- `from_str()` - Deserialize YAML string to ConfigSchema
- `to_string()` / `to_styled_compact_yaml()` - Serialize ConfigSchema to YAML string

## Webhooks & Callbacks

**Incoming:**
- None - This is not an HTTP service

**Outgoing:**
- None - No external callbacks or notifications

## Configuration Schema

**VM Configuration Structure:**
- Metadata: Schema version, VM name
- Host resources: Storage (block devices) and network (bridges)
- Virtual machine: CPU, memory, boot configuration, devices (PCIE, SATA, SCSI, USB)
- Chipset support: Q35 (with version specification)

**Example Configuration Source:** `wakiza.yaml` embedded in test code at `src/main.rs` lines 41-105

## Data Format Standards

**Format:** YAML 1.1 compatible (via saphyr parser)

**Serialization:**
- Serde framework for Rust data → YAML conversion
- Derives implemented via `#[derive(Serialize, Deserialize)]` macros
- Encoding: UTF-8 (handled by encoding_rs transitive dependency)

**Structured Types:**
- All configuration structs defined in `src/config/ezkvm/schema/` modules
- Auto-derived getters via `#[derive(Getters)]` from derive-getters
- Auto-derived constructors via `#[derive(new)]` from derive-new
- Supports nested structures for hierarchical VM configuration

---

*Integration audit: 2026-07-22*
