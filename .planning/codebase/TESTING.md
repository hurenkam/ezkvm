# Testing Patterns

**Analysis Date:** 2026-07-22

## Test Framework

**Runner:**
- Rust built-in test framework via `cargo test`
- Config: Cargo.toml specifies edition 2024 with standard test configuration
- Version: Rust edition 2024

**Assertion Library:**
- Standard Rust assertions: `assert!()`, `assert_eq!()`, `assert_ne!()`
- No external assertion library (no `pretty_assertions` or `claim` crates)

**Run Commands:**
```bash
cargo test              # Run all tests
cargo test -- --nocapture  # Run with output visibility
cargo test -- --test-threads=1  # Run serially for deterministic output
```

## Test File Organization

**Location:**
- Tests are **co-located** with source code in the same file
- Test modules at end of implementation files using `#[cfg(test)]` gate
- Example: `src/serde_yaml.rs` (line 53), `src/config/ezkvm/runtime/builder.rs` (end of file)

**Naming:**
- Test function prefix: `test_` or `round_trip_`
- Organized by functionality: `round_trip_q35_pvscsi_scsi_ssd`, `round_trip_i440fx_memory`
- Helper functions lack `test_` prefix: `assert_runtime_has_memory_and_q35_counts()`

**Structure:**
```
Tests appear in these files (only found via #[cfg(test)] blocks):
- src/serde_yaml.rs (5 tests)
- src/config/ezkvm/runtime/builder.rs (3 tests)
- src/config/ezkvm/runtime/parser.rs (4 tests)
```

## Test Structure

**Module Organization:**
```rust
// From src/serde_yaml.rs line 53
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_bool() {
        let result: bool = from_str("true").unwrap();
        assert!(result);
    }
}
```

**Patterns:**
- **Setup pattern:** Direct construction via builder or struct initialization; no fixtures
  ```rust
  let runtime = RuntimeBuilder::new()
      .with_memory(Memory::new(1024))
      .with_chipset(Chipset::Q35(...))
      .build()
      .expect("runtime build failed");
  ```

- **Teardown pattern:** None; all test objects are stack-allocated and auto-dropped

- **Assertion pattern:** Direct equality and boolean checks
  ```rust
  assert_eq!(memory_size, Some(expected_memory));
  assert_eq!(q35_counts, Some((expected_pcie, expected_scsi, expected_sata)));
  assert!(result.is_ok());
  ```

## Round-Trip Testing

**Integration Test Pattern:**
This codebase extensively uses "round-trip" testing to verify bidirectional conversions:

```
Runtime ──try_from──→ ConfigSchema ──try_from──→ Runtime
```

**Example from `builder.rs` lines 358-376:**
```rust
#[test]
fn round_trip_q35_pvscsi_scsi_ssd() {
    let runtime = RuntimeBuilder::new()
        .with_memory(Memory::new(1024))
        .with_chipset(Chipset::Q35(
            Q35ChipsetBuilder::new()
                .with_pcie_device(Some(PcieAddress::new(0, 0)), Arc::new(...))
                .build()
        ))
        .build()
        .expect("runtime build failed");
    
    let schema = crate::config::ezkvm::ConfigSchema::try_from(runtime)
        .expect("runtime -> schema conversion failed");
    let round_tripped = Runtime::try_from(schema)
        .expect("schema -> runtime conversion failed");
    
    // Assertions verify equivalence
}
```

**Coverage of Scenarios:**
- `round_trip_i440fx_memory`: Legacy chipset with memory
- `round_trip_q35_pvscsi_scsi_ssd`: Modern chipset with SCSI disk
- `round_trip_q35_sata_ssd`: SATA device configuration
- `round_trip_q35_extended_devices`: Multiple device types (PCIe, USB, IDE, SATA, SCSI)

## Test Assertions

**Custom Assertion Helpers:**
```rust
// From parser.rs lines 411-439
fn assert_runtime_has_memory_and_q35_counts(
    runtime: &Runtime,
    expected_memory: usize,
    expected_pcie: usize,
    expected_scsi: usize,
    expected_sata: usize,
) {
    let mut memory_size: Option<usize> = None;
    let mut q35_counts: Option<(usize, usize, usize)> = None;
    
    for root in runtime.root_devices() {
        if let Some(memory) = root.as_any().downcast_ref::<Memory>() {
            memory_size = Some(*memory.size());
        }
        if let Some(Chipset::Q35(q35)) = root.as_any().downcast_ref::<Chipset>() {
            // Extract device counts...
        }
    }
    
    assert_eq!(memory_size, Some(expected_memory));
    assert_eq!(q35_counts, Some((expected_pcie, expected_scsi, expected_sata)));
}
```

**Assertion Count by File:**
- `builder.rs`: ~20+ assertions across test blocks
- `parser.rs`: Multiple assertions verifying schema field values
- `serde_yaml.rs`: ~5 basic assertions on serialization correctness

## Test Coverage Areas

**YAML Serialization (serde_yaml.rs tests):**
- `test_deserialize_bool`: Boolean YAML parsing
- `test_deserialize_string`: String YAML parsing
- `test_deserialize_int`: Integer YAML parsing
- `test_serialize_bool`: Boolean YAML emission
- `test_serialize_string`: String YAML emission

**Runtime/Schema Conversions (parser.rs and builder.rs):**
1. **Builder Construction Tests:**
   - Q35 PCIe with PvSCSI and SCSI disk construction
   - Memory and chipset assembly via builder pattern
   - Error handling on build failures

2. **Round-Trip Tests:**
   - Runtime → ConfigSchema → Runtime equivalence
   - Device count preservation through conversion
   - Memory size and chipset type validation

3. **Device Parsing Tests:**
   - IDE, SATA, SCSI, PCIe, USB, PCI device schema parsing
   - Address and resource allocation validation
   - Multiple device type handling in single configuration

## Mocking

**Framework:** None detected

**Patterns:**
- No mocking library (no `mockito`, `mockall`, or similar)
- Uses **real object construction** for all tests
- Trait objects used directly without mocking: `Arc<dyn RootDevice>`
- All test dependencies are production types

**What to Mock:**
- N/A - testing philosophy is "test real implementations end-to-end"
- Tests construct actual Runtime, Memory, Chipset objects

**What NOT to Mock:**
- YAML parsing: Uses real `serde_yaml` module implementation
- Device hierarchy: Uses real device trait objects
- Builder logic: Full builder pattern verified, not stubbed

## Fixtures and Factories

**Test Data:**
- Inline construction pattern (no data files or factories):
  ```rust
  RuntimeBuilder::new()
      .with_memory(Memory::new(1024))
      .with_chipset(Chipset::Q35(...))
      .build()
  ```

- For YAML tests, inline strings:
  ```rust
  let result: bool = from_str("true").unwrap();
  let result: String = from_str("hello").unwrap();
  ```

**Location:**
- Test data is defined inline within `#[test]` functions
- No separate fixture files or factory modules
- Hard-coded configuration constants for each test scenario

## Coverage

**Requirements:** No enforced coverage requirements detected

**View Coverage:**
```bash
# Rust does not have built-in coverage without llvm-cov or tarpaulin
cargo tarpaulin          # If tarpaulin installed
cargo llvm-cov           # If llvm-cov installed
# Or: cargo test --cov (future Rust feature)
```

**Estimated Coverage:**
- YAML module: ~80% (5 basic tests on serialization API)
- Runtime builder: ~70% (round-trip tests cover common paths)
- Parser: ~60% (device type matching tested, edge cases sparse)
- Large files (`de.rs` 760 lines, `ser.rs` 332 lines): Likely <50% coverage

## Test Types

**Unit Tests:**
- Scope: Individual functions and small module interactions
- Approach: Direct function call + assertion
- Example: `test_deserialize_bool()` tests `from_str()` on single type

**Integration Tests:**
- Scope: Cross-module conversions and multi-step workflows
- Approach: Build full Runtime → serialize to schema → deserialize → verify
- Example: `round_trip_q35_extended_devices()` tests entire config pipeline
- Files: Both `builder.rs` and `parser.rs` contain integration tests

**End-to-End Tests:**
- Scope: None explicit (main.rs is manual demo, not automated E2E)
- Approach: Would require external YAML files and file I/O verification

## Common Patterns

**Async Testing:**
- Not applicable - no async code detected in codebase
- Single-threaded, synchronous execution model

**Error Testing:**
- Indirect approach: `.expect()` used to panic on conversion failure
- `Result::is_ok()` assertions used to verify success
- Example from `parser.rs:500`:
  ```rust
  let result = crate::runtime::Runtime::try_from(schema);
  assert!(result.is_ok());
  ```

- Error cases NOT explicitly tested (no test for malformed YAML input)

**Serialization Testing:**
- Round-trip pattern: Struct → Serialized → Deserialized → Verify equivalence
- YAML-specific assertions check for expected output format
- Example from `serde_yaml.rs` line 84:
  ```rust
  let result = to_string(&"hello").unwrap();
  assert!(result.contains("hello"));
  ```

## Test Execution

**Isolation:**
- Tests are independent (no global state mutations detected)
- Each test constructs fresh objects
- No interdependencies between tests

**Determinism:**
- Deterministic: All tests are reproducible
- No random data or time-dependent operations
- Safe to run in parallel (default behavior)

## Testing Gaps

**Critical Untested Areas:**

1. **Error Paths:** No tests for:
   - Malformed YAML input (missing required fields, wrong types)
   - Invalid device configurations (conflicting bus IDs, reserved addresses)
   - Mutex poisoning in RuntimeBuilder

2. **Edge Cases:**
   - Empty device lists
   - Maximum device count per bus
   - Address conflicts or duplicates
   - Null/None handling in optional fields

3. **File I/O:** No tests for:
   - ConfigFileStore (declared but no usage tests)
   - Compact YAML format edge cases

4. **Deserialization Robustness:**
   - Large YAML documents
   - Special characters in resource IDs
   - Unicode handling

5. **Performance:**
   - No performance benchmarks
   - Large device tree parsing speed untested

---

*Testing analysis: 2026-07-22*
