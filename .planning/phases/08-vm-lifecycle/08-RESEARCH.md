# Phase 8: VM Lifecycle - Research

**Researched:** 2026-07-28
**Domain:** Rust CLI process orchestration (spawn/monitor/terminate `qemu-system-x86_64` + `swtpm` + UI client child processes), QEMU QMP (Unix-socket JSON-RPC) protocol
**Confidence:** HIGH

## Summary

Phase 8 replaces `src/main.rs`'s demo script with a real subcommand CLI (`ezkvm start|stop|kill|reset|status <vm-name>`) that reads a new host-level `host.yaml`, resolves `<vm-name>.yaml` to a `Runtime`, renders a `QemuCommandLine` (Phase 7 output) with a QMP socket flag appended, starts `swtpm` (if TPM configured) and polls its Unix socket for readiness, spawns `qemu-system-x86_64` as a detached process, persists a combined state file (`VmHandle`) recording every spawned PID, and optionally launches a UI client mapped from `display:` config. `stop`/`kill`/`reset` are separate CLI invocations that read the state file back, connect to the QMP Unix socket, negotiate capabilities, and send `system_powerdown`/`quit`/`system_reset` respectively.

Everything needed is either already a project dependency (`serde`, `thiserror`, `derive-getters`, `derive-new`) or in Rust `std` (`std::process::Command`, `std::os::unix::net::UnixStream`, `std::os::unix::process::CommandExt`). Two new crates are worth adding: `clap` (CLI parsing) and `serde_json` (QMP wire format — QMP is JSON, not YAML, so `crate::serde_yaml` does not apply here). PID-liveness checking can be done via `/proc/<pid>` inspection (no new dependency) since the target OS is Linux-only (matches the existing Proxmox/QEMU-only scope) — but `/proc/<pid>` alone conflates "alive" with "zombie"; the state must also check `/proc/<pid>/stat` for a `Z` (zombie) state, or add `libc` for a `kill(pid, 0)` liveness check instead. All four candidate crates (`clap`, `libc`, `nix`, `serde_json`) passed the package-legitimacy gate with `OK` verdicts and long-established GitHub repos.

A critical fact this phase must account for: Phase 7's `QemuCommandLine`/`QemuContext` do **not** emit a `-qmp` flag at all — QMP is host-plumbing, not a VM-schema-driven device, so it was correctly out of scope for Phase 7. Phase 8 must append `-qmp unix:<path>,server,nowait` (or the modern `-qmp unix:<path>,server=on,wait=off` spelling — both are accepted by QEMU 10.0, confirmed on this host) to the rendered command-line string at the call site, without needing to touch Phase 7's builder/handler internals.

