# Phase 8: VM Lifecycle - Context

**Gathered:** 2026-07-24
**Status:** Ready for planning

<domain>

## Phase Boundary

ezkvm gains a real CLI entry point (`main.rs` today is just a demo script) that can start, stop, kill, reset, and query the status of a VM defined by an ezkvm YAML config. This requires:

- A host-level configuration file (`host.yaml`) defining directories (config, VM configs, tmp, pid/state, sockets), tool paths/defaults (qemu, swtpm, looking-glass-client, remote-viewer), and a host-resources schema (pci/usb devices) whose *allocation logic* is explicitly out of scope for this phase.
- A `VmHandle`/state-file mechanism that survives across separate CLI process invocations (start in one process, stop/kill/reset/status in another).
- QEMU monitor (QMP over Unix socket) communication for `system_powerdown`, `quit`, `system_reset`.
- Automatic (but overridable) launch of a UI client (remote-viewer / looking-glass-client) mapped from the VM's `display:` config, plus cleanup of that client on VM stop/kill.
- swtpm readiness polling before qemu is allowed to connect to its socket.

</domain>

<decisions>

## Implementation Decisions

### CLI Surface & Host Configuration
- **D-01:** `ezkvm <verb> <vm-name>` — VMs are referenced by name, not by file path. ezkvm looks up `<vm-name>.yaml` in a configured VM directory, not an arbitrary path passed on every invocation.
- **D-02:** Verbs for this phase: `start`, `stop`, `kill`, `reset`, `status` (status added beyond the roadmap's original 4, per user request).
- **D-03:** A host-level config file `host.yaml` lives at `<config_dir>/host.yaml`. `config_dir` defaults to `/etc/ezkvm`, overridable via `--config-dir` CLI flag.
- **D-04:** `host.yaml` fields (this phase, schema only where noted):
  - `vm_dir` — VM config directory, defaults to `<config_dir>/vm.d`.
  - Other directory references: temp files, pid/state files, socket files (paths/names to be finalized by the planner/researcher within this decision's intent).
  - Tool paths and default commandline options for: `qemu` executable, `swtpm`, `looking-glass-client`, `remote-viewer`.
  - Host resources (pci/usb devices available for passthrough), structurally similar to `host.resources` in a VM config — **schema/shape only**, no allocation/reservation/conflict-checking logic this phase (that's future work).
- **D-05:** `host.yaml` uses plain YAML (same saphyr pipeline as VM configs) — not TOML or another format.
- **D-06:** If `host.yaml` is missing at the resolved `config_dir`, ezkvm fails with a clear error. No silent fallback to hardcoded defaults.

### VmHandle / State Persistence
- **D-07:** Runtime state (PIDs, socket paths) is persisted as a single combined state file per VM, in **YAML** (via the existing `crate::serde_yaml` pipeline, for consistency with every other ezkvm config file) at `<pid_dir>/<vm-name>.state` (or equivalent — exact location/dir key to be finalized against `host.yaml`'s dir fields from D-04), not separate conventional `.pid` files.
- **D-08:** The state file tracks **all** processes ezkvm spawned for that VM: qemu PID, swtpm PID (if TPM configured), and the UI client PID (remote-viewer / looking-glass-client), if one was auto-launched.
- **D-09:** Staleness detection is PID-liveness only (is the process still alive?) — not monitor-socket connectivity. A dead PID means the state file is stale and gets cleaned up automatically.
- **D-10:** `ezkvm start <vm-name>` on a VM whose state file shows a live PID fails with a clear error ("already running, stop it first") — it does NOT auto-stop the old instance.

### UI Client Launch Mapping
- **D-11:** The UI client to launch is inferred by default from the VM's `display:` config type (`Spice` → `remote-viewer`, `LookingGlass` → `looking-glass-client`, `Vnc` → `remote-viewer vnc://...`, others → no auto-launch) — using binary paths/default args sourced from `host.yaml` (D-04). This default mapping can be overridden per-VM in the VM's own YAML config.
- **D-12:** "No UI client" is a valid, expected outcome for headless/`EglHeadless`-style VMs — ezkvm does not error or force a client when the display type doesn't map to one.
- **D-13:** Timing: launch the UI client after a **fixed short delay** (~1-2s) post-qemu-start — no socket/port readiness polling for the display itself (contrast with swtpm, D-14, which does poll).
- **D-14 (swtpm crossover, see below).**
- **D-15:** If the UI client fails to launch (binary missing, immediate crash), ezkvm logs a warning and continues — the VM keeps running regardless. Matches the roadmap's explicit risk callout.

### swtpm Readiness & Process Supervision
- **D-14:** ezkvm polls the swtpm socket path until it's connectable (with a timeout that errors out if swtpm never becomes ready) before starting qemu — NOT a fixed delay.
- **D-16:** `ezkvm start` is fire-and-forget: after successfully launching qemu (+ swtpm, + optionally a UI client), the `ezkvm start` process exits/returns — it does not stay resident as a supervisor. — **Reversibility:** costly — **Rationale (user's own words):** "For now conform to behavior in item 1 [fire-and-forget]. In the future, a CLI option may be added to automatically connect to the qemu monitor cli for interactive use." Adding a supervisor/attached mode later is additive, not a breaking change, but retrofitting it onto a fire-and-forget process model built without that concern in mind could require rework of how state/exit codes are reported.
- **D-17:** If qemu fails to launch after swtpm already started successfully, ezkvm kills the now-orphaned swtpm process and cleans up the (partial) state file — no leftover orphaned swtpm processes.

### Stop / Kill / Reset Semantics
- **D-18:** `stop` sends `system_powerdown` via QMP and waits (blocking, indefinitely by default) for the qemu process to exit. It does NOT auto-escalate to `kill` unless auto-escalation is explicitly configured. — **Reversibility:** reversible — **Rationale:** purely a runtime behavior toggle, not a data format or API shape.
- **D-19:** Auto-escalation from `stop` to `kill` after a timeout is available as an **override**, configurable in `host.yaml` (host-wide default) or via a CLI flag (per-invocation override) — but is NOT the default behavior.
- **D-20:** When `stop`/`kill` succeeds (qemu process confirmed exited), ezkvm also terminates any still-running UI client process it had launched for that VM (D-08's tracked PID) — it does not leave the client running for the user to close manually.
- **D-21:** `reset` (sends `system_reset` via QMP) is fire-and-forget — no response/acknowledgement is awaited from QEMU; ezkvm sends the command and reports success without waiting for a QMP reply.

### the agent's Discretion
- Exact naming/layout of `host.yaml`'s directory keys beyond `vm_dir` (tmp dir, pid/state dir, socket dir key names) — user described the intent, not exact field names.
- Exact swtpm/UI-client readiness timeout durations (D-14's timeout, D-13's fixed delay length) — user did not specify exact numbers.
- Exact filename/location convention for the YAML state file under the pid/state dir (format itself is locked — see D-07).
- Exact QMP client implementation approach (raw JSON-over-Unix-socket vs a crate), as long as it satisfies D-18/D-19/D-21.

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Requirements
- `.planning/ROADMAP.md` §"Phase 8: VM Lifecycle" — goal, requirements (VMGR-01..05), success criteria, risks, and the originally-proposed 08-01..08-04 plan breakdown (now superseded/expanded by this CONTEXT.md's CLI-surface and host.yaml additions).
- `.planning/REQUIREMENTS.md` — VMGR-01 through VMGR-05 requirement text.

### Existing Display Schema (relevant to UI client mapping, D-11/D-12)
- `src/config/ezkvm/schema/display.rs` — `DisplaySchema` enum (`Vnc`, `Spice`, `EglHeadless`, `LookingGlass`, `Gtk`, `Sdl`) and each variant's fields (ports, listen addresses, TLS, GL/rendernode, clipboard, etc.). No client-launch binary/args fields exist yet — this phase must add that mapping (host.yaml defaults + per-VM override, per D-11).

### Existing QEMU Cmdline Emitter (Phase 7, just completed)
- `src/config/qemu.rs`, `src/config/qemu/handlers/*.rs` — `QemuCommandLine::try_from((Runtime, QemuContext))` is the source of the actual qemu argument list `ezkvm start` must launch. `QemuContext` already carries `tpm_socket_path: Option<String>` (D-06 from Phase 7) — this is the same socket path Phase 8's swtpm-readiness poll (D-14) must target before qemu is spawned.
- `.planning/phases/07-qemu-cmdline/07-CONTEXT.md` — D-06 (TPM socket path is caller-supplied via `QemuContext`, never parsed) — Phase 8 is the caller that must supply it, derived from `host.yaml`'s socket/tmp dir + vm-name.

### Current CLI Entry Point (to be replaced)
- `src/main.rs` — currently a hardcoded demo script (builds a fixed `Runtime`, round-trips through `EzkvmConfigSchema`, no argument parsing, no subcommands). This phase replaces it with the real `start`/`stop`/`kill`/`reset`/`status` CLI (D-01/D-02).

No external specs beyond the above — requirements for this phase are fully captured in the decisions above plus the roadmap's original success criteria.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crate::serde_yaml` module (wraps saphyr) — already used for VM config YAML; `host.yaml` (D-05) should reuse this same serialization path rather than introducing a second YAML library or format.
- `QemuCommandLine::try_from((Runtime, QemuContext))` (Phase 7) — the exact string to pass to the spawned `qemu-system-x86_64` process; `Display` impl renders it as a single space-joined string.
- `DisplaySchema` enum (`src/config/ezkvm/schema/display.rs`) — already has all the per-protocol fields (port, listen, tls, gl, rendernode, etc.) needed to construct a `remote-viewer`/`looking-glass-client` connection URI/args.

### Established Patterns
- `derive_getters::Getters` + `derive_new::new` on plain data structs throughout the codebase (Runtime types, schema types) — likely applies to any new `HostConfig`/`VmHandle`/state-file structs this phase introduces.
- `thiserror`-based typed error enums per module (e.g. `QemuConversionError` in `src/config/qemu.rs`) — new fallible operations (host.yaml load, VmHandle read/write, monitor communication) should follow this same pattern rather than `Result<_, ()>` or `anyhow`.

### Integration Points
- New CLI entry point in `src/main.rs` (replacing the current demo script) is where argument parsing, `host.yaml` loading, VM-name-to-YAML-path resolution, and dispatch to start/stop/kill/reset/status handlers all connect.
- The QEMU monitor client (QMP) is new code with no existing analog in the codebase — first phase to need process-external communication (Unix socket JSON-RPC-like protocol).

</code_context>

<specifics>

## Specific Ideas

- User explicitly described the `host.yaml` file's role in detail across multiple answers: it is the single source of truth for host-level paths (config/vm/tmp/pid/socket dirs), external tool locations + default args (qemu, swtpm, looking-glass-client, remote-viewer), and a host-resources schema (pci/usb) mirroring the existing `host.resources` shape used in VM configs — but allocation logic for host resources is explicitly deferred, this phase only needs the schema/shape to exist.
- User specifically called out that the state file must track not just the qemu PID but *every* process ezkvm spawns for a VM: qemu, swtpm (if present), and the UI client (if auto-launched) — this is a stronger requirement than the roadmap's original `VmHandle` wording ("tracking qemu PID, swtpm PID, monitor socket path, and UI client PID") but confirms and reinforces it.
- User anticipates a *future* CLI option to attach interactively to the QEMU monitor CLI after start — explicitly out of scope for this phase, but the fire-and-forget design (D-16) should not be so rigid that adding this later requires a rewrite (kept as a "costly, not one-way" reversibility note on D-16).

</specifics>

<deferred>

## Deferred Ideas

- **Host-resource pool allocation/reservation logic** (tracking which pci/usb devices are free vs. assigned to a running VM, conflict detection across concurrently-running VMs) — only the `host.yaml` schema/shape for declaring available host resources is in scope for Phase 8; the allocation logic itself is future work (likely its own phase or a Phase 8 follow-up).
- **Interactive QEMU monitor CLI attachment** (`ezkvm monitor <vm-name>` or similar, connecting a human to the running QMP/HMP session) — explicitly named by the user as a future CLI option, not part of this phase's fire-and-forget start model.

### Reviewed Todos (not folded)
None — discussion stayed within phase scope (no cross-referenced todos existed for this phase at discussion time).

</deferred>

---

*Phase: 8-VM Lifecycle*
*Context gathered: 2026-07-24*
