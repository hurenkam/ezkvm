# QEMU Command Import Guide

This guide documents the standalone `import-qemu-cmd` workflow.

Use this mode when you have a captured QEMU command file (for example `input/<host>/<vmid>.qemu.cmd`) and want an ezkvm YAML starting point.

This workflow is independent from Proxmox config import. For `.conf` source files, use [proxmox.md](proxmox.md).

## Runtime Target Selection

`import-qemu-cmd` supports the same runtime-target flag family as Proxmox import:

| Target | Use when | Behavior |
| --- | --- | --- |
| `portable-linux` (default) | You want host-independent YAML as the primary output | Omits Proxmox-specific netdev helper paths (`script`, `downscript`, `helper`) during mapping so imported network intent remains portable. |
| `proxmox-parity` | You need strict source parity for migration/validation comparisons | Preserves source Proxmox netdev helper path fields in mapped YAML. |

Examples:

```bash
# default portable behavior
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --runtime-target portable-linux --dry-run

# parity-preserving behavior
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --runtime-target proxmox-parity --dry-run
```

## Separation Contract

`import-qemu-cmd` is intentionally separate from `import-proxmox`:

- Input contract is a QEMU command capture file, not Proxmox VM config.
- Parsing and mapping are implemented under `src/import/qemu_cmd/`.
- Shared behavior is reused only through importer-common helpers.
- Warning taxonomy is qemu-cmd specific.

## First-Release Scope

Current qemu-cmd mapping focuses on core representable options:

- VM identity and system basics:
  - `-name`
  - `-machine`
  - `-cpu`
  - `-m`
  - `-smp`
- Network pairing:
  - `-netdev`
  - `-device ... netdev=<id>`
- Host PCI placement:
  - `-device vfio-pci,...`
- Storage/controller placement:
  - controller extraction from `-device` (for example `virtio-scsi-pci`, `pvscsi`, `ahci`)
  - drive attachment mapping from `-device` plus `-drive`/`-blockdev` source nodes
- SPICE basics:
  - `-spice`

Unsupported or ambiguous option families are not silently dropped. They are surfaced as structured warnings in import output.

When source command placement fields are representable, importer output preserves them (for example host PCI `bus/addr` and drive/controller attachment fields).

For `portable-linux`, importer output may intentionally omit Proxmox-only network helper script paths while preserving bridge and device intent.

Q35 host PCI placement now follows the same shared placement planner used by Proxmox import. For overlapping source semantics, Proxmox and qemu-cmd imports converge on equivalent Q35 root-port assignment behavior.

## Placement Conflict Validation and Precedence

Before import output is rendered, ezkvm validates merged effective placement for conflicts.

Current conflict checks include:

- PCI placement collisions (`bus` + `addr`) for explicitly placed devices.
- Drive attachment collisions (`bus` + `unit`, and `bus` + `scsi_id`).

When a conflict is detected, import fails before export with deterministic diagnostics listing conflicting assignment owners.

Placement precedence contract is:

1. Explicit placement from source/config
2. Profile defaults
3. Normalization fallback

Conflicts are rejected; they are not auto-normalized away.

## CLI Usage

Basic dry run:

```bash
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --dry-run
```

Write output file:

```bash
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --output 108.yaml
```

Fail on warnings:

```bash
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --strict --dry-run
```

Output modes:

```bash
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --output-mode canonical --dry-run
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --output-mode compact --dry-run
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --output-mode debug --dry-run
```

Runtime-target variants:

```bash
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --runtime-target portable-linux --dry-run
ezkvm import-qemu-cmd input/felucia/108.qemu.cmd --runtime-target proxmox-parity --dry-run
```

## Output Modes

| Mode | Behavior |
| --- | --- |
| `canonical` | Emit canonical YAML without debug preamble comments. |
| `compact` | Current behavior matches canonical for qemu-cmd importer. |
| `debug` | Emit canonical YAML with deterministic source/warning comment preamble. |

`debug` mode is useful for reviewing how warnings relate to source options.

Compact-mode contract for portable target:

- Compact output is replay-safe and deterministic across repeated imports of the same input.
- Round-tripping compact YAML through parse and command-build preserves deterministic runtime args.
- Portable compact output does not persist Proxmox host runtime literals (for example `/var/run/qemu-server/*` and `/usr/libexec/qemu-server/*`).

Host-specific path resolution remains a runtime concern.

## Warning Model

Warnings include `source_field` and message text. Warning categories currently include:

- `UnsupportedFlag`: option flag recognized but not mapped in current scope.
- `UnsupportedValue`: flag is handled but value shape/content is not supported.
- `AmbiguousPairing`: relationships such as `-device` to `-netdev` cannot be resolved safely.

Dry-run output prints warnings before YAML. Non-dry-run output prints warnings in the completion summary.

## Post-Import Validation Checklist

After importing qemu-cmd YAML:

1. Validate the generated config:

```bash
ezkvm validate 108.yaml
```

2. Inspect generated runtime arguments:

```bash
ezkvm start 108.yaml --dry-run
```

3. Review and resolve import warnings:
- map unsupported device options into schema fields where available,
- keep intentional raw behavior via supported option fields,
- rerun import in `--strict` mode once warning debt is resolved.

4. For host-dependent resources, verify paths and permissions:
- firmware/OVMF paths,
- socket and runtime directories,
- device passthrough references,
- bridge-helper/network prerequisites.

## Troubleshooting

If strict mode fails immediately:

- rerun without `--strict` and inspect printed warnings,
- fix mapped fields incrementally,
- re-run with `--strict` to confirm warning-free import.

If generated YAML validates but runtime dry-run fails:

- compare with [../../config/troubleshooting.md](../../config/troubleshooting.md),
- verify host-specific dependencies and path overrides,
- confirm expected behavior against your source command intent.
