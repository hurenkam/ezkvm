# Development Domain Knowledge

This area stores reusable, non-project-specific technical knowledge relevant to ezkvm.

Example domains:
- qemu and q35 behavior
- proxmox concepts and mappings
- host OS/runtime behavior
- generic virtualization networking and storage behavior

Transition reference:
- `doc/dev/domain-knowledge/q35-chipset-domain.md`
- `doc/dev/domain-knowledge/i440fx-chipset-domain.md`
- `doc/dev/domain-knowledge/qemu-chipset-behavior.md`

Out-of-scope for this area:
- ezkvm policy, workflow rules, and implementation-specific decisions
- normative guidance that belongs in architecture or workflow docs

Maintenance guidance:
- Include metadata headers in each note: date, scope, and purpose.
- Add a short validation section that lists how findings were verified (for example: source files, command traces, or reproducible test steps).
- Keep normative rules in architecture/workflow docs; keep domain notes factual and reference-driven.
- Prefer primary vendor or upstream project documentation as sources.
- If primary sources are blocked by bot protection, add a "Primary Source Request" section listing exact document titles and URLs needed for manual download.