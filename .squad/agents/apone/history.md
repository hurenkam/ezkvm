# Project Context

- **Owner:** Mark Hurenkamp
- **Project:** ezkvm_v3 redesign for import-source-agnostic VM runtime translation
- **Stack:** Rust, Cargo, YAML schema, QEMU/KVM guest compatibility on Linux
- **Created:** 2026-05-28

## Learnings

- Day 1 context: product intent is drop-in Proxmox-origin VM execution on non-Proxmox Linux hosts.
- Runtime capability validation and dry-run reporting are core requirements.
- Windows 11, passthrough viability, and Looking Glass constraints should be modeled as explicit host capability assumptions.
- 2026-05-28: Assigned windows/gpu/looking-glass documentation coverage audit in this session.
- 2026-05-28: Added concise operational domain docs for Windows 11 baseline, GPU passthrough host readiness, and Looking Glass integration, all aligned to FR-006/FR-007 preflight and deterministic troubleshooting expectations.
