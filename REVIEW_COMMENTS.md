## Findings (Residual Only)

### 1. High: Section 13 `mod.rs` cleanliness remains partially unmet
- Guideline: `mod.rs` files should contain module wiring only (no `struct`, `fn`, `impl`).
- Residual files:
  - [src/cli/mod.rs](src/cli/mod.rs)
  - [src/cli/tests/mod.rs](src/cli/tests/mod.rs)
  - [src/config/loader/mod.rs](src/config/loader/mod.rs)
  - [src/config/tests/mod.rs](src/config/tests/mod.rs)
  - [src/network/mod.rs](src/network/mod.rs)
  - [src/qemu/command_builder/mod.rs](src/qemu/command_builder/mod.rs)
  - [src/state/mod.rs](src/state/mod.rs)
  - [src/storage/mod.rs](src/storage/mod.rs)
- Impact: Entry modules still mix wiring and implementation, reducing consistency with Section 13.

### 2. Medium: Section 2 file-length threshold still has five outliers
- Guideline: keep files under 250 lines when practical.
- Residual files:
  - [src/qemu/command_builder/composition.rs](src/qemu/command_builder/composition.rs) (261)
  - [src/qemu/args/devices.rs](src/qemu/args/devices.rs) (258)
  - [src/qemu/args/system.rs](src/qemu/args/system.rs) (254)
  - [tests/tests/config_parsing_env.rs](tests/tests/config_parsing_env.rs) (253)
  - [tests/tests/config_spice_usb_input.rs](tests/tests/config_spice_usb_input.rs) (251)
- Impact: These files are now the primary remaining candidates for targeted splits.

### 3. Medium: Section 2 function-length threshold still has residual outliers
- Guideline: keep functions under 35 lines when practical.
- Audit result: 33 production functions in `src/**` remain over 35 lines (excluding files under `/tests/`).
- Highest remaining outliers:
  - [src/config/vm_schema/drive.rs](src/config/vm_schema/drive.rs#L62) `From<DriveConfig>::from` (119)
  - [src/qemu/builder.rs](src/qemu/builder.rs#L35) `from_config` (67)
  - [src/storage/operations.rs](src/storage/operations.rs#L93) `create_snapshot` (62)
  - [src/network/firewall.rs](src/network/firewall.rs#L82) `setup_network_isolation` (61)
  - [src/cli/runtime/auxiliary/swtpm/preview.rs](src/cli/runtime/auxiliary/swtpm/preview.rs#L6) `build_swtpm_launch_preview` (57)
  - [src/cli/runtime/auxiliary/launch.rs](src/cli/runtime/auxiliary/launch.rs#L101) `build_looking_glass_launch` (56)
  - [src/config/validation/devices/network.rs](src/config/validation/devices/network.rs#L5) `validate_network_config` (54)
  - [src/cli/runtime/start.rs](src/cli/runtime/start.rs#L10) `handle_start` (52)
- Impact: Core paths are much improved, but a final pass is still needed to reach threshold targets.

## Scope and Method
- Reviewed all Rust files under [src](src) and [tests](tests).
- Focused checks: Section 2 thresholds and Section 13 `mod.rs` cleanliness.
- Used line-count scans, `mod.rs` signature scans, and parser-like function-length scans.

## Current Gate Status
- The repository remains quality-gate clean from the last validation pass:
  - `cargo fmt --all --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --quiet`
