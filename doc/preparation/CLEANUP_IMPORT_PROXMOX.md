# Code Review: import/proxmox Module Against Guidelines

**Date:** 2026-04-16  
**Scope:** Architecture, Coding, and Module Ownership compliance review of `src/import/proxmox/`  
**References:**
- `doc/dev/ARCHITECTURE_GUIDELINES.md`
- `doc/dev/CODING_GUIDELINES.md`
- `doc/dev/MODULE_OWNERSHIP.md`

---

## Overview

The import/proxmox module is generally well-structured with correct dependency direction and error handling. However, it has **significant file size and cohesion issues** that violate coding guidelines and harm maintainability.

| Category | Status | Severity |
|----------|--------|----------|
| Dependency direction | ✅ Compliant | — |
| Error handling | ✅ Compliant | — |
| Module wiring | ✅ Compliant | — |
| Test placement | ✅ Compliant | — |
| **File size** | ❌ Violated | **Critical** |
| **Function cohesion** | ⚠️ Borderline | High |
| **Public surface area** | ⚠️ Over-exposed | Medium |
| Struct sizes | ✅ Compliant | — |

---

## Compliant Areas ✅

### 1. Dependency Direction (MODULE_OWNERSHIP.md)

- ✅ Only depends on `crate::config` — correct direction (`import -> config`)
- ✅ No upward dependencies to `cli` or `runtime`
- ✅ No cross-layer shortcuts
- ✅ Matches allowed edge: `import -> config` per ownership matrix

### 2. Error Handling (CODING_GUIDELINES.md §4)

- ✅ Production code uses `?` operator and `map_err()` appropriately
- ✅ No unwrap/expect in production paths (all confined to test code)
- ✅ Errors include actionable context with file paths and I/O details
- ✅ Example from `io.rs`:
  ```rust
  let input = std::fs::read_to_string(input_path).map_err(|e| {
      ImportError::ParseError(format!("unable to read input file '{}': {}", input_path, e))
  })?;
  ```

### 3. Module Wiring (CODING_GUIDELINES.md §13 & ARCHITECTURE_GUIDELINES.md §1)

- ✅ `src/import/proxmox/mod.rs` is clean wiring-only file (13 lines)
- ✅ Re-exports are explicit and minimal
- ✅ No struct/fn/impl bodies mixed into mod.rs

### 4. Test Placement (ARCHITECTURE_GUIDELINES.md §1)

- ✅ Unit tests located in `mod tests` within source files (mapper.rs, parser.rs)
- ✅ Tests use appropriate layout adjacent to tested code
- ✅ Test-only expect/unwrap calls are acceptable

### 5. Struct Sizes (CODING_GUIDELINES.md §2)

- ✅ All structs in model.rs are small (< 35 lines):
  - `ProxmoxVmConfig`: 8 fields
  - `ProxmoxStorageEntry`: 3 fields
  - `ProxmoxDiskEntry`: 5 fields
  - All intermediate model types appropriately sized

---

## Non-Compliant Areas ⚠️

### 1. File Size Violations (CODING_GUIDELINES.md §2 & ARCHITECTURE_GUIDELINES.md §1)

**Guideline Target:** < 250 lines per file

| File | Lines | Guideline | Variance | Status |
|------|-------|-----------|----------|--------|
| `mapper.rs` | **2082** | < 250 | **+1832 lines** (+732%) | ❌ **CRITICAL** |
| `parser.rs` | 372 | < 250 | +122 lines (+49%) | ❌ Over |
| `io.rs` | 269 | < 250 | +19 lines (+8%) | ⚠️ Borderline |
| `yaml_compact.rs` | 162 | < 250 | — | ✅ OK |
| `storage_parser.rs` | 191 | < 250 | — | ✅ OK |
| `error.rs` | 17 | < 250 | — | ✅ OK |
| `model.rs` | 56 | < 250 | — | ✅ OK |

**mapper.rs Violation Detail:**
- 2082 lines in a single file is 8.3× over guideline
- Guideline states: "If not possible, document in comments the reason" — no justification comment present
- File contains ~40 functions with diverse responsibilities (see next section)

### 2. Function Proliferation and Unclear Cohesion

**mapper.rs** contains 40 functions spanning disparate responsibilities:

**Top-Level Entry Points (2):**
- `map_proxmox_to_canonical_yaml`
- `map_proxmox_to_canonical_yaml_with_storage`

**High-Level Mappers (8 domains, ~15 functions):**
- CPU/memory/boot/SMBIOS system mapping
- Storage/drive/SCSI controller mapping
- Network device/backend/queue mapping
- Display/VGA/audio/SPICE mapping
- Device mapping (USB, PCI, IOMMU, TPM)

