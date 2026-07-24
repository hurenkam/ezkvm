---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 0
total_count: 1
last_updated: 2026-07-24T14:28:40.319Z
---

# Broken Windows Ledger

> Cross-phase defect register. `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 07 | deviation | src/config/ezkvm/schema.rs |  | CpuTopology/VgaConfig root devices (Plan 07-01) have no ezkvm YAML schema representation yet; excluded from yaml_round_trip.rs fidelity assertion until a future phase extends the schema | open |  | 2026-07-24T14:28:40.319Z |  |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "07",
    "file": "src/config/ezkvm/schema.rs",
    "line": null,
    "description": "CpuTopology/VgaConfig root devices (Plan 07-01) have no ezkvm YAML schema representation yet; excluded from yaml_round_trip.rs fidelity assertion until a future phase extends the schema",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-07-24T14:28:40.319Z",
    "resolved_at": null
  }
]
````