**Primary recommendation:** Use `clap` (derive API) for the CLI, `serde_json` for QMP messages, `std::os::unix::net::UnixStream` for both QMP and swtpm-readiness polling, `std::process::Command` + `pre_exec`(`libc::setsid()`) + `Stdio::null()` for detached process spawning, and a single YAML `VmHandle` state file (via the project's existing `crate::serde_yaml` pipeline, for consistency with all other ezkvm config files) per VM under a `host.yaml`-configured state directory.

## Architectural Responsibility Map

Single-tier application — ezkvm is a CLI/host-process orchestrator, not a client/server web app. All capabilities below live in the "CLI / Host Process" tier (there is no browser, no frontend server, no separate API backend, no CDN). The one boundary worth naming explicitly is the distinction between "this process" (a single `ezkvm <verb>` invocation) and "the processes it spawns" (qemu, swtpm, UI client), because state must cross that boundary via the filesystem, not memory.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI arg parsing / subcommand dispatch | CLI / Host Process | — | Entry point, `main.rs` |
| `host.yaml` / VM YAML loading | CLI / Host Process | Filesystem | Config lives on disk, loaded per-invocation |
| Process spawning (qemu, swtpm, UI client) | CLI / Host Process | OS (Linux process model) | `std::process::Command`, detached via new session |
| State persistence (`VmHandle`) | Filesystem | CLI / Host Process | Must survive process exit — this is the only "storage" tier in this phase |
| QMP client (stop/kill/reset) | CLI / Host Process | OS (Unix domain socket) | New `ezkvm stop/kill/reset` process connects to a socket the qemu process already has open |
| swtpm readiness poll | CLI / Host Process | OS (Unix domain socket) | Same process that will spawn qemu, before spawning it |
| UI client launch mapping | CLI / Host Process | — | Reads `DisplaySchema` (and `Ivshmem` Runtime device for Looking Glass) to pick a binary + args |

## User Constraints

<user_constraints>

### Locked Decisions

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
- **D-07:** Runtime state (PIDs, socket paths) is persisted as a single combined state file per VM, in **YAML** (via `crate::serde_yaml`, for consistency with all other ezkvm config files) at `<pid_dir>/<vm-name>.state` (or equivalent — exact location/dir key to be finalized against `host.yaml`'s dir fields from D-04), not separate conventional `.pid` files.
- **D-08:** The state file tracks **all** processes ezkvm spawned for that VM: qemu PID, swtpm PID (if TPM configured), and the UI client PID (remote-viewer / looking-glass-client), if one was auto-launched.
- **D-09:** Staleness detection is PID-liveness only (is the process still alive?) — not monitor-socket connectivity. A dead PID means the state file is stale and gets cleaned up automatically.
- **D-10:** `ezkvm start <vm-name>` on a VM whose state file shows a live PID fails with a clear error ("already running, stop it first") — it does NOT auto-stop the old instance.
- **D-11:** The UI client to launch is inferred by default from the VM's `display:` config type (`Spice` → `remote-viewer`, `LookingGlass` → `looking-glass-client`, `Vnc` → `remote-viewer vnc://...`, others → no auto-launch) — using binary paths/default args sourced from `host.yaml` (D-04). This default mapping can be overridden per-VM in the VM's own YAML config.
- **D-12:** "No UI client" is a valid, expected outcome for headless/`EglHeadless`-style VMs — ezkvm does not error or force a client when the display type doesn't map to one.
- **D-13:** Timing: launch the UI client after a **fixed short delay** (~1-2s) post-qemu-start — no socket/port readiness polling for the display itself (contrast with swtpm, D-14, which does poll).
- **D-14:** ezkvm polls the swtpm socket path until it's connectable (with a timeout that errors out if swtpm never becomes ready) before starting qemu — NOT a fixed delay.
- **D-15:** If the UI client fails to launch (binary missing, immediate crash), ezkvm logs a warning and continues — the VM keeps running regardless. Matches the roadmap's explicit risk callout.
- **D-16:** `ezkvm start` is fire-and-forget: after successfully launching qemu (+ swtpm, + optionally a UI client), the `ezkvm start` process exits/returns — it does not stay resident as a supervisor. — **Reversibility:** costly — **Rationale:** a future CLI option may attach interactively to the QEMU monitor; fire-and-forget must not be built so rigidly that adding that later requires a rewrite.
- **D-17:** If qemu fails to launch after swtpm already started successfully, ezkvm kills the now-orphaned swtpm process and cleans up the (partial) state file — no leftover orphaned swtpm processes.
- **D-18:** `stop` sends `system_powerdown` via QMP and waits (blocking, indefinitely by default) for the qemu process to exit. It does NOT auto-escalate to `kill` unless auto-escalation is explicitly configured. — **Reversibility:** reversible.
- **D-19:** Auto-escalation from `stop` to `kill` after a timeout is available as an **override**, configurable in `host.yaml` (host-wide default) or via a CLI flag (per-invocation override) — but is NOT the default behavior.
- **D-20:** When `stop`/`kill` succeeds (qemu process confirmed exited), ezkvm also terminates any still-running UI client process it had launched for that VM (D-08's tracked PID) — it does not leave the client running for the user to close manually.
- **D-21:** `reset` (sends `system_reset` via QMP) is fire-and-forget — no response/acknowledgement is awaited from QEMU; ezkvm sends the command and reports success without waiting for a QMP reply.

### the agent's Discretion

- Exact naming/layout of `host.yaml`'s directory keys beyond `vm_dir` (tmp dir, pid/state dir, socket dir key names) — user described the intent, not exact field names.
- Exact swtpm/UI-client readiness timeout durations (D-14's timeout, D-13's fixed delay length) — user did not specify exact numbers.
- Exact filename/location convention for the YAML state file under the pid/state dir (format itself is locked — see D-07).
- Exact QMP client implementation approach (raw JSON-over-Unix-socket vs a crate), as long as it satisfies D-18/D-19/D-21.

### Deferred Ideas (OUT OF SCOPE)

- **Host-resource pool allocation/reservation logic** (tracking which pci/usb devices are free vs. assigned to a running VM, conflict detection across concurrently-running VMs) — only the `host.yaml` schema/shape for declaring available host resources is in scope for Phase 8; the allocation logic itself is future work.
- **Interactive QEMU monitor CLI attachment** (`ezkvm monitor <vm-name>` or similar, connecting a human to the running QMP/HMP session) — explicitly named by the user as a future CLI option, not part of this phase's fire-and-forget start model.

</user_constraints>

## Phase Requirements

<phase_requirements>

| ID | Description | Research Support |
|----|-------------|------------------|
| VMGR-01 | ezkvm starts a VM by launching `qemu-system-x86_64` (and `swtpm` when TPM is configured) as child processes | Process Spawning & Detachment; swtpm Readiness Polling; Code Examples §1-3 |
| VMGR-02 | After VM start, ezkvm launches the configured UI client (Looking Glass client or `remote-viewer`) to connect to the running VM | UI Client Launch Mapping; Code Examples §6 |
| VMGR-03 | ezkvm gracefully shuts down a running VM via QEMU monitor `system_powerdown` command | QMP Protocol Specifics; Code Examples §4 |
| VMGR-04 | ezkvm force-stops a running VM via QEMU monitor `quit` (or SIGTERM fallback) | QMP Protocol Specifics; Code Examples §4; Common Pitfalls (quit vs. powerdown semantics) |
| VMGR-05 | ezkvm resets a running VM via QEMU monitor `system_reset` command | QMP Protocol Specifics; Code Examples §4 |

</phase_requirements>

## Project Constraints (from copilot-instructions.md)

- Never run `git commit`/`git add` automatically in this session — this research file must be left untracked/uncommitted (matches `.planning/config.json`'s `commit_docs: false`, which this agent is honoring by skipping Step 7's commit).
- GSD-generated planning docs (PLAN.md and similarly-structured CONTEXT/RESEARCH/VALIDATION docs) must format pseudo-XML sections with blank lines after opening/before closing tags, atomic bullets instead of dense paragraphs, code blocks for multi-line code, and no HTML entities — applied throughout this document.
- Project tech stack constraint (from `PROJECT.md`): "Rust (edition 2024) — all implementation in Rust; no runtime deps outside Cargo." Read as: any new external tool invoked as a child process (qemu, swtpm, remote-viewer, looking-glass-client) is fine (these are the product's whole purpose), but no shell-script wrappers or non-Cargo build tooling should be introduced for Phase 8's own logic.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `clap` | 4.6.4 (derive feature) `[VERIFIED: crates.io registry, package-legitimacy OK]` | CLI subcommand parsing (`start/stop/kill/reset/status <vm-name>`, `--config-dir` global flag) | De facto standard Rust CLI crate; `#[derive(Parser)]` + `#[derive(Subcommand)]` gives typed subcommands with near-zero boilerplate; not currently a project dependency (confirmed absent from `Cargo.toml`) |
| `serde_json` | 1.0.151 `[VERIFIED: crates.io registry, package-legitimacy OK]` | QMP message serialization/deserialization (`{"execute": "...", "arguments": {...}}` / `{"return": ...}` / `{"error": {...}}`) | QMP is JSON-over-Unix-socket, not YAML — `crate::serde_yaml` (the project's saphyr wrapper) is the wrong tool here; `serde_json` is the standard, and the project already depends on `serde` with `derive`, so this is additive, not a new serialization paradigm |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `libc` | 0.2.189 `[VERIFIED: crates.io registry, package-legitimacy OK]` | `kill(pid, 0)` liveness probe (returns `ESRCH` if the process doesn't exist, `0`/`EPERM` if it does) via `pre_exec`'s `libc::setsid()` for process detachment | Use if you want a liveness check that also distinguishes "no permission" from "doesn't exist" and don't want to hand-parse `/proc/<pid>/stat` for zombie detection |
| *(none — use `std::fs`)* | — | Alternative PID-liveness check via `/proc/<pid>` existence + `/proc/<pid>/stat` state field | Use if you'd rather avoid adding `libc` at all; requires manually parsing the third whitespace-delimited field of `/proc/<pid>/stat` for `Z` (zombie) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `libc::setsid()` in `pre_exec` | `nix` crate (`nix::unistd::setsid()`) | `nix` gives a safe wrapper (no `unsafe` block needed for the call itself, though `pre_exec`'s closure is still `unsafe` by the `std` API contract) at the cost of one more dependency; `libc` is already the lower-level building block `nix` itself depends on, so pulling `libc` directly is more minimal |
| `Command::process_group(0)` (stable since Rust 1.64) | `pre_exec` + `libc::setsid()` | `process_group` only creates a new **process group** (blocks foreground `Ctrl-C`/`SIGINT` propagation) — it does **not** create a new **session**, so the child is still tied to the controlling terminal and receives `SIGHUP` if the terminal closes. `setsid()` creates a new session (detaches from the controlling terminal entirely), which is what "survives the parent `ezkvm start` process exiting" (D-16) actually requires. Use `process_group` only if terminal-close survival is not a hard requirement — but `setsid()` is the correct choice for this phase's fire-and-forget model |
| Raw `std::os::unix::net::UnixStream` QMP client | A dedicated `qapi`/`qmp` crate | No mature, actively-maintained `qmp`-client-only crate was found with the confidence needed to recommend it over ~40 lines of hand-rolled JSON-line-framed socket I/O using already-a-dependency `serde_json`; the protocol is simple enough (newline-delimited JSON objects) that hand-rolling is the pragmatic choice here, not "hand-rolling something complex" (see Don't Hand-Roll section for the line to not cross) |

**Installation:**
```bash
cargo add clap --features derive
cargo add serde_json
cargo add libc
```

**Version verification:** All three verified against the crates.io registry API directly (`curl https://crates.io/api/v1/crates/<name>`) on 2026-07-28, cross-checked against `gsd-tools query package-legitimacy check --ecosystem crates`. All returned `verdict: OK` with GitHub source repos, decade-plus publish history, and tens-of-millions of weekly downloads each — no `[SUS]`/`[SLOP]` findings.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|--------------|---------|-------------|
| `clap` | crates.io | ~11 yrs (since 2015-03-01) | ~16M/wk | `github.com/clap-rs/clap` | OK | Approved |
| `serde_json` | crates.io | ~11 yrs (since 2015-08-07) | ~19M/wk | `github.com/serde-rs/json` | OK | Approved |
| `libc` | crates.io | ~11 yrs (since 2015-01-15) | ~24M/wk | `github.com/rust-lang/libc` | OK | Approved |
| `nix` | crates.io | ~11 yrs (since 2014-11-11) | ~12M/wk | `github.com/nix-rust/nix` | OK | Approved (listed as an alternative, not required if `libc` is chosen) |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
                         ┌──────────────────────────────┐
                         │  ezkvm <verb> <vm-name>      │  (new process each invocation)
                         │  --config-dir <dir>          │
                         └──────────────┬───────────────┘
                                        │ clap parses argv → Verb enum
                                        ▼
                         ┌──────────────────────────────┐
                         │ Load host.yaml (config_dir)  │──▶ fail hard if missing (D-06)
                         └──────────────┬───────────────┘
                                        │
                                        ▼
                         ┌──────────────────────────────┐
                         │ Resolve <vm-name>.yaml       │──▶ EzkvmConfigSchema::from_str
                         │ (host.yaml's vm_dir)         │      → Runtime::try_from(schema)
                         └──────────────┬───────────────┘
                                        │
                    ┌───────────────────┼────────────────────────┐
                    ▼                   ▼                        ▼
              verb == start       verb ∈ {stop,kill,reset}   verb == status
                    │                   │                        │
                    ▼                   ▼                        ▼
       ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
       │ Read VmHandle state │  │ Read VmHandle state │  │ Read VmHandle state │
       │ file — must be      │  │ file — must exist & │  │ file; PID-liveness  │
       │ absent/dead (D-10)  │  │ qemu PID alive      │  │ check each tracked  │
       └──────────┬──────────┘  └──────────┬──────────┘  │ PID; print report   │
                  │                        │             └─────────────────────┘
                  ▼                        ▼
     ┌──────────────────────┐   ┌──────────────────────────┐
     │ swtpm spawn (if TPM) │   │ Connect UnixStream to    │
     │ + poll socket ready  │   │ QMP socket path from     │
     │ (D-14, no fixed      │   │ VmHandle                 │
     │ sleep)               │   └──────────┬───────────────┘
     └──────────┬───────────┘              │
                ▼                          ▼
     ┌──────────────────────┐   ┌──────────────────────────┐
     │ Render QemuCommand-  │   │ qmp_capabilities         │
     │ Line (Phase 7) +     │   │ handshake, then          │
     │ append -qmp unix:... │   │ system_powerdown /       │
     └──────────┬───────────┘   │ quit / system_reset      │
                ▼               └──────────┬───────────────┘
     ┌──────────────────────┐               ▼
     │ Spawn qemu detached  │   ┌──────────────────────────┐
     │ (setsid via pre_exec)│   │ stop/kill: wait for qemu │
     └──────────┬───────────┘   │ PID to exit (blocking);  │
                ▼               │ then kill tracked UI     │
     ┌──────────────────────┐   │ client PID (D-20)        │
     │ Fixed delay (~1-2s)  │   └──────────┬───────────────┘
     │ then launch UI client│              ▼
     │ (D-13, D-11/D-12)    │   ┌──────────────────────────┐
     └──────────┬───────────┘   │ Delete/rewrite VmHandle  │
                ▼               │ state file               │
     ┌──────────────────────┐   └──────────────────────────┘
     │ Write VmHandle state │
     │ file (qemu/swtpm/    │
     │ client PIDs, sockets)│
     └──────────┬───────────┘
                ▼
        process exits (D-16, fire-and-forget)
```

### Recommended Project Structure

```
src/
├── main.rs                   # clap CLI entry point, verb dispatch only
├── lifecycle/
│   ├── mod.rs                # re-exports
│   ├── host_config.rs        # HostConfig struct + from-YAML loader (D-03..D-06)
│   ├── vm_handle.rs          # VmHandle struct + state-file read/write/staleness (D-07..D-10)
│   ├── process.rs            # detached spawn helpers (setsid, stdio redirection)
│   ├── readiness.rs           # Unix-socket connect-with-timeout poll (D-14)
│   ├── qmp.rs                 # QMP client: handshake + system_powerdown/quit/system_reset
│   └── ui_client.rs           # DisplaySchema → binary/args mapping (D-11..D-13, D-15)
├── config/                    # unchanged — existing schema/runtime/qemu layers
├── runtime/                   # unchanged
└── serde_yaml.rs               # unchanged — NOT used for QMP (see Standard Stack)
```

### Pattern 1: Detached Child Process Spawning

**What:** Spawn `qemu-system-x86_64` (or `swtpm`, or a UI client) so it survives the spawning `ezkvm` process's exit, without inheriting the parent's controlling terminal (so it doesn't get `SIGHUP`/`SIGINT` from that terminal later).

**When to use:** Every process this phase spawns (qemu, swtpm, remote-viewer/looking-glass-client) — this is D-16's fire-and-forget requirement made concrete.

**Example:**
```rust
// Source: std docs — std::os::unix::process::CommandExt (doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html)
// `.setsid()` on Command is nightly-only (tracking issue #105376) — do NOT reach for it on stable.
// Use `pre_exec` + `libc::setsid()` instead. `process_group()` (stable since 1.64) is NOT
// equivalent — it blocks SIGINT propagation but does not detach from the controlling
// terminal, so the child still dies on SIGHUP when the terminal closes.
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

fn spawn_detached(program: &str, args: &[String]) -> std::io::Result<std::process::Child> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null()) // or redirect to a log file under host.yaml's tmp/log dir
        .stderr(Stdio::null());

    unsafe {
        cmd.pre_exec(|| {
            // Runs in the forked child, before exec. Must only call async-signal-safe
            // functions here — setsid() is async-signal-safe.
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    cmd.spawn()
}
```

### Pattern 2: QMP Client — Capabilities Handshake Then Commands

**What:** QMP requires a specific sequence: connect → read the server's greeting JSON → send `qmp_capabilities` → read its `{"return": {}}` → only then are other commands accepted.

**When to use:** Every `stop`/`kill`/`reset` invocation, and D-14's readiness check is a *separate*, lower-level step (raw connect, not QMP handshake) that must complete before qemu itself is even started.

**Example:**
```rust
// Source: QEMU QMP spec — qemu.org/docs/master/interop/qmp-spec.html (fetched 2026-07-28)
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

#[derive(Debug, thiserror::Error)]
pub enum QmpError {
    #[error("failed to connect to QMP socket at {path}: {source}")]
    Connect { path: String, source: std::io::Error },
    #[error("QMP socket closed before greeting was received")]
    NoGreeting,
    #[error("qmp_capabilities negotiation failed: {desc}")]
    CapabilitiesFailed { desc: String },
    #[error("QMP command '{command}' returned an error: {desc}")]
    CommandFailed { command: String, desc: String },
    #[error("I/O error communicating with QMP socket: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse QMP JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct QmpClient {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
}

impl QmpClient {
    pub fn connect(socket_path: &str) -> Result<Self, QmpError> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|source| QmpError::Connect { path: socket_path.to_string(), source })?;
        let reader = BufReader::new(stream.try_clone()?);
        let mut client = Self { stream, reader };

        // 1. Read the server greeting: {"QMP": {"version": ..., "capabilities": [...]}}
        let mut line = String::new();
        if client.reader.read_line(&mut line)? == 0 {
            return Err(QmpError::NoGreeting);
        }

        // 2. Negotiate capabilities — required before any other command is accepted.
        let response = client.execute("qmp_capabilities", None)?;
        if response.get("error").is_some() {
            return Err(QmpError::CapabilitiesFailed {
                desc: response["error"]["desc"].as_str().unwrap_or("unknown").to_string(),
            });
        }

        Ok(client)
    }

    /// Sends `{"execute": command, "arguments": args}` and returns the parsed JSON response.
    /// Caller inspects `.get("return")` vs `.get("error")`.
    pub fn execute(
        &mut self,
        command: &str,
        args: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, QmpError> {
        let mut request = serde_json::json!({ "execute": command });
        if let Some(a) = args {
            request["arguments"] = a;
        }
        let mut payload = serde_json::to_string(&request)?;
        payload.push('\n');
        self.stream.write_all(payload.as_bytes())?;

        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(&line)?)
    }

    /// D-18: system_powerdown — response confirms QEMU *accepted* the request, not that
    /// the guest has shut down. Caller must separately poll qemu's PID for exit.
    pub fn system_powerdown(&mut self) -> Result<(), QmpError> {
        let response = self.execute("system_powerdown", None)?;
        if let Some(err) = response.get("error") {
            return Err(QmpError::CommandFailed {
                command: "system_powerdown".to_string(),
                desc: err["desc"].as_str().unwrap_or("unknown").to_string(),
            });
        }
        Ok(())
    }

    /// D-19/VMGR-04: quit — terminates the QEMU *process* itself (not just the guest OS).
    /// Per the QMP spec: "a premature EOF would not be unexpected" — a closed socket
    /// with no response is a SUCCESS signal here, not a protocol error.
    pub fn quit(&mut self) -> Result<(), QmpError> {
        match self.execute("quit", None) {
            Ok(response) => {
                if let Some(err) = response.get("error") {
                    return Err(QmpError::CommandFailed {
                        command: "quit".to_string(),
                        desc: err["desc"].as_str().unwrap_or("unknown").to_string(),
                    });
                }
                Ok(())
            }
            // EOF/connection-reset while reading the response is expected per spec.
            Err(QmpError::Io(_)) => Ok(()),
            Err(other) => Err(other),
        }
    }

    /// D-21: reset is fire-and-forget — write the request, do NOT read/await the response.
    pub fn system_reset_fire_and_forget(&mut self) -> Result<(), QmpError> {
        let request = serde_json::json!({ "execute": "system_reset" });
        let mut payload = serde_json::to_string(&request)?;
        payload.push('\n');
        self.stream.write_all(payload.as_bytes())?;
        Ok(())
    }
}
```

### Pattern 3: Readiness Polling (swtpm Socket, D-14)

**What:** Poll a Unix socket path with a bounded retry loop + timeout, rather than a fixed sleep.

**Example:**
```rust
// Source: std docs — no external crate needed; std::os::unix::net::UnixStream + std::time.
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum ReadinessError {
    #[error("socket at {path} did not become ready within {timeout_secs}s")]
    Timeout { path: String, timeout_secs: u64 },
}

pub fn wait_for_socket(path: &str, timeout: Duration) -> Result<(), ReadinessError> {
    let deadline = Instant::now() + timeout;
    let poll_interval = Duration::from_millis(50);
    loop {
        if UnixStream::connect(path).is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(ReadinessError::Timeout {
                path: path.to_string(),
                timeout_secs: timeout.as_secs(),
            });
        }
        std::thread::sleep(poll_interval);
    }
}
```

### Pattern 4: PID Liveness Check

**Example (no new dependency, `/proc`-based):**
```rust
// Source: Linux /proc(5) semantics — [ASSUMED: standard Linux /proc behavior, not
// fetched from a specific doc page this session, but a well-known, stable kernel ABI].
use std::fs;

/// Returns true only if the PID exists AND is not a zombie ('Z' state). A zombie means
/// the process has exited but not yet been reaped by its parent — treat as "not alive"
/// for D-09's staleness check, since a zombie's resources (sockets, etc.) are gone.
pub fn is_pid_alive(pid: u32) -> bool {
    let stat_path = format!("/proc/{}/stat", pid);
    let Ok(contents) = fs::read_to_string(&stat_path) else {
        return false; // ENOENT → process does not exist
    };
    // Format: "pid (comm) state ...". `comm` may itself contain spaces/parens, so find
    // the LAST ')' before reading the state field, not the first.
    match contents.rfind(')') {
        Some(idx) => contents[idx + 1..]
            .split_whitespace()
            .next()
            .map(|state| state != "Z")
            .unwrap_or(false),
        None => false,
    }
}
```

**Example (with `libc`, distinguishes "no permission" from "doesn't exist"):**
```rust
// Source: POSIX kill(2) — sending signal 0 checks existence/permission without
// actually sending a signal. [CITED: man7.org/linux/man-pages/man2/kill.2.html]
pub fn is_pid_alive_libc(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true; // process exists and we have permission
    }
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    errno == libc::EPERM // exists but owned by another user — still "alive"
}
```

### Anti-Patterns to Avoid

- **Using `Command::setsid()` on stable Rust:** it is a nightly-only experimental API (`process_setsid`, tracking issue rust-lang/rust#105376) — compiling against it requires `#![feature(...)]` and a nightly toolchain, which contradicts this being a normal `cargo build` project. Use `pre_exec` + `libc::setsid()` instead.
- **Confusing `process_group()` with true session detachment:** `process_group()` (stable since 1.64) only changes the PGID — it stops the child from receiving `SIGINT` sent to the parent's foreground process group, but the child is still attached to the controlling terminal and will still receive `SIGHUP` if that terminal closes. D-16 (survive the parent exiting, indefinitely) needs a real `setsid()`.
- **Awaiting a QMP response after `quit`:** the spec explicitly says a premature EOF is expected — treating a closed connection as an error will make VMGR-04's "force-stop" path report false failures.
- **Treating `system_powerdown`'s successful QMP response as "the VM is off":** the response only confirms QEMU *received* the request; the guest OS may take any amount of time (or refuse) to actually power down. D-18's "wait (blocking, indefinitely) for the qemu process to exit" must be implemented as a separate PID-exit wait, not inferred from the QMP response.
- **Checking `/proc/<pid>` existence alone for liveness:** a zombie (defunct, exited-but-unreaped) process still has a `/proc/<pid>` entry. Since spawned processes are reparented to PID 1 (or a subreaper) once `ezkvm start` exits, this is mostly moot for later `stop`/`kill`/`status` invocations (a different process, not the parent, is polling) — PID 1 reaps its orphaned children automatically. It matters only within a single `ezkvm` invocation's own lifetime (e.g. checking swtpm's PID right after spawning it, before `ezkvm` itself exits).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument/subcommand parsing | A manual `std::env::args()` matcher | `clap` (derive API) | Help text, `--config-dir` global flag inheritance across subcommands, and error messages for missing/invalid `<vm-name>` come for free; hand-rolled arg parsing is a common source of subtly wrong `--flag=value` vs `--flag value` handling |
| JSON serialization for QMP | Hand-written string formatting of QMP requests | `serde_json::json!` macro + `serde_json::to_string` | QMP argument objects can be arbitrarily nested (though this phase's commands — `system_powerdown`/`quit`/`system_reset` — take no arguments); string-formatting JSON by hand is exactly the kind of "reinvent a well-solved format" the project should avoid, and `serde_json` is already effectively free (the project already depends on `serde`) |
| Detecting whether a process is alive | Parsing `ps`/`pgrep` command output | `/proc/<pid>/stat` read or `libc::kill(pid, 0)` | Shelling out to `ps` adds a process-spawn round-trip and output-parsing fragility for information the kernel already exposes directly via `/proc` or the `kill(2)` syscall |

**Key insight:** The QMP *protocol framing* (JSON handshake + line-delimited request/response) is simple enough that hand-rolling a small client using `serde_json` + `std::os::unix::net::UnixStream` is the right level of "build it yourself" — the line not to cross is re-implementing JSON parsing itself, or re-implementing `clap`'s argument-grammar handling.

## Runtime State Inventory

> Omitted — this is a greenfield phase (new CLI verbs, new `host.yaml`, new `VmHandle` state file, new QMP client). It introduces new runtime state rather than renaming/migrating existing state. No existing production data, live services, OS-registered state, or secrets reference the concepts this phase introduces (`host.yaml`, VM state files) under their new names, because they don't exist yet.

## Common Pitfalls

### Pitfall 1: QMP Requires Capabilities Negotiation Before Any Real Command

**What goes wrong:** Sending `system_powerdown` (or any command) immediately after connecting, without first sending `qmp_capabilities`, gets rejected — QEMU starts in a "capabilities negotiation" mode where only `qmp_capabilities` itself is accepted; all other commands return a `CommandNotFound` error until negotiation succeeds.

**Why it happens:** The QMP spec models the connection as starting in a restricted mode specifically so clients can't accidentally issue commands before checking the server's advertised feature set.

**How to avoid:** Always: connect → read greeting line → send `{"execute": "qmp_capabilities"}` → read+check its `{"return": {}}` → only then send `system_powerdown`/`quit`/`system_reset`.

**Warning signs:** A QMP `error` response with `"class": "CommandNotFound"` on the very first real command sent.

### Pitfall 2: `-qmp` Flag Is Not Emitted by Phase 7's `QemuCommandLine`

**What goes wrong:** Assuming the rendered `QemuCommandLine::to_string()` (or its `Display` impl) already includes a QMP socket flag, because `QemuContext` already threads through `tpm_socket_path`.

**Why it happens:** `tpm_socket_path` is VM-schema-driven (the `TpmState` root device triggers `-chardev socket,...`/`-tpmdev emulator,...`); QMP is pure host-orchestration plumbing that has no corresponding `Runtime`/`DisplaySchema` device — it was correctly left out of Phase 7's scope.

**How to avoid:** Append `-qmp unix:<socket_path>,server,nowait` (classic spelling, still accepted by QEMU 10.0 confirmed on this host) or `-qmp unix:<socket_path>,server=on,wait=off` (modern spelling) to the string produced by `QemuCommandLine`'s `Display` impl, at the call site in the new lifecycle module — do not modify Phase 7's `QemuContext`/handlers to add a `qmp_socket_path` field unless the planner deliberately decides that's cleaner; a pure string-append avoids touching already-tested Phase 7 code.

**Warning signs:** `stop`/`kill`/`reset` fail to connect because no QMP socket file was ever created by qemu.

### Pitfall 3: `swtpm` Must Be Listening *Before* qemu's `-chardev socket` Connects

**What goes wrong:** Starting qemu before `swtpm` has created and bound its control socket causes qemu to fail immediately with a chardev connection error (qemu's `-chardev socket,path=...` without `server` acts as the *client* side, connecting to a path `swtpm` must already own as the *server*).

**Why it happens:** Confirmed directly against the existing Phase 7 handler (`src/config/qemu/handlers/root.rs`): the emitted flag is `-chardev socket,id=tpmchar,path=<socket_path>` with no `server`/`wait` sub-options — qemu is the client, `swtpm socket` (started with `--ctrl type=unixio,path=<socket_path>`) is the server.

**How to avoid:** D-14's poll-until-connectable loop (Pattern 3 above) targeting the same `tpm_socket_path` that will be passed into `QemuContext` — spawn `swtpm socket ...` first, poll, then spawn qemu only after the poll succeeds.

**Warning signs:** qemu process exits almost immediately with a `chardev: opening backend "socket" failed` (or similar) error when TPM is configured.

### Pitfall 4: `quit`'s Response May Never Arrive

**What goes wrong:** Treating "the socket read returned an I/O error / EOF" as a failure of the `quit` command.

**Why it happens:** Per the QMP spec (quoted verbatim): "While every attempt is made to send the QMP response before terminating, this is not guaranteed. When using this interface, a premature EOF would not be unexpected." QEMU may tear down the QMP socket before flushing the JSON response.

**How to avoid:** Treat a connection-closed/EOF condition while awaiting `quit`'s response as success, not error (see Pattern 2's `quit()` implementation, which maps `Err(QmpError::Io(_))` to `Ok(())`).

**Warning signs:** VMGR-04 ("force-stop") intermittently reports failure even though `ps`/`/proc` confirms qemu did in fact exit.

### Pitfall 5: Orphaned `swtpm` on qemu Launch Failure

**What goes wrong:** If `swtpm` starts successfully but the subsequent `qemu-system-x86_64` spawn fails (bad cmdline, missing binary, etc.), a naive implementation leaves `swtpm` running forever with no VM ever using its socket.

**Why it happens:** The two processes are spawned sequentially with no automatic cleanup coupling in `std::process::Command` — killing one on the other's failure is entirely the caller's responsibility.

**How to avoid:** D-17 requires this explicitly — wrap the qemu spawn in a result check; on failure, send `SIGTERM` (or `SIGKILL`) to the already-recorded `swtpm` PID and delete the partial state file before returning the error to the CLI caller.

**Warning signs:** `ps aux | grep swtpm` shows swtpm processes with no matching `qemu-system-x86_64` process for the same VM.

### Pitfall 6: Zombie Detection Only Matters for Same-Process PID Checks

**What goes wrong:** Over-engineering zombie-state handling into every `is_pid_alive` call site, when it's only relevant during the single `ezkvm start` invocation's own lifetime (before it exits).

**Why it happens:** Because `ezkvm start` is fire-and-forget (D-16), the qemu/swtpm/UI-client processes get reparented to PID 1 (or a subreaper like systemd, if running under one) as soon as `ezkvm start` exits — PID 1 automatically reaps zombies. Later `stop`/`kill`/`reset`/`status` invocations are entirely separate processes that are never the parent of the tracked PIDs, so from their point of view a zombie state is nearly impossible to observe (PID 1 reaps almost immediately).

**How to avoid:** Use the simple `/proc/<pid>` existence check (or `libc::kill(pid, 0)`) for `stop`/`kill`/`reset`/`status`'s PID-liveness checks (D-09); reserve the more careful zombie-aware check (Pattern 4's `/proc/<pid>/stat` variant) specifically for the D-17 in-process check of `swtpm`'s own PID right after spawning it, within the same `ezkvm start` invocation that spawned it (where `ezkvm` genuinely is the parent and zombies can accumulate if not reaped).

## Code Examples

See **Architecture Patterns** section above (Patterns 1-4) — all code examples are embedded there with source citations, since each pattern's code IS the primary example for that concern (detached spawn, QMP client, readiness polling, PID liveness).

### UI Client Command-Line Syntax (VMGR-02, D-11)

```bash
# remote-viewer (virt-viewer package) — SPICE or VNC.
# Source: virt-viewer man/remote-viewer.pod (fetched from
# gitlab.com/virt-viewer/virt-viewer, 2026-07-28). Synopsis: `remote-viewer [OPTIONS] -- [URI]`
remote-viewer spice://<host>:<port>
remote-viewer vnc://<host>:<port>
# Unix-socket SPICE connections are supported since virt-viewer 8.0 via the
# `unix-path` connection-file key, but there is no plain CLI URI form documented
# for a bare unix-socket SPICE target — if ezkvm's Spice display uses TCP
# (port + listen, matching `SpiceSchema`'s existing fields), the `spice://host:port`
# form above is what applies directly.

# looking-glass-client — connects to an ivshmem-backed shared memory device.
# Source: looking-glass.io/docs/B7/usage (fetched 2026-07-28).
looking-glass-client -f /dev/shm/<shm-name>
# Long form equivalent:
looking-glass-client app:shmFile=/dev/shm/<shm-name>
# The shm path is NOT a field on `LookingGlassSchema` (which only has port/pulisten/
# disable_ticketing — SPICE-channel fields for input/clipboard/audio alongside LG's
# own video path) — it comes from the VM's `Ivshmem` Runtime device's `mem_path` field
# (src/runtime/devices/ivshmem.rs). The UI-client-launch code must look up the
# Ivshmem root device on the loaded Runtime, not just the DisplaySchema, to build the
# looking-glass-client invocation.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| HMP (Human Monitor Protocol) text commands | QMP (JSON-based) | QMP has been the recommended machine-facing interface since QEMU ~0.14 (2011) | This phase should use QMP exclusively, never HMP — HMP is documented as intended for interactive human use, and QMP is the interface libvirt/Proxmox/virt-manager all script against |
| `-qmp unix:<path>,server,nowait` | `-qmp unix:<path>,server=on,wait=off` | The `nowait`/bare-`server` boolean-flag shorthand syntax is legacy; modern QEMU (7.x+) also accepts the explicit `server=on,wait=off` form | Both forms work on the QEMU 10.0.2 build present on this host (`pve-qemu-kvm_10.0.2-4`) `[VERIFIED: qemu-system-x86_64 present on this host, version confirmed via --version]`; either is acceptable, but the explicit `server=on,wait=off` form is more future-proof/self-documenting and is recommended for new code |

**Deprecated/outdated:**
- HMP for anything programmatic — still exists for interactive `telnet`/`-monitor stdio` use, but this phase's `stop`/`kill`/`reset` must use QMP, matching D-18/D-19/D-21's explicit "via QMP" wording and VMGR-03/04/05's requirement text.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `/proc/<pid>/stat`'s field layout (`pid (comm) state ...`) and the zombie-state character `Z` are treated as a stable, well-known Linux kernel ABI, not fetched from a specific doc page this session | Pattern 4, Pitfall 6 | Low — this is extremely stable, decades-old kernel ABI; if wrong, the fallback is simply to add `libc` and use `kill(pid, 0)` instead, which needs no `/proc` parsing at all |
| A2 | `remote-viewer` has no documented plain-CLI URI scheme for connecting to a Unix-socket SPICE target (only the `unix-path` key inside a connection-settings *file*, not a bare `spice+unix://` URI) | Code Examples §UI Client Command-Line Syntax | Medium — if the planner decides ezkvm's Spice display should default to a Unix socket instead of TCP host:port, `remote-viewer`'s exact invocation for that case needs to be re-verified against a newer virt-viewer release or its `--help` output at implementation time; `SpiceSchema`'s existing fields (`port`/`listen`) suggest TCP is the intended transport already, which sidesteps this |
| A3 | The `-qmp unix:<path>,server,nowait` classic syntax remains accepted (not just the modern `server=on,wait=off` form) in current QEMU — confirmed only by cross-referencing this host's installed QEMU 10.0.2 binary's general acceptance of legacy chardev syntax, not by executing an actual `-qmp` smoke test in this research session | State of the Art, Pitfall 2 | Low — both syntaxes are recommended as acceptable in this research; the planner/executor should include an actual smoke test (spawn qemu with `-qmp`, confirm socket file appears and `qmp_capabilities` succeeds) as part of this phase's Nyquist validation regardless of which syntax is chosen |

## Open Questions (RESOLVED)

1. **Exact `host.yaml` directory key names (tmp/pid/socket dirs)**

   - What we know: `vm_dir` is locked (D-04). The user explicitly left other directory key names to the agent's discretion.
   - What's unclear: Whether to use one shared `state_dir` for both the PID/state file (D-07) and QMP/swtpm Unix sockets, or separate `pid_dir` and `socket_dir` keys.
   - Recommendation: A single `state_dir` (e.g. `<config_dir>/state` or `/run/ezkvm`) holding both `<vm-name>.state` (the `VmHandle` YAML) and `<vm-name>.qmp.sock`/`<vm-name>.tpm.sock` keeps related per-VM runtime artifacts colocated and simplifies cleanup (one directory to check/clear on `stop`), and mirrors the common Linux convention of using `/run/<app>/` for exactly this kind of ephemeral runtime state. The planner should confirm this shape with the user if there's any ambiguity, since it's flagged as "discretion" not "locked."
   - **RESOLVED:** `08-01-PLAN.md` adopted the single shared `state_dir` recommendation as-is — one `HostConfig.state_dir` field (default `/run/ezkvm`) holds the `VmHandle` `.state` file, the QMP socket, and (from `08-02`) the swtpm control socket, all created together under the `0700` permission mitigation in `process::ensure_dir_secure`.

2. **Auto-escalation timeout value for `stop → kill` (D-19)**

   - What we know: The feature is a configurable override, off by default, discretion on the exact timeout value.
   - What's unclear: No specific number was given by the user or found in the roadmap.
   - Recommendation: A reasonable default if/when the override is enabled would be in the 30-90 second range (long enough for a graceful Windows/Linux guest shutdown, short enough to not hang indefinitely when auto-escalation is explicitly opted into) — but since this feature is off by default, the exact number matters less than ensuring the *config knob* exists and is respected.
   - **RESOLVED:** `08-03-PLAN.md` implements this as a pure opt-in config knob with **no hardcoded default value** — `HostConfig.stop_escalation_timeout_secs: Option<u64>` is `None` unless the operator sets it in `host.yaml`, and a per-invocation `--escalate-after <secs>` CLI flag overrides it. The 30-90s range from this recommendation is left as *operator guidance* (documented, not enforced in code), since D-19 only requires the mechanism to exist and default to "off," not any particular number when turned on.

3. **swtpm readiness timeout duration (D-14) and UI-client fixed delay duration (D-13)**

   - What we know: Both are explicitly at the agent's discretion per CONTEXT.md.
   - What's unclear: Exact seconds.
   - Recommendation: swtpm readiness timeout: 5-10 seconds (swtpm startup is near-instant once the binary is invoked; a generous timeout mainly guards against a misconfigured/missing binary hanging the whole `start` flow). UI-client fixed delay: 1-2 seconds as the user's own CONTEXT.md phrasing already suggests ("~1-2s") — treat this as effectively locked despite being filed under "discretion," since the user supplied the exact range themselves.
   - **RESOLVED:** `08-02-PLAN.md` adopted concrete values from this recommendation's range: an **8-second** swtpm readiness timeout (`readiness::wait_for_socket(tpm_socket_path, Duration::from_secs(8))`), and a **~2-second** fixed delay before the UI-client launch (`std::thread::sleep(Duration::from_secs(2))`), matching the user's own "~1-2s" phrasing in `08-CONTEXT.md`.


## Environment Availability (Historical / Informational Only)

> **Note:** This table records what real binaries were confirmed installed on the researcher's dev host at research time — it is **historical/informational only** and is **not a testing prerequisite** for Phase 8. Every Phase 8 test uses the pure-Rust stub binaries `src/bin/fake_qemu.rs`, `src/bin/fake_swtpm.rs`, and `src/bin/fake_ui_client.rs` (see the plans' Test Independence notes) instead of any of the real binaries listed below, so the phase's test suite requires none of them to be installed and passes identically on any Linux distribution with the same Rust toolchain. Real-binary, real-distro verification is deferred to **Phase 10 — "Deployment Packaging - Debian/Ubuntu"**.

