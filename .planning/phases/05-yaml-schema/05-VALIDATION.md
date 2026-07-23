# Phase 05 Validation Plan

## Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `cargo test` |
| Quick run | `cargo test --lib 2>&1 \| tail -20` |
| Full suite | `cargo test` |

## Phase Requirements → Test Map

| Req ID | Behavior | Test Name | Command | File |
|--------|----------|-----------|---------|------|
| YAML-03 | EfiDiskSchema serializes and deserializes losslessly | `efidisk_schema_round_trips` | `cargo test efidisk_schema_round_trips` | `src/config/ezkvm/schema/efidisk.rs` |
| YAML-03 | SwtpmSchema round-trips YAML with version as String | `swtpm_schema_round_trips_yaml` | `cargo test swtpm_schema_round_trips_yaml` | `src/config/ezkvm/schema/tpm.rs` |
| YAML-03 | HostPci variant round-trips YAML with `resource` ref | `hostpci_round_trips_yaml` | `cargo test hostpci_round_trips_yaml` | `src/config/ezkvm/schema/pcie.rs` |
| YAML-03 | PcieDeviceResourceSchema::HostAddress with `functions: Vec<u8>` round-trips | `hostpci_resource_round_trips_yaml` | `cargo test hostpci_resource_round_trips_yaml` | `src/config/ezkvm/schema/resources.rs` |
| YAML-03 | Memory ResourceSchema round-trips YAML | `memory_resource_round_trips_yaml` | `cargo test memory_resource_round_trips_yaml` | `src/config/ezkvm/schema/resources.rs` |
| YAML-03 | AudioDeviceSchema round-trips YAML | `audio_device_schema_round_trips` | `cargo test audio_device_schema_round_trips` | `src/config/ezkvm/schema/audio_device.rs` |
| YAML-03 | SpiceSchema round-trips YAML with gl/rendernode/clipboard | `spice_schema_round_trips_yaml` | `cargo test spice_schema_round_trips_yaml` | `src/config/ezkvm/schema/display.rs` |
| YAML-03 | RawArgsSchema preserves verbatim string including whitespace | `raw_args_round_trips` | `cargo test raw_args_round_trips` | `src/config/ezkvm/schema/rawargs.rs` |
| YAML-03 | ConfigSchema with all seven new fields compiles and emits valid YAML | `config_with_all_seven_types` | `cargo test config_with_all_seven_types` | `src/config/ezkvm/runtime/parser.rs` or `builder.rs` |

## Regression Gate

After all tasks complete, the full test suite must pass with no regressions:

```
cargo test
```

Expected: all pre-existing tests still pass (30 unit + 7 integration + 5 runtime_phase2).
