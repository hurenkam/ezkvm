## Findings

### 1. High: Section 13 `mod.rs` cleanliness is violated in QEMU module
- Guideline: Section 13 says `mod.rs` files should contain no `struct`, `fn`, or `impl` sections.
- Evidence: [src/qemu/mod.rs](src/qemu/mod.rs#L16), [src/qemu/mod.rs](src/qemu/mod.rs#L21), [src/qemu/mod.rs](src/qemu/mod.rs#L68), [src/qemu/mod.rs](src/qemu/mod.rs#L327).
- Impact: The module entrypoint is carrying substantial implementation logic, making navigation and ownership boundaries inconsistent with the project guideline.

### 2. High: Section 13 `impl From<A> for B` co-location rule is violated for config schema types
- Guideline: Keep `impl From<A> for B` in the same file as `struct A` and `impl A`.
- Evidence:
  - Structs are in [src/config/vm_schema.rs](src/config/vm_schema.rs#L202), [src/config/vm_schema.rs](src/config/vm_schema.rs#L215), [src/config/vm_schema.rs](src/config/vm_schema.rs#L270), [src/config/vm_schema.rs](src/config/vm_schema.rs#L296), [src/config/vm_schema.rs](src/config/vm_schema.rs#L307), [src/config/vm_schema.rs](src/config/vm_schema.rs#L92).
  - `From` impls are in [src/config/devices.rs](src/config/devices.rs#L8), [src/config/devices.rs](src/config/devices.rs#L36), [src/config/devices.rs](src/config/devices.rs#L158), [src/config/devices.rs](src/config/devices.rs#L201), [src/config/devices.rs](src/config/devices.rs#L221), [src/config/system.rs](src/config/system.rs#L15).
- Impact: Type behavior is split across files, increasing lookup cost and reducing locality for schema + conversion maintenance.

### 3. Medium: Section 13 large-struct cohesion rule is not met for several config types
- Guideline: Large structs (or structs with impl/trait impl sections) should live together with their impls in a single file per type.
- Evidence:
  - Large structs: [src/config/vm_schema.rs](src/config/vm_schema.rs#L11), [src/config/vm_schema.rs](src/config/vm_schema.rs#L146), [src/config/vm_schema.rs](src/config/vm_schema.rs#L215), [src/config/platform.rs](src/config/platform.rs#L401).
  - Related impls are split into other files: [src/config/loader.rs](src/config/loader.rs#L7), [src/config/devices.rs](src/config/devices.rs#L36), [src/config/system.rs](src/config/system.rs#L7).
- Impact: The type definition and behavior are still fragmented for key schema types.

### 4. Medium: Section 13 directory grouping recommendation is only partially met in loader area
- Guideline: If >=3 files in a directory have similar purpose, group them into a subdirectory.
- Evidence: [src/config/loader.rs](src/config/loader.rs), [src/config/loader_env.rs](src/config/loader_env.rs), [src/config/loader_merge.rs](src/config/loader_merge.rs) are clearly one concern family but remain at the root of `src/config`.
- Impact: Flat directory growth continues despite strong topical affinity across loader files.

### 5. Medium: Section 2 file-length threshold (250 lines) is exceeded by many files
- Guideline: file length target under 250 lines when possible.
- Evidence (current Rust files over threshold):
  - [src/config/tests.rs](src/config/tests.rs)
  - [src/qemu/args.rs](src/qemu/args.rs)
  - [tests/config_tests.rs](tests/config_tests.rs)
  - [src/cli/runtime.rs](src/cli/runtime.rs)
  - [src/config/validation/platform.rs](src/config/validation/platform.rs)
  - [src/cli.rs](src/cli.rs)
  - [src/qemu/mod.rs](src/qemu/mod.rs)
  - [src/config/platform.rs](src/config/platform.rs)
  - [src/config/devices.rs](src/config/devices.rs)
  - [tests/integration_tests.rs](tests/integration_tests.rs)
  - [src/qemu/process.rs](src/qemu/process.rs)
  - [src/config/vm_schema.rs](src/config/vm_schema.rs)
  - [src/network.rs](src/network.rs)
  - [src/storage.rs](src/storage.rs)
  - [src/state.rs](src/state.rs)
  - [src/config/validation/devices.rs](src/config/validation/devices.rs)
- Impact: The codebase still has multiple high-density files that are hard to navigate and review.

### 6. Medium: Section 2 function-length threshold (35 lines) is exceeded in core production paths
- Guideline: function length target under 35 lines when possible.
- Evidence (selected high-impact production functions):
  - [src/qemu/mod.rs](src/qemu/mod.rs#L68) `build_command` (257 lines)
  - [src/cli/runtime.rs](src/cli/runtime.rs#L536) `handle_start` (167 lines)
  - [src/config/devices.rs](src/config/devices.rs#L37) `From<DriveConfig>::from` (119 lines)
  - [src/config/validation/devices.rs](src/config/validation/devices.rs#L25) `validate_drive_config` (107 lines)
  - [src/cli/runtime.rs](src/cli/runtime.rs#L304) `start_swtpm_if_configured` (105 lines)
  - [src/config/validation/platform.rs](src/config/validation/platform.rs#L222) `validate_audio_devices` (104 lines)
  - [src/cli/commands.rs](src/cli/commands.rs#L34) `handle_storage` (91 lines)
  - [src/qemu/mod.rs](src/qemu/mod.rs#L327) `build_boot_args` (88 lines)
- Audit note: parser-based scan found 40 production functions over 35 lines (excluding `test_` functions and excluding [src/config/tests.rs](src/config/tests.rs)).
- Impact: Several core execution paths remain monolithic and above guideline targets.

### 7. Low: Section 2 struct-length threshold (35 lines) is exceeded in key schema and command types
- Guideline: struct length target under 35 lines when possible.
- Evidence:
  - [src/config/vm_schema.rs](src/config/vm_schema.rs#L11) `VmConfig` (91 lines)
  - [src/qemu/types.rs](src/qemu/types.rs#L10) `QemuArgs` (53 lines)
  - [src/config/vm_schema.rs](src/config/vm_schema.rs#L215) `DriveConfig` (52 lines)
  - [src/config/platform.rs](src/config/platform.rs#L401) `HypervConfig` (49 lines)
  - [src/config/vm_schema.rs](src/config/vm_schema.rs#L146) `BootConfig` (46 lines)
- Impact: Larger data types are still common and should either be justified with comments or split into focused subtypes where reasonable.

## Scope and Method
- Reviewed all Rust source files under [src](src) and [tests](tests).
- Focused checks: Section 2 thresholds and Section 13 structural rules.
- Used repository-wide file inventory, line counts, `mod.rs` signature scanning, `impl From` placement scanning, and parser-like function/struct length analysis.

## Current Gate Status
- The repository currently remains quality-gate clean:
  - `cargo fmt --all --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --quiet`