**Utility Parsers (15+ functions):**
- `parse_machine_and_options()`, `parse_cpu_model_and_features()`
- `parse_boot_order()`, `parse_smbios_uuid()`
- `parse_source_and_options()`, `parse_options()` (variants)
- `parse_human_size_to_bytes()`, `shell_split()`
- `parse_prefixed_options()`

**Transformation Helpers (10+ functions):**
- `normalize_host_pci_device()`, `increment_function_address()`
- `resolve_volume_reference()`, `resolve_dir_volume()`, `resolve_zfspool_volume()`
- `is_enabled()`, `is_q35_machine()`, `is_ms_cert_enabled()`, `is_vdagent_spicevmc()`

**Issue:** Functions lack clear module grouping, making it difficult to navigate and maintain.

### 3. Excessive Public Module Exports (CODING_GUIDELINES.md §3 & ARCHITECTURE_GUIDELINES.md §1)

**Current `src/import/proxmox/mod.rs`:**
```rust
pub mod error;         // ✅ Needed for re-exports
pub mod io;           // ⚠️ Should be private
pub mod mapper;       // ⚠️ Should be private
pub mod model;        // ✅ Needed externally (types)
pub mod parser;       // ⚠️ Should be private
pub mod storage_parser;  // ⚠️ Should be private

pub use error::ImportError;
pub use io::{ImportRunOptions, run_import_from_files};
pub use mapper::map_proxmox_to_canonical_yaml;  // ⚠️ Bypass io interface
pub use mapper::map_proxmox_to_canonical_yaml_with_storage;  // ⚠️ Bypass io interface
pub use parser::parse_proxmox_config;  // ⚠️ Bypass io interface
pub use storage_parser::parse_proxmox_storage_config;  // ⚠️ Bypass io interface
```

**Problem:** Users can call parser/mapper functions directly, bypassing the high-level `run_import_from_files()` contract, creating tight coupling to implementation details.

**Acceptable Usage (should be):**
```rust
let result = run_import_from_files("vm.conf", &options)?;
```

**Current Leaked Usage (should not be allowed):**
```rust
let parsed = parse_proxmox_config("...")?;
let mapped = map_proxmox_to_canonical_yaml(&parsed)?;
```

---

## Root Cause Analysis

**Why mapper.rs is 2082 lines:**

The mapper was designed as a single, monolithic function composition that internally coordinates:
1. Parsing intermediate representations (already parsed by `parser.rs`)
2. Mapping each Proxmox construct to ezkvm schema equivalents
3. Handling storage resolution, path transformation, and device composition
4. Emitting canonical YAML with validation

**Original Design Intent:**
- All mapping logic in one place for auditability and consistency
- One-shot transformation with single entry point

**Unintended Consequence:**
- Difficult to navigate, test, and extend
- Supporting functions are discovery-by-search rather than by module structure
- Violates guideline that files > 250 lines must have documented justification

---

## Recommended Mitigations

### Priority 1: Refactor mapper.rs (High Impact, High Value)

**Goal:** Break 2082-line monolith into focused, navigable modules.

**Proposed Structure:**
```
src/import/proxmox/
├── mapper.rs           # Entry points + top-level orchestration (~150 lines)
├── mapper/
│   ├── system.rs       # CPU, memory, boot, TPM, SMBIOS (~250 lines)
│   ├── storage.rs      # Drives, SCSI, storage resolution (~250 lines)
│   ├── network.rs      # Network devices, backends (~200 lines)
│   ├── display.rs      # Display, audio, SPICE (~200 lines)
│   ├── device.rs       # USB, PCI, IOMMU (~150 lines)
│   └── helpers.rs      # Parsing utilities, volume resolution (~300 lines)
```

**Result per Module:**
- Each file targets 150–300 lines
- Clear responsibility separation
- Easier to test and debug
- Simpler to extend (e.g., adding B-17/B-18/B-19/B-20 features)

**Example Refactor - mapper.rs entry point:**
```rust
// src/import/proxmox/mapper.rs (~150 lines)
use super::{error::ImportError, model::*, mapper::*};
use crate::config::VmConfig;

pub struct MappingWarning {
    pub source_field: String,
    pub message: String,
}

pub fn map_proxmox_to_canonical_yaml(
    proxmox: &ProxmoxVmConfig,
) -> Result<VmConfig, ImportError> {
    map_proxmox_to_canonical_yaml_with_storage(proxmox, None)
}

pub fn map_proxmox_to_canonical_yaml_with_storage(
    proxmox: &ProxmoxVmConfig,
    storage_config: Option<&ProxmoxStorageConfig>,
) -> Result<VmConfig, ImportError> {
    let mut warnings = Vec::new();

    let name = proxmox.scalars.get("name").cloned()
        .unwrap_or_else(|| "imported-vm".to_string());
    let architecture = system::map_architecture(proxmox.scalars.get("arch"), &mut warnings);
    let system = system::map_system(proxmox, &mut warnings)?;
    let devices = device::map_devices(proxmox, storage_config, &mut warnings)?;
    let host = device::map_host(proxmox, &mut warnings)?;
    
    // ... assemble VmConfig
}
```

