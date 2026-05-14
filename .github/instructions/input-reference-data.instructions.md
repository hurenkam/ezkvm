---
description: "Use when editing Proxmox importer code, compact-mode config serialization, runtime QEMU/swtpm argument generation, or host-specific config overlays in input/**/etc/. Enforces immutability of captured reference files and requires explicit user consent before regenerating derived files."
applyTo: "src/**/*.rs, input/**/etc/**, etc/**/*.yaml"
---

# Input Reference Data Rules

The `input/` directory contains captured data from real Proxmox hosts. These files serve as regression-detection references for the Proxmox importer, compact-mode serialization, and QEMU/swtpm command generation.

## Directory Structure

```
input/
  <host>/                               # One subdirectory per Proxmox host
    <vmid>.conf                         # Proxmox VM config — IMMUTABLE
    <vmid>.qemu.cmd                     # Proxmox-generated QEMU command — IMMUTABLE
    <vmid>.qemu.cmd.split               # Line-split copy of above — IMMUTABLE
    <vmid>.swtpm.cmd                    # Proxmox-generated swtpm command — IMMUTABLE (when present)
    <vmid>.yaml                         # ezkvm compact config (import output) — DERIVED, see below
    <vmid>.resolved-config.yaml         # Resolved ezkvm config (validate output) — DERIVED, see below
    <vmid>.ezkvm.qemu.cmd               # ezkvm-generated QEMU command — DERIVED, see below
    <vmid>.ezkvm.swtpm.cmd              # ezkvm-generated swtpm command — DERIVED, see below
    etc/
      ezkvm/
        ezkvm.yaml                      # Host-specific global config overlay — version-controlled
        profiles.d/
          *.yaml                        # Host-specific profile overrides — version-controlled
```

## Immutable Reference Files

**NEVER edit, regenerate, or reformat these files under any circumstance:**

- `input/<host>/<vmid>.conf`
- `input/<host>/<vmid>.qemu.cmd`
- `input/<host>/<vmid>.qemu.cmd.split`
- `input/<host>/<vmid>.swtpm.cmd`

These are verbatim captures from live Proxmox hosts. Any change would silently corrupt the regression baseline.

## Derived Files — When to Check for Updates

Derived files (`*.yaml`, `*.resolved-config.yaml`, `*.ezkvm.qemu.cmd`, `*.ezkvm.swtpm.cmd`) may become stale when the code or config they depend on changes.

### `<vmid>.yaml` and `<vmid>.resolved-config.yaml` may need regeneration when:

| Trigger | Affected hosts/VMs |
|---|---|
| Proxmox importer code changes (`src/import/proxmox/**`) | All hosts with `.yaml` files |
| Compact-mode serialization changes (`src/config/**`, `src/qemu/**`) | All hosts with `.yaml` files |
| Host-specific config overlay changes (`input/<host>/etc/ezkvm/ezkvm.yaml` or `input/<host>/etc/ezkvm/profiles.d/*.yaml`) | Only the affected host's VMs |
| Global profile changes (`etc/profiles.d/*.yaml`) that affect imported VMs | All hosts with `.yaml` files |

### `<vmid>.ezkvm.qemu.cmd` and `<vmid>.ezkvm.swtpm.cmd` may need regeneration when:

| Trigger | Affected hosts/VMs |
|---|---|
| Runtime QEMU argument generation changes (`src/qemu/**`) | All hosts with `.ezkvm.qemu.cmd` files |
| swtpm command generation changes (`src/**`) | All hosts with `.ezkvm.swtpm.cmd` files |
| Host-specific config overlay changes (`input/<host>/etc/**`) | Only the affected host's VMs |
| The corresponding `<vmid>.yaml` is updated | Only the VMs whose `.yaml` changed |

## Consent Requirement

**Before regenerating any derived file in `input/`:**

1. Stop and inform the user which files need updating and why (which code or config change triggered the check).
2. Describe what the regenerated content would be (which command will be run).
3. **Wait for explicit user approval** before running any regeneration command.

Do not regenerate files silently as a side-effect of a code change task.

## Regeneration Commands (Reference)

Run these from the workspace root after obtaining user consent. Substitute `<host>`, `<vmid>`, and the host's config path as appropriate.

```bash
# Regenerate <vmid>.yaml from the corresponding .conf using host-specific config overlay
ezkvm import-proxmox input/<host>/<vmid>.conf \
  --config input/<host>/etc/ezkvm/ezkvm.yaml \
  --output input/<host>/<vmid>.yaml

# Regenerate <vmid>.ezkvm.qemu.cmd (dry-run, no VM started)
ezkvm validate input/<host>/<vmid>.yaml --show-resolved-config \
  > input/<host>/<vmid>.resolved-config.yaml
ezkvm run input/<host>/<vmid>.yaml --dry-run \
  > input/<host>/<vmid>.ezkvm.qemu.cmd
```

Verify the regenerated output against the corresponding Proxmox reference (`.qemu.cmd`) to detect regressions before committing.

## Regression Detection

When derived files are regenerated, diff them against the previous committed version:

```bash
git diff input/<host>/<vmid>.ezkvm.qemu.cmd
```

Unexpected diffs are regressions and must be understood and explained before the updated file is committed. If a diff is intentional (e.g. a deliberate topology change), document the reason in the commit message.