| Dependency | Required By (production) | Available on researcher's dev host | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `qemu-system-x86_64` | VMGR-01 (spawn qemu) | ✓ | 10.0.2 (`pve-qemu-kvm_10.0.2-4`) | Phase 8 tests use `src/bin/fake_qemu.rs` instead |
| `swtpm` | VMGR-01 (TPM-configured VMs) | ✓ | 0.8.0 | Phase 8 tests use `src/bin/fake_swtpm.rs` instead |
| `remote-viewer` | VMGR-02 (SPICE/VNC UI client) | ✓ | present (`/usr/bin/remote-viewer`) | Phase 8 tests use `src/bin/fake_ui_client.rs` instead |
| `looking-glass-client` | VMGR-02 (Looking Glass UI client) | ✗ | — | Not installed on this dev host — irrelevant for Phase 8 since tests use `src/bin/fake_ui_client.rs`; real Looking Glass verification happens in Phase 10 |
| `setsid` (util-linux binary) | Not required — this phase uses `pre_exec`+`libc::setsid()`, not the external `setsid` binary | ✓ (present, unused) | — | N/A — listed only because it was probed; the recommended implementation calls the `setsid()` syscall directly via `libc`, so the external binary's presence/absence is irrelevant |
| `clap` (Cargo dependency) | CLI parsing | ✗ (not yet added) | latest 4.6.4 | `cargo add clap --features derive` |
| `serde_json` (Cargo dependency) | QMP JSON | ✗ (not yet added) | latest 1.0.151 | `cargo add serde_json` |
| `libc` (Cargo dependency) | `setsid`/`kill(pid,0)` FFI | ✗ (not yet added) | latest 0.2.189 | `cargo add libc` |

