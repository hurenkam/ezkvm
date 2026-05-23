# ADR-0003: Hooks Policy

**Date:** 2026-04-15  
**Status:** Accepted  
**Context:** ezkvm Incremental Convergence Initiative - Flexible Lifecycle Hooks

## Question

How should VM lifecycle hooks (pre/post start/stop) be designed to enable device-specific setup while maintaining deterministic behavior and observability?

## Decision

**Hooks are optional, deterministic, shell-based, and execute in strict order with explicit timeout and error policies.**

### Hook Design

1. **Hook Points:**
   - `pre_start` - Before QEMU process launch
   - `post_start` - After QEMU process confirms running
   - `pre_stop` - Before shutdown signal sent
   - `post_stop` - After QEMU process exits

2. **Hook Invocation:**
   - Serial execution (not parallel)
   - Strict ordering: pre_start → launch → post_start → normal operation → pre_stop → stop → post_stop
   - Native shell invocation with full environment

3. **Timeout Policy:**
   - Default: 30 seconds per hook
   - Configurable: `timeout: 60` in hook config
   - Exceeded timeout → force terminate hook process
   - Pre-start/pre-stop timeout: **fail-closed** (block start/stop)
   - Post-start/post-stop timeout: **fail-open** (log warning, continue)

4. **Error Policy:**
   - Non-zero exit codes treated as failures
   - Pre-start/pre-stop failure: **blocks operation** (fail-closed)
   - Post-start/post-stop failure: **logged, operation continues** (fail-open)
   - No automatic retry

5. **Environment:**
   - `EZKVM_VM_NAME` - VM name
   - `EZKVM_VM_PID` - QEMU process ID (post-start/post-stop only)
   - `EZKVM_HOOK_POINT` - Current hook phase
   - `EZKVM_DRY_RUN` - Set to "1" if running in --dry-run mode
   - Full user environment inherited

## Configuration Schema

```yaml
options:
  hooks:
    pre_start:
      command: "/etc/ezkvm/hooks/setup-network.sh"
      timeout: 30
      on_error: fail_closed
    post_start:
      command: "/opt/ezkvm/post-start.sh"
      timeout: 10
      on_error: fail_open
    pre_stop:
      command: "/etc/ezkvm/hooks/cleanup.sh"
      timeout: 30
      on_error: fail_closed
    post_stop:
      command: "/var/lib/ezkvm/post-stop.sh"
      timeout: 5
      on_error: fail_open
```

## Logging and Observability

**Log Format (structured):**
```
[HOOK] {timestamp} {vm_name} {hook_point} START
[HOOK] {timestamp} {vm_name} {hook_point} EXEC {command}
[HOOK] {timestamp} {vm_name} {hook_point} STDOUT {line}
[HOOK] {timestamp} {vm_name} {hook_point} STDERR {line}
[HOOK] {timestamp} {vm_name} {hook_point} EXIT {code} {elapsed_ms}
[HOOK] {timestamp} {vm_name} {hook_point} TIMEOUT {elapsed_ms} terminated PID {pid}
```

**Log Location:** `~/.ezkvm/logs/{vm_name}-{session}.log`

## Dry-Run Behavior

- Hooks execute (not skipped)
- All stdout/stderr captured and printed
- Exit codes validated
- Non-zero codes reported but do not block progression (for testing)
- `$EZKVM_DRY_RUN=1` allows hook authors to detect and skip destructive operations

## Implementation Layers

**Config Layer:**
- Parse `options.hooks.*` from YAML
- Validate command is executable (fail early if not found)
- Store timeout and error policy

**Runtime Layer:**
- Hook runner service executes hooks in order
- Timeout enforcement via process group termination
- Signal propogation to child processes on VM stop

**Shell Interface:**
- No hook DSL; standard shell scripts
- Access to environment variables only (no SDK library)
- Script authors responsible for atomicity and idempotency

## Consequences

**Positive:**
- Shell-based hooks enable maximal flexibility
- Fail-open/fail-closed policies make failure modes explicit
- Timeout enforcement prevents hung processes
- Dry-run support enables testing
- Structured logs enable automation/monitoring
- Deterministic ordering prevents race conditions

**Negative:**
- Script failures not automatically retried
- Shell environment dependency (portability risk)
- Timeout configured statically (not adaptive)
- Limited built-in recovery (user must script around hook failures)

## Alternatives Considered

### Alternative 1: Rust Plugin System
- Rejected: complexity, security (loading untrusted .so files), slower iteration
- Decision: shells scripts are accessible to operators

### Alternative 2: Hook Chains (multiple hooks per point)
- Rejected: increases complexity without clear use case
- Decision: use single hook script that can delegate internally

### Alternative 3: Async Hooks
- Rejected: complicates concurrent execution, state tracking
- Decision: serial atomic execution is easier to reason about

## Related ADRs
- [ADR-0001: Base Selection](ADR-0001-base-selection.md)
- [ADR-0004: Trait Seam Policy](ADR-0004-trait-seam-policy.md)

## References

- Backlog task: `doc/backlog/BACKLOG.md` - C-01 through C-05
- v1 Lifecycle Hooks: `/home/hurenkam/Workspace/ezkvm_v1/src/vm/virtual_machine.rs` (trait methods)
- Current layers: `doc/dev/architecture/architecture-guidelines.md`
