# Review Checklist

Date: 2026-04-08  
Scope: Entire Rust source under src/  
Validation: cargo test (66 passed, 0 failed)

## Fix Now (High)

- [x] Passthrough TPM panics at runtime.
	- Location: src/vm/config/system/tpm/pass_through_tpm.rs:7, src/vm/config/system.rs:56
	- Impact: VM aborts instead of returning a clean validation error.
	- Trigger: system.tpm.type = passthrough.
	- Action: Implement passthrough TPM args or reject this variant during validation/deserialization and add a regression test.

- [x] Apple SMC is configured but never emitted into QEMU args.
	- Location: src/vm/config/system.rs:33, src/vm/config/system.rs:56, src/vm/config/system/applesmc.rs:17
	- Impact: macOS-oriented configs can fail due to missing device arg.
	- Trigger: Any config with applesmc set.
	- Action: Append applesmc args in System::get_qemu_args and add a test asserting presence.

- [x] Network device PCI address collisions across multiple NICs.
	- Location: src/vm/config/network/bridge.rs:24, src/vm/config/network/tap.rs:36, src/vm/config/network/x550vf.rs:30, src/vm/config.rs:249
	- Impact: Duplicate PCI addresses can fail or miswire devices.
	- Trigger: Multi-NIC configs.
	- Action: Derive unique addresses by index or allow explicit slot assignment; add a two-NIC test.

- [x] VNC TCP mapping likely inconsistent with QEMU syntax and may double-select display backend.
	- Location: src/vm/config/vnc.rs:42
	- Impact: Port/display mismatch and potential backend conflict.
	- Trigger: VNC TCP config.
	- Action: Normalize field semantics (display number vs TCP port), map consistently, remove conflicting duplicate display emission, add CLI-shape test.

- [ ] Unix socket RPC is not stream-message safe.
	- Location: src/rpc/connection/socket_connection.rs:23, src/rpc/connection/socket_connection.rs:37
	- Impact: Partial/coalesced frames can break JSON handling and cause flaky RPC behavior.
	- Trigger: Fragmented or merged reads.
	- Action: Add framing-aware buffered parsing and tests for split/combined frames.

## Next (Medium / Medium-High)

- [ ] VM startup rebuilds argv via whitespace split.
	- Location: src/vm/virtual_machine.rs:50
	- Impact: Args containing spaces are corrupted before spawn.
	- Trigger: Any arg value with whitespace.
	- Action: Pass original vector directly to Command::args; add path-with-space test.

- [ ] post_start hooks execute even when spawn fails.
	- Location: src/vm/virtual_machine.rs:64
	- Impact: Side processes may start after failed VM launch.
	- Trigger: Spawn errors (missing binary, invalid args, permission issues).
	- Action: Run post_start only after successful spawn; add negative-path test.

- [ ] extras are modeled but never appended to final QEMU args.
	- Location: src/vm/config.rs:117, src/vm/config.rs:153, src/vm/config.rs:217
	- Impact: User-supplied raw extras are silently ignored.
	- Trigger: Configs relying on extras.
	- Action: Append extras in Config::get_qemu_args at documented order point; add direct test.

- [ ] Config search locations are not honored by Osal.
	- Location: src/vm/config.rs:162, src/osal.rs:36
	- Impact: Lookup depends on current working directory.
	- Trigger: Config file not in cwd.
	- Action: Resolve and search all requested locations (including home expansion where intended); add filesystem-backed tests.

- [ ] CLI exposes QMP lifecycle commands that always return unsupported.
	- Location: src/args.rs:52, src/args.rs:73, src/vm/virtual_machine.rs:140
	- Impact: User-visible commands parse but fail at runtime.
	- Trigger: qmp-system-reset, qmp-system-powerdown, qmp-system-wakeup, qmp-quit.
	- Action: Implement via monitor path or hide/remove until supported.

## Hardening (Medium-Low)

- [ ] Top-level config tests sort args, masking ordering regressions.
	- Location: src/vm/config.rs:758
	- Impact: Ordering-sensitive regressions can pass tests.
	- Trigger: Refactors that reorder argument assembly.
	- Action: Keep order-sensitive assertions for critical arg sequences.

## Open Questions / Assumptions

- [ ] Confirm whether vnc.port is intended as TCP port or display number.
- [ ] Confirm passthrough TPM is meant to be supported now vs explicitly unsupported.
- [ ] Decide whether live integration validation (QEMU/QMP) is required beyond unit tests.

## Overall Status

- [ ] Runtime risk remains moderate despite green unit tests, concentrated in startup/argument composition, RPC stream handling, and partially exposed features.