**Missing dependencies with no fallback:** none — all missing items have a documented fallback or are Cargo deps trivially added by the planner's first task.

**Test-time dependencies:** none of the four real binaries above (`qemu-system-x86_64`, `swtpm`, `remote-viewer`, `looking-glass-client`) are required to run Phase 8's test suite — every test uses `fake_qemu`/`fake_swtpm`/`fake_ui_client` (Cargo `[[bin]]` targets under `src/bin/`, referenced via `env!("CARGO_BIN_EXE_<name>")`) instead.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust's built-in `cargo test` (existing convention — no `#[test]` framework crate beyond `std`) |
| Config file | none — see Wave 0 gaps below |
| Quick run command | `cargo test --test <new_test_file> -- --test-threads=1` (process/socket tests should be serialized to avoid port/socket-path collisions) |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|--------------|
| VMGR-01 | `ezkvm start` launches qemu (+swtpm if TPM configured) without error | integration | `cargo test --test vm_lifecycle -- test_start_launches_qemu_and_swtpm` | ❌ Wave 0 |
| VMGR-01 | swtpm readiness poll blocks qemu spawn until socket connectable, times out cleanly if swtpm never starts | integration | `cargo test --test vm_lifecycle -- test_swtpm_readiness_timeout` | ❌ Wave 0 |
| VMGR-01 | `ezkvm start` on an already-running VM fails with a clear error (D-10) | integration | `cargo test --test vm_lifecycle -- test_start_already_running_fails` | ❌ Wave 0 |
| VMGR-01 | qemu spawn failure after swtpm success kills the orphaned swtpm and cleans up state (D-17) | integration | `cargo test --test vm_lifecycle -- test_qemu_failure_kills_orphaned_swtpm` | ❌ Wave 0 |
| VMGR-02 | UI client launch mapping picks the right binary per `DisplaySchema` variant (D-11/D-12) | unit | `cargo test --test ui_client_mapping -- test_display_to_client_mapping` | ❌ Wave 0 |
| VMGR-02 | UI client launch failure (binary missing) logs a warning, VM keeps running (D-15) | integration | `cargo test --test vm_lifecycle -- test_ui_client_launch_failure_is_nonfatal` | ❌ Wave 0 |
| VMGR-03 | `stop` sends `system_powerdown`, blocks until qemu PID exits (D-18) | integration | `cargo test --test qmp_client -- test_stop_sends_powerdown_and_waits` | ❌ Wave 0 |
| VMGR-04 | `kill` sends `quit`, tolerates EOF-without-response (Pitfall 4) | unit | `cargo test --test qmp_client -- test_quit_tolerates_premature_eof` | ❌ Wave 0 |
| VMGR-05 | `reset` sends `system_reset` without awaiting a response (D-21) | unit | `cargo test --test qmp_client -- test_reset_is_fire_and_forget` | ❌ Wave 0 |
| VMGR-03/04 | `stop`/`kill` success also terminates the tracked UI client PID (D-20) | integration | `cargo test --test vm_lifecycle -- test_stop_also_kills_ui_client` | ❌ Wave 0 |
| — | QMP capabilities handshake required before other commands succeed (Pitfall 1) | unit | `cargo test --test qmp_client -- test_capabilities_handshake_required` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** targeted `cargo test --test <file> -- <test_name>`
- **Per wave merge:** `cargo test` (full suite)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `tests/vm_lifecycle.rs` — new integration test file covering VMGR-01/02/03/04's start/stop/kill/orphan-cleanup behaviors against this phase's `fake_qemu`/`fake_swtpm` stub binaries (see Environment Availability — no real binary installation required)
- [ ] `tests/qmp_client.rs` — new unit/integration test file for the QMP client's handshake, `system_powerdown`/`quit`/`system_reset` framing, and the EOF-tolerance behavior (Pitfall 4) — this can be tested against a minimal fake QMP server (a test-local `UnixListener` that speaks just enough JSON to exercise the client) rather than a real qemu process, keeping it a fast unit-style test
- [ ] `tests/ui_client_mapping.rs` — new unit test file for the pure `DisplaySchema` (+ `Ivshmem` Runtime device, for Looking Glass) → binary/args mapping function, independent of actually spawning a process
- [ ] Framework install: none — `cargo test` is already the project's test runner; no new test framework crate is needed, only new test *files*

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|----------------|---------|-------------------|
| V2 Authentication | no | ezkvm is a local, single-host CLI tool run by a trusted operator; no network-facing auth surface is introduced by this phase |
| V3 Session Management | no | No web/API session concept in this phase |
| V4 Access Control | partial | Unix socket file permissions (QMP socket, swtpm control socket) should be created with restrictive mode bits (owner-only, `0700`/`0600`-equivalent directory/socket perms) so other local users on a multi-user host cannot connect to a running VM's monitor and issue `quit`/`system_reset` against it. QEMU's `-qmp unix:<path>,server=on,wait=off` creates the socket with the process's default umask — the state/socket directory itself should be created with restrictive permissions (e.g. `0700`) by ezkvm before qemu/swtpm are spawned into it |
| V5 Input Validation | yes | `<vm-name>` CLI argument must be validated/sanitized before being used to build a filesystem path (`<vm_dir>/<vm-name>.yaml`) — reject path-traversal sequences (`../`, absolute paths, embedded `/`) to prevent a malicious/typo'd `<vm-name>` from escaping the configured VM directory. `host.yaml`/VM YAML parsing already goes through the existing, presumably-hardened `crate::serde_yaml` pipeline |
| V6 Cryptography | no | No cryptographic operations introduced by this phase (QMP over a local Unix socket has no TLS layer, matching how libvirt/qemu itself treats local monitor sockets — protection comes from filesystem permissions, not transport encryption) |

