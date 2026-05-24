# Epic G Candidate Backlog

Date added: 2026-05-24
Source migrated from: legacy monolithic backlog

## Title

Environment Variable Substitution

## Problem Statement

Configuration loading lacks first-class environment-variable substitution behavior and deterministic unresolved-variable handling.

## Expected Value

Improve deployment ergonomics while preserving deterministic config behavior.

## Dependencies

- None listed in legacy backlog.

## Rough Priority

Medium

## Candidate Tickets

| ID | Title | Depends On | Estimate |
|---|---|---|---|
| G-01 | Add substitution preprocessor in config load pipeline | None | 2 days |
| G-02 | Add unresolved variable policy and tests | None | 1.5 days |

## Ticket Definitions

### G-01 Add substitution preprocessor in config load pipeline
Status: Candidate
Dependencies: None

Acceptance Criteria:
- Supports `${VAR}` and optional default policy if adopted.

Estimate: 2 days

### G-02 Add unresolved variable policy and tests
Status: Candidate
Dependencies: None

Acceptance Criteria:
- Deterministic fail behavior when required variables are missing.

Estimate: 1.5 days
