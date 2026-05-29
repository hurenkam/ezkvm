# Development Domain Knowledge

This area stores reusable, non-project-specific technical knowledge relevant to virtualization, chipsets, and guest platform behavior.

## Structure

- `q35/`: Q35 chipset architecture and platform behavior references.
- `i440fx/`: i440fx chipset architecture and platform behavior references.
- `qemu/`: QEMU machine-model internals and behavior references.
- `proxmox/`, `windows/`, `linux/`, `macos/`, `gpu/`, `swtpm/`, `looking-glass/`: Reserved for domain notes as they are added.

## Current Documents

- `q35/q35-chipset-domain.md`
- `i440fx/i440fx-chipset-domain.md`
- `qemu/qemu-chipset-behavior.md`
- `windows/windows-11-guest-baseline.md`
- `gpu/gpu-passthrough-host-readiness.md`
- `looking-glass/looking-glass-integration.md`

## Scope Rules

- Keep notes factual and source-driven.
- Exclude repository-specific implementation choices, defaults, and workflow policy.
- If guidance is specific to this repository, place it under architecture, workflow, requirements, or planning docs instead.