### Priority 2: Restrict Public Exports (Medium Impact)

**Change `src/import/proxmox/mod.rs`:**

```rust
mod model;  // Was: pub mod model
mod error;  // Was: pub mod error
mod io;     // Already private ✅
mod parser; // Make private (was: pub mod parser)
mod mapper; // Make private (was: pub mod mapper)
mod storage_parser;  // Make private (was: pub mod storage_parser)
mod yaml_compact;    // Already private ✅

// Re-export only high-level entry points
pub use error::ImportError;
pub use io::{ImportRunOptions, run_import_from_files};

// Remove direct exports:
// ❌ pub use mapper::map_proxmox_to_canonical_yaml;
// ❌ pub use parser::parse_proxmox_config;
// ❌ pub use storage_parser::parse_proxmox_storage_config;
```

**Enforcement:** This makes the API contract explicit: users call `run_import_from_files()` only. Internal functions are implementation details.

### Priority 3: Document Remaining Over-Size Files (Low Impact)

**For parser.rs (372 lines, +49% over guideline):**

Add documentation comment at file top:
```rust
//! Proxmox configuration file parser
//! 
//! This module parses Proxmox VM config files into an intermediate typed model.
//! File size exceeds guidelines (372 > 250 lines) due to:
//! - 9 parsing functions for diverse device types (disk, network, USB, PCI, TPM)
//! - Regex and string tokenization rules for each device bus type
//! - Refactoring into separate device parsers is deferred to future cleanup
//! (tracks improvement in future backlog items)
```

**For io.rs (269 lines, +8% over guideline):**

Add comment if needed; currently borderline and refactoring mapper.rs may naturally reduce this below threshold.

---

## Acceptance Criteria for Mitigation

### Refactor mapper.rs ✅
- [ ] All new module files < 300 lines (target < 250)
- [ ] `mapper.rs` entry points are < 150 lines
- [ ] Each submodule (`system.rs`, `storage.rs`, etc.) has single, clear responsibility
- [ ] All public functions at module level clearly named (e.g., `system::map_system()`)
- [ ] Tests remain co-located with implementation (in `#[cfg(test)] mod tests {}`)
- [ ] No behavioral changes; output YAML identical before/after
- [ ] All backlog feature mappings (B-17, B-18, B-19, B-20) can be implemented in clear, focused locations

### Restrict Public Exports ✅
- [ ] Only `ImportRunOptions`, `run_import_from_files()`, and `ImportError` are public
- [ ] Internal types (ProxmoxVmConfig, parsers, mappers) are private
- [ ] No direct import path usage in main CLI or tests; all go through `run_import_from_files()`
- [ ] Integration tests still pass (they use high-level API)

### Documentation ✅
- [ ] Comments justify file size overages where they remain
- [ ] ADR or backlog entry records cleanup intention

---

## Implementation Roadmap

| Phase | Task | Backlog Item | Effort |
|-------|------|--------------|--------|
| **Immediate** | Add justification comments to oversize files | Tech Debt | 0.5 days |
| **Immediate** | Restrict public exports (Priority 2) | B-02 refinement | 1 day |
| **Phase 1** | Split mapper.rs into focused submodules (Priority 1) | B-21 (new) | 3–4 days |
| **Phase 1+** | Implement B-17/B-18/B-19/B-20 in refactored structure | B-17 through B-20 | Existing |

---

## Risk Assessment

### Risk: Behavioral Changes During Refactor

**Mitigation:**
- Comprehensive snapshot tests exist for fixture configs
- Verify generated YAML byte-for-byte matches before/after refactor
- Run full integration test suite

### Risk: API Breaking Change

**Mitigation:**
- Restrict public exports only removes implementation leakage, not high-level contract
- `run_import_from_files()` remains stable (primary entry)
- Feasible deprecation path if external callers use direct parser/mapper (unlikely, pre-release)

### Risk: Over-Fragmentation

**Mitigation:**
- Consolidate similar domains (e.g., all device mapping in `device.rs`)
- Don't exceed 7 submodules; goal is clarity, not micro-modules

---

## Conclusion

The import/proxmox module has good fundamentals (correct dependencies, error handling, test placement) but needs structural refactoring to comply with file size and cohesion guidelines. Priority 1 (mapper.rs refactor) and Priority 2 (restrict public exports) are recommended before Phase 2 hardening and eventual public release. This work aligns with backlog items B-17 through B-20 and improves the codebase's maintainability and extensibility.
