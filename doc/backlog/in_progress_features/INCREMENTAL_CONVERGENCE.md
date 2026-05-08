# Incremental Convergence Plan

Date: 2026-04-15  
Project: ezkvm  
Approach: Option A style incremental convergence

## 1. Strategic Decision

Recommended base: **current codebase** (`/home/hurenkam/Workspace/ezkvm`)

Rationale:
- Already contains the target foundations to preserve: Profile System, clean layered architecture, and clap-based CLI.
- Lowest migration risk and shortest path to value.
- Allows selective import of v1 strengths without destabilizing active workflows.

Not recommended:
- Full rewrite from scratch (high risk, long lead time, likely feature regression).
- Starting from v1 and modernizing it to parity (would re-implement major current capabilities).

## 2. Capability Direction

### Preserve from v1 (Phase 1 scope)
- Proxmox config import
- Flexible lifecycle hooks
- Trait-based extensibility

### Preserve from current (must remain intact)
- Profile System
- Clean layered architecture
- clap-based CLI

### Later-stage scope (Phase 3)
- Fail-fast validation
- Environment variable substitution
- Storage and device management subcommands
- Improved resource pooling support
- Comprehensive documentation
  - User-oriented: Getting Started, Feature Documentation, Reference Guide
  - Developer-oriented: How to contribute, Architecture and Design, Coding Guidelines

## 3. Architecture Guardrails

1. Canonical schema remains the single source of truth.
2. Proxmox import must normalize to canonical schema output.
3. No bypass of layering (`cli -> config -> domain/runtime -> qemu/os`).
4. Trait extensibility only at explicit seams (import mapping and runtime extension points), not across entire core model.
5. Hooks must be deterministic and observable (timeouts, logs, error policy).
6. New functionality must be gated behind tests before default enablement.

## 4. Delivery Phases

## Phase 0: Program Setup (1 week)
Goal: align architecture, delivery process, and scope boundaries.

## Phase 1: Port v1 strengths (4-6 weeks)
Goal: land Proxmox import, lifecycle hooks, and trait seams safely.

## Phase 2: Integration hardening (2-3 weeks)
Goal: stabilize behavior, remove regressions, prepare release.

## Phase 3: Later-stage enhancements (6-10 weeks)
Goal: validation, env substitution, management subcommands, resource pooling, and full docs.

---

## 5. Backlog Location

The complete implementation backlog is now maintained in `BACKLOG.md`.