### Known Threat Patterns for This Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Path traversal via `<vm-name>` argument (e.g. `ezkvm start ../../etc/passwd`) | Tampering / Information Disclosure | Validate `<vm-name>` contains no path separators or `..` components before joining with `vm_dir`; reject with a clear CLI error rather than silently normalizing |
| World-readable/connectable QMP or swtpm control socket allowing another local user to send `quit`/`system_reset` to someone else's VM | Elevation of Privilege / Denial of Service | Create the state/socket directory with `0700` permissions before spawning qemu/swtpm into it; rely on standard Unix socket filesystem permission enforcement (no additional QMP-level auth exists to lean on) |
| Symlink attack on the state file path (`<pid_dir>/<vm-name>.state`) | Tampering | Open the state file with `O_NOFOLLOW`-equivalent care (e.g. via `std::fs::File::options().create_new(true)` on first write, or checking `symlink_metadata` before trusting an existing path) if the state directory could ever be group/world-writable — low risk given the `0700` mitigation above already restricts write access, but worth a defensive check given this file's content (PIDs) is used to decide whether a `kill`/`quit` command is issued against a live process |

## Sources

### Primary (HIGH confidence)

- crates.io registry API (`https://crates.io/api/v1/crates/{clap,libc,nix,serde_json}`) — verified current versions, download counts, and repository URLs, cross-checked against `gsd-tools query package-legitimacy check`, 2026-07-28
- Existing codebase: `src/config/qemu.rs`, `src/config/qemu/handlers/root.rs`, `src/config/qemu/builder.rs` — confirmed `QemuContext`'s exact fields, the exact `-chardev socket,id=tpmchar,path=...`/`-tpmdev emulator,...` flags emitted for TPM, and the absence of any `-qmp` emission
- Existing codebase: `src/config/ezkvm/schema/display.rs`, `src/config/ezkvm/schema/resources.rs`, `src/runtime/devices/ivshmem.rs` — confirmed `DisplaySchema`/`LookingGlassSchema`/`SpiceSchema`/`VncSchema` fields and the `Ivshmem` Runtime device's `mem_path` field
- Local shell (`swtpm socket --help`, `qemu-system-x86_64 --version`, `command -v`) — confirmed `swtpm socket --ctrl type=unixio,path=...` syntax and installed tool versions on this host, 2026-07-28

