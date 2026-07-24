# Phase 7: QEMU Cmdline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-24
**Phase:** 07-qemu-cmdline
**Areas discussed:** Device coverage / base-args scope, Testing strategy, Error handling strictness

---

## Device coverage / base-args scope

| Option | Description | Selected |
|--------|-------------|----------|
| All root devices via device_kind() | Phase 7 handles ALL root devices, including pre-existing ones (Memory → -m, Q35Chipset → -machine q35, storage/network/usb) — not just the 7 new v1 types | ✓ |
| Only 7 v1 types | Pre-existing device types already have emitters elsewhere | |
| Host args out of scope | Host/process-level args hardcoded/deferred | |

**User's choice:** All root devices via device_kind() dispatch, including pre-existing ones.
**Notes:** Follow-up question clarified the host/process-level args boundary specifically.

## Host/process-level args boundary

| Option | Description | Selected |
|--------|-------------|----------|
| In scope now | QemuContext carries vm_name/uuid/paths; emitter generates -id/-name/-smbios/-pidfile/-daemonize/-boot/-global/-cpu/-smp | |
| Out of scope | Defer all host/process-level args to Phase 8 | |
| Partial | Emit only what's needed to make cmdline runnable for testing (-machine, -smp, -m, -cpu); defer daemon/process flags (-pidfile, -daemonize) to Phase 8 | ✓ |

**User's choice:** Partial — runnable-for-testing base args now, daemon/process-management flags deferred to Phase 8.
**Notes:** None.

---

## Testing strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Golden fixture | Build Runtime from felucia/108.conf corpus via existing Proxmox importer; assert emitted cmdline matches key tokens/ordering from 108.qemu.cmd | ✓ |
| Synthetic only | Hand-construct minimal Runtime fixtures per device type, no real-world comparison | |
| Both | Synthetic per-device + one felucia-based ordering/coverage test | |

**User's choice:** Golden fixture (felucia corpus).
**Notes:** Follow-up question clarified exact-match vs structural-match expectations.

## Testing strategy — match strictness

| Option | Description | Selected |
|--------|-------------|----------|
| Structural match only | Assert presence/ordering of expected tokens; don't expect byte-for-byte match | ✓ |
| Exact match | Emitted cmdline should exactly reproduce 108.qemu.cmd token-for-token | |
| More questions | — | |

**User's choice:** Structural match only.
**Notes:** ezkvm's generated cmdline is expected to differ from Proxmox's `qm`-generated one; byte-for-byte / boot verification belongs to Phase 9 (QEMU-04).

---

## Error handling strictness

| Option | Description | Selected |
|--------|-------------|----------|
| Typed error | QemuConversionError gains variants for missing/invalid context data; TryFrom returns Err | |
| Panic/unwrap | Missing context data treated as programmer error since QemuContext is built internally | ✓ |
| Best-effort defaults | Fall back to sensible defaults rather than erroring | |

**User's choice:** Panic/unwrap on missing context data.
**Notes:** QemuContext is internally constructed, not parsed from untrusted input — typed errors remain reserved for genuine Runtime-data conversion problems.

---

## the agent's Discretion

None — all areas resolved with explicit user decisions.

## Deferred Ideas

- Host/process-management args (`-id`, `-name`, `-smbios`, `-pidfile`, `-daemonize`, `-readconfig`) — Phase 8 (VM Lifecycle).
- Byte-for-byte / boot-verified cmdline correctness against real Proxmox output — Phase 9 (Round-Trip Verification, QEMU-04).
