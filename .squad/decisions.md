# Squad Decisions

## Active Decisions

- 2026-05-28 (Hudson): Canonical machine model is authoritative for guest-visible Q35/i440fx topology; runtime host resolution may adapt host-local values only and must not silently rewrite placement. Conflict policy prefers explicit import data, then deterministic adapter inference, then ezkvm defaults, with validation failure on chipset-invariant violations.

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction
