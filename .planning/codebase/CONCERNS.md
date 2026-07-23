# Codebase Concerns

**Analysis Date:** 2026-07-22

## Tech Debt

**Complex YAML Deserialization Logic:**
- Issue: Custom YAML deserializer implementation spans 760 lines with extensive pattern matching logic scattered across `src/serde_yaml/de.rs`. No comprehensive test coverage for edge cases (complex nested structures, special YAML features like tags and aliases).
- Files: `src/serde_yaml/de.rs` (760 lines), `src/serde_yaml/ser.rs` (332 lines)
- Impact: Deserialization failures on edge-case YAML inputs could crash the application or silently misparse configuration. Maintenance is difficult due to code complexity.
- Fix approach: Either use a mature YAML library (like `serde_yaml` crate) or add comprehensive property-based testing with `proptest` covering alias resolution, tagged types, all scalar types, and nested structures.

**Type-Unsafe Runtime Downcasting Pattern:**
- Issue: Codebase heavily relies on `as_any()` + `downcast_ref::<T>()` pattern (54 instances) for runtime type checking. No type registry or compile-time guarantees for device handling.
- Files: `src/runtime.rs`, `src/config/ezkvm/runtime/parser.rs` (lines 34, 39, 98-120, 137-170, 189-191, 236-239), `src/config/ezkvm/runtime/builder.rs` (multiple)
- Impact: Runtime crashes if wrong device type is used. Adding new device types requires changes in 10+ places. Refactoring is risky.
- Fix approach: Replace downcasting with sealed trait pattern + trait objects with explicit methods per device type, or use an enum-based registry pattern.

**Extensive Commented-Out Code:**
- Issue: `src/config/ezkvm.rs` contains ~110 lines of commented-out code (lines 8-110) defining obsolete handler pattern.
- Files: `src/config/ezkvm.rs` (8-110)
- Impact: Increases cognitive load, unclear if code is deprecated or planned. Violates "do one thing" principle.
- Fix approach: Delete immediately or move to a `.archived` branch. Document decision in ARCHITECTURE.md.

**Hardcoded Default Addresses:**
- Issue: Device address defaults are spread across builder methods. `src/config/ezkvm/runtime/builder.rs` uses `unwrap_or` with hardcoded defaults on lines 134, 198-199, 211, 219, 235, 243, 261, 272, 285, 290, 299, 326.
- Files: `src/config/ezkvm/runtime/builder.rs` (130+ lines of defaults)
- Impact: If defaults conflict with user configuration, errors are silent. No validation that addresses don't collide.
- Fix approach: Create `AddressAllocator` trait/struct to manage address space with explicit conflict detection before building.

**Serialization with expect() in Hot Path:**
- Issue: `src/config/ezkvm/file/compact_yaml.rs:51` calls `.expect("failed to serialize...")` in `from_serde()`. This is called during YAML rendering.
- Files: `src/config/ezkvm/file/compact_yaml.rs:51`
- Impact: Serde errors (which are possible with malformed data) will panic instead of returning errors.
- Fix approach: Return `Result<YamlOwned, Error>` instead of panicking.

---

## Known Bugs

**Mutex Poisoning in RuntimeBuilder:**
- Symptoms: Panic if any thread panics while holding lock during `.build()` or device addition
- Files: `src/runtime.rs:98, 103, 108`
- Trigger: Run builder with panicking thread; call `.lock().unwrap()` on poisoned Mutex
- Workaround: Restructure to not require Mutex in builder (builder should be single-threaded during construction)
- Specific issue lines:
  - Line 98: `.expect("Root devices mutex should not be poisoned")`
  - Line 103-104: `.lock().unwrap().push(...)`
  - Line 108-109: `.lock().unwrap().push(...)`

**YAML Alias Resolution Not Implemented:**
- Symptoms: YAML files with aliases (e.g., `&anchor` and `*anchor`) will silently fail deserialization
- Files: `src/serde_yaml/de.rs:53` returns `Err(Error::Message("unresolved alias".to_string()))`
- Trigger: Use any YAML config with aliases
- Workaround: Expand all aliases manually in YAML before parsing
- Impact: Users cannot use YAML aliasing features; reduces reusability of config templates

**stripPrefix/unwrap_or Pattern Hides Errors:**
- Symptoms: YAML emitter output stripping prefix silently continues if prefix missing
- Files: `src/config/ezkvm/file/compact_yaml.rs:84-86`
- Trigger: Unknown (may depend on saphyr version)
- Workaround: None; check saphyr version compatibility
- Risk: Output format may differ from expected without warning

---

## Security Considerations

**Unsafe Memory Transmute in EnumAccess:**
- Risk: Critical - Lifetime transmutation of leaked Box string
- Files: `src/serde_yaml/de.rs:719-721`
- Code: 
  ```rust
  let variant_str: &'static str = Box::leak(variant_name.clone());
  let variant_str: &'de str =
      unsafe { std::mem::transmute::<&'static str, &'de str>(variant_str) };
  ```
- Current mitigation: Comments claim "immediately re-borrowed with shorter lifetime" but this is incorrect reasoning. The transmute converts `'static` to `'de`, which could be longer. If deserializer outlives the string or the string is reused elsewhere, this is a use-after-free.
- Recommendations: 
  1. **CRITICAL**: Do NOT use Box::leak in deserialization
  2. Use `Box::into_raw()` and reconstruct via `Box::from_raw()` with proper ownership tracking
  3. Better: Use `Cow<str>` or owned String references instead of transmute
  4. Add MIRI test (`cargo +nightly miri test`) to catch undefined behavior

**No Input Validation on YAML Size:**
- Risk: Denial of service - malicious YAML with deeply nested structures could consume unbounded memory
- Files: `src/serde_yaml/de.rs` (LoadableYamlNode::load_from_str has no depth limit)
- Current mitigation: None
- Recommendations:
  1. Add max-depth parameter to YAML parser
  2. Limit document size to reasonable maximum (e.g., 10MB)
  3. Add timeout to YAML parsing

**No Validation of Resource Paths:**
- Risk: Path traversal - if device resources can be arbitrary paths, user input isn't validated
- Files: `src/config/ezkvm/schema/` (ResourceSchema definitions)
- Current mitigation: None visible
- Recommendations:
  1. Validate resource paths are within allowed directories
  2. Use `Path::canonicalize()` and check against whitelist
  3. Add tests for path traversal attempts

---

## Performance Bottlenecks

**Repeated Type Downcasting in Parsing:**
- Problem: Parser iterates over devices multiple times, re-downcasting the same device to check type (see parser.rs lines 98, 110, 114, 118 for single device)
- Files: `src/config/ezkvm/runtime/parser.rs:92-240`
- Cause: Pattern matching with sequential if-let chains instead of match/pattern exhaustion
- Improvement path:
  1. Use `as_any().type_id()` once and match on TypeId
  2. Cache type information in device struct
  3. Refactor to use enum wrapper instead of trait object downcasting

**Box::leak in Deserialization Loop:**
- Problem: Each variant deserialization leaks memory until end of deserialization
- Files: `src/serde_yaml/de.rs:719`
- Cause: `Box::leak()` intentionally prevents deallocation
- Current scope: Only called during enum variant deserialization, but in large YAML files with many enums, this could add up
- Improvement path: Implement proper Cow<str> or BorrowedStrDeserializer with actual lifetime management

**No Streaming YAML Parsing:**
- Problem: Entire YAML document loaded into memory as AST before parsing
- Files: `src/serde_yaml/de.rs` (via saphyr::LoadableYamlNode::load_from_str)
- Cause: Using saphyr's Document-based API rather than streaming
- Improvement path: For large config files, consider event-based YAML parsing

---

## Fragile Areas

**YAML Serialization/Deserialization Round-trip:**
- Files: `src/config/ezkvm/runtime/parser.rs:280-409` (tests only), `src/serde_yaml/` (entire module)
- Why fragile: Tests only cover happy paths (Q35 with standard devices, I440FX, specific device combinations). No tests for:
  - Minimum/maximum values for numeric fields
  - Unicode and special characters in strings
  - Missing optional fields
  - Fields with default values being omitted
  - Circular references via aliases
- Safe modification: 
  1. Add property-based tests with `proptest` for all schema types
  2. Add fuzz testing with `cargo-fuzz`
  3. Add tests for schema validation errors
- Test coverage gaps:
  - Display/Audio schemas never tested
  - NetworkResourceSchema never tested
  - Boot configuration never tested
  - CPU configuration never tested

**Device Address Collision Detection:**
- Files: `src/config/ezkvm/runtime/builder.rs` (lines 62-330), `src/runtime/q35.rs`
- Why fragile: Builder silently overwrites addresses if collisions occur. No validation that:
  - Bus numbers are within valid range
  - Addresses don't conflict with existing devices
  - Device slot restrictions are respected (e.g., certain PCI slots reserved)
- Safe modification:
  1. Maintain address allocation set during build
  2. Return error if address already occupied
  3. Add tests for address conflict detection
- Test coverage gaps:
  - No tests for overlapping address assignments
  - No tests for invalid bus numbers

**Enum Variant Type Checking in Parser:**
- Files: `src/config/ezkvm/runtime/parser.rs:110-120, 160-170`
- Why fragile: Sequential if-let chain with multiple `downcast_ref::<T>()` calls on same device. If new storage type added, must update all these chains.
- Example (lines 110-120): Checking Hdd, then Ssd, then Cdrom - any new storage type requires changes here
- Safe modification:
  1. Add `fn device_type(&self) -> StorageDeviceType` method to StorageDevice trait
  2. Match on returned enum instead of downcasting
  3. Compiler will catch missing variants

---

## Scaling Limits

**Mutex in Builder:**
- Current capacity: Single builder can hold arbitrary number of devices (bounded by available memory)
- Limit: Mutex serializes all builder operations; concurrent builds require separate builders
- Scaling path: Remove Mutex (builders are typically single-threaded), or use lock-free structures for read-heavy workloads

