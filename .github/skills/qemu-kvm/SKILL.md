---
name: qemu-kvm
description: 'Use when working on ezkvm QEMU or KVM features, configuration parsing, QEMU argument generation, device management, testing, or project-specific runtime behavior.'
---

# qemu-kvm

**Scope**: Workspace skill for `ezkvm` project-specific QEMU/KVM workflows.

## Description

This skill captures the recurring `ezkvm` development workflow for QEMU/KVM integration, configuration parsing, CLI command expansion, storage/network/device management, and documentation/testing. Use it when working on code changes, bug fixes, or feature additions related to QEMU/KVM behavior in this repository.

## Use when

- implementing or extending `ezkvm` CLI commands for QEMU/KVM features
- adding or refining YAML configuration parsing and validation
- generating or validating QEMU command-line arguments
- integrating storage, network, or device management support
- adding unit/integration tests for `ezkvm` behavior
- documenting architecture, usage, or migration guidance

## Workflow

1. **Understand the current design**
   - `src/config/` handles YAML schema, validation, and environment variable substitution
   - `src/cli.rs` defines CLI commands and dispatches handlers
   - `src/qemu/` generates QEMU execution logic and process management
   - `src/state.rs` tracks VM status and PID/state artifacts
   - `README.md` and `ProjectPlan.md` document the project scope and user-facing guidance

2. **Review relevant areas first**
   - config parsing/validation for required fields and defaults
   - command dispatch in `src/cli.rs`
   - QEMU command builder and executor behavior
   - storage/device/network helper functions and existing process utilities

3. **Implement feature changes**
   - add new CLI subcommands only when the command model is clear
   - keep config types and YAML layout human-readable and backward-compatible
   - use `anyhow::Result` and clear error messages for invalid configs or missing binaries
   - prefer dry-run support before actual QEMU execution
   - preserve current command patterns and CLI ergonomics

4. **Validate behavior**
   - run `cargo build` and `cargo test`
   - verify `./target/debug/ezkvm --help` lists new commands
   - check storage and network command help output
   - test sample commands with `qemu-img` and present host tooling if available

5. **Document and finalize**
   - update `README.md` to include new commands and examples
   - keep `ProjectPlan.md` aligned with current phase and implementation status
   - capture feature coverage and test results in the README or progress notes

## Field Debug Playbook (Portable Linux)

Use this quick sequence before changing code when imported VMs behave unexpectedly:

1. **Bridge path verification first**
   - Confirm bridge exists and is up (`ip -br link show br0`)
   - Confirm bridge-helper ACL (`/etc/qemu/bridge.conf`) includes requested bridge
   - Confirm QEMU actually emitted bridge backend args (`-netdev ... br=<bridge> ...`)

2. **If guest gets DHCP but no internet**
   - Treat as host routing/NAT issue, not guest NIC mapping issue
   - Check `sysctl -n net.ipv4.ip_forward` (must be `1`)
   - Add/verify NAT + FORWARD policy from bridge subnet to uplink

3. **Display isolation strategy for black-screen boots**
   - Isolate with `vnc` + `devices.displays: [{type: vga}]`
   - Remove SPICE/QXL variables while keeping storage/controller baseline fixed
   - Test one variable at a time; avoid changing storage + display simultaneously

4. **Shutdown analysis before force-stop**
   - If VM appears hung with high vCPU usage, check process args for `-no-shutdown`
   - Verify with `ezkvm status` over time before concluding process leak
   - Prefer evidence gathering first, force-stop only when user requests

## Decision points

- **Backend support**: `ezkvm` currently targets QEMU only; do not introduce alternative backends without explicit plan updates
- **Machine type policy**: choose machine type intentionally (`q35` for PCIe-first topologies and passthrough-heavy modern guests; `i440fx` for legacy compatibility cases)
- **Q35 topology policy**: keep PCIe devices on PCIe root/downstream ports and place legacy PCI devices behind `pcie-pci-bridge`/`pci-bridge`
- **Topology complexity**: keep PCIe hierarchy flat unless bus budgeting requires switch depth
- **Config validation**: reject unsupported architectures or invalid resource values early
- **Command behavior**: separate interactive vs daemon execution clearly
- **Storage/network subcommands**: implement minimal helper functionality first, then expand with real QEMU integration
- **Documentation**: every new CLI command should be reflected in `README.md`

## Quality criteria

- new code compiles cleanly with the current toolchain
- tests pass for unit and integration coverage
- CLI help reflects new command surface
- YAML config parsing and environment var substitution behave consistently
- `qemu-img` operations are wrapped with helpful output and error handling
- README includes usage examples and migration guidance where appropriate

## Example prompts

- `Use the qemu-kvm skill to extend ezkvm with a new network bridge command.`
- `Use the qemu-kvm skill to add support for storage snapshot management in ezkvm.`
- `Use the qemu-kvm skill to fix YAML parsing and env var substitution for ezkvm config files.`
- `Use the qemu-kvm skill to update README documentation for the new storage and device commands.`

## Next customization ideas

- Add a file-scoped instruction for `src/cli.rs` and `src/config/` to enforce CLI style and config validation rules
- Create a prompt template for `ezkvm` feature implementation requests, focusing on QEMU/KVM command generation and testing
- Add a workspace-level `copilot-instructions.md` for `ezkvm` coding conventions and Rust module patterns
