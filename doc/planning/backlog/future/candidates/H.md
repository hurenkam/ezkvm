# Epic H Candidate Backlog

Date added: 2026-05-24
Source migrated from: legacy monolithic backlog

## Title

Storage/Device Management Subcommands

## Problem Statement

The CLI lacks full storage and device management command surfaces expected for operational workflows.

## Expected Value

Improve operator workflow completeness by exposing common storage/device actions directly in ezkvm CLI.

## Dependencies

- None listed in legacy backlog.

## Rough Priority

Medium

## Candidate Tickets

| ID | Title | Depends On | Estimate |
|---|---|---|---|
| H-01 | Storage subcommands baseline | None | 5 days |
| H-02 | Device subcommands baseline | None | 4 days |

## Ticket Definitions

### H-01 Storage subcommands baseline
Status: Candidate
Dependencies: None

Acceptance Criteria:
- Create/info/list/resize/snapshot commands implemented and tested.

Estimate: 5 days

### H-02 Device subcommands baseline
Status: Candidate
Dependencies: None

Acceptance Criteria:
- USB/PCI listing and selected hotplug operations available.

Estimate: 4 days