### Secondary (MEDIUM confidence)

- QEMU QMP protocol spec — `https://www.qemu.org/docs/master/interop/qmp-spec.html` (fetched 2026-07-28) — server greeting format, `qmp_capabilities` negotiation requirement, request/response/error JSON shapes `[CITED: qemu.org/docs/master/interop/qmp-spec.html]`
- QEMU QMP command reference — `https://qemu-project.gitlab.io/qemu/interop/qemu-qmp-ref.html` (fetched 2026-07-28) — exact `system_powerdown`/`system_reset`/`quit` semantics and examples `[CITED: qemu-project.gitlab.io/qemu/interop/qemu-qmp-ref.html]`
- virt-viewer `remote-viewer` man page source — `https://gitlab.com/virt-viewer/virt-viewer/-/raw/master/man/remote-viewer.pod` (fetched 2026-07-28) — SYNOPSIS, URI examples (`spice://host:port`, `vnc://host:port`), `unix-path` connection-file key `[CITED: gitlab.com/virt-viewer/virt-viewer]`
- Looking Glass client usage docs — `https://looking-glass.io/docs/B7/usage/` (fetched 2026-07-28) — `looking-glass-client -f <shmFile>` / `app:shmFile=<path>` syntax `[CITED: looking-glass.io/docs/B7/usage]`
- Rust std docs — `https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html` (fetched 2026-07-28) — confirmed `pre_exec` (unsafe, stable), `process_group` (stable since 1.64.0), and `setsid` (nightly-only, tracking issue rust-lang/rust#105376) `[CITED: doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html]`

### Tertiary (LOW confidence)

- `/proc/<pid>/stat` field layout and zombie-state semantics — standard, extremely stable Linux kernel ABI knowledge, not fetched from a specific `proc(5)` man page this session `[ASSUMED — see Assumptions Log A1]`
- POSIX `kill(pid, 0)` liveness-check semantics (`ESRCH` vs `EPERM` vs success) — well-established POSIX behavior, not fetched from `man7.org` this session despite being cited inline in the code example `[ASSUMED — treat the inline citation as a pointer to further reading, not a session-verified fetch]`

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all four candidate crates verified live against crates.io and the project's own package-legitimacy gate; no `[SUS]`/`[SLOP]` findings
- Architecture: HIGH — cross-checked against actual project source (`qemu.rs`, `display.rs`, `resources.rs`, `ivshmem.rs`) rather than assumed from the CONTEXT.md description alone; the "Phase 7 emits no `-qmp` flag" finding in particular came from direct code inspection, not assumption
- Pitfalls: HIGH for QMP-protocol pitfalls (sourced from the official QEMU spec/reference fetched this session); MEDIUM for the `/proc`-based zombie-detection pitfall (well-established but not freshly re-verified against a `proc(5)` man page this session)

**Research date:** 2026-07-28
**Valid until:** 2026-08-27 (30 days — QMP protocol and Rust std APIs are stable/slow-moving; re-verify crate versions if this research is reused after that window)