**YAML Parser Memory:**
- Current capacity: Entire document in memory
- Limit: Documents > available RAM will OOM
- Scaling path: Implement streaming YAML parser or chunk-based loading for multi-gigabyte configs

---

## Dependencies at Risk

**saphyr v0.0.11 - Pre-1.0 Dependency:**
- Risk: Pre-release YAML library with potential breaking changes
- Impact: Version bumps could break deserialization without warning; no stability guarantees
- Migration plan: 
  1. Monitor saphyr releases and pin to exact version
  2. Prepare fallback to `serde_yaml` or `yaml-rust2`
  3. Add integration tests that verify round-trip compatibility across saphyr versions

**derive-getters v0.5.0:**
- Risk: Generates getter methods; future versions could change naming convention
- Impact: Generated code changes would require refactoring all call sites
- Migration plan: Review changelog before any version updates; test thoroughly after upgrades

**derive-new v0.7.0:**
- Risk: Generates `new()` constructors; compile-time code generation could break
- Impact: Field order changes in structs could silently generate different constructors
- Migration plan: Keep constructor generation manual for critical types; add tests to verify struct field order

---

## Missing Critical Features

**Configuration Validation:**
- Problem: No schema validation step before building runtime. Invalid configs silently fail during type conversions.
- Blocks: Users cannot get early feedback on configuration errors; error messages are cryptic
- Example: Missing required fields return generic "conversion failed" errors instead of specific validation errors
- Impact: Poor UX for config file debugging

**Error Context and Backtraces:**
- Problem: Most errors return flat `String` errors; no context about which resource or device failed
- Blocks: Debugging large configs is difficult; cannot determine which line in YAML failed
- Example: Parser returns "unsupported root device in runtime parser: ???" without naming the device
- Impact: Users cannot efficiently fix configuration issues

**Logging/Observability:**
- Problem: No logs, warnings, or traces; only panics and silent failures
- Blocks: Debugging config issues, performance profiling, security auditing
- Missing: 
  - Debug logs for device enumeration
  - Warnings for deprecated configs
  - Traces for long operations
  - Error recovery attempts

---

## Test Coverage Gaps

**YAML Deserialization Edge Cases:**
- What's not tested: 
  - Alias resolution (currently returns error, never tested)
  - Empty documents
  - Documents with only comments
  - Very deeply nested structures (DOS risk)
  - Invalid UTF-8 sequences
  - Very large numeric values (overflow risk)
  - Null values in unexpected positions
- Files: `src/serde_yaml.rs` (53-86: only 5 basic tests)
- Risk: Crashes or silent data corruption on edge-case inputs
- Priority: **HIGH** - custom YAML parser needs comprehensive test coverage

**Config Schema Validation:**
- What's not tested:
  - Invalid CPU counts (0, 1, or > 1024)
  - Memory sizes outside reasonable ranges (0, > 1TB)
  - Invalid boot device references (pointing to non-existent resources)
  - Device type mismatches (e.g., CDROM on SCSI expecting disk)
  - Bus number collisions
  - Missing required resources
- Files: `src/config/ezkvm/schema/` (comprehensive schemas but no validation tests)
- Risk: Silent runtime errors during VM configuration
- Priority: **MEDIUM** - add validation tests before first release

**RuntimeBuilder Thread Safety:**
- What's not tested:
  - Concurrent builder operations (should fail or be serialized)
  - Builder panic recovery
  - Mutex poisoning scenarios
  - Builder drop during in-progress build
- Files: `src/runtime.rs:83-111` (no thread safety tests)
- Risk: Crashes under concurrent access
- Priority: **HIGH** - if builder is exposed in API

**Round-trip Completeness:**
- What's not tested:
  - All device types in all combinations (currently only ~4 combinations tested)
  - Optional fields being properly omitted/included
  - Display devices (schema exists, never tested)
  - Audio devices (schema exists, never tested)
  - Guest agent configuration (schema exists, never tested)
  - TPM configuration (schema exists, never tested)
- Files: `src/config/ezkvm/runtime/parser.rs:280-409` (4 tests, many schema types untested)
- Risk: Unknown - could silently drop device configuration on round-trip
- Priority: **MEDIUM** - should test before 1.0 release

---

## Architecture Issues

**No Error Type Hierarchy:**
- Problem: Errors are all flat `String` or one-off enum with no context
- Files: `src/serde_yaml/error.rs`, throughout config module
- Impact: Cannot programmatically distinguish error types; error handling is brittle
- Fix: Create comprehensive error enum with Display/From traits and error codes

**Unsafe Lifetime Transmutation Anti-Pattern:**
- Problem: Using unsafe transmute to extend lifetimes violates Rust safety guarantees
- Files: `src/serde_yaml/de.rs:719-721`
- Impact: Undefined behavior; cannot be audited or safely modified
- Fix: Use proper lifetime parameters or owned String

---

*Concerns audit: 2026-07-22*
