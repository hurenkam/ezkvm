# Common Validation Failures and Remediation

This guide shows typical validation errors, their root causes, and step-by-step fixes.

## Missing Required Field

**Problem:** Configuration is missing a required metadata or system field.

**Input YAML:**

```yaml
metadata:
  # schema_version is missing
  vm_name: "win11-prod"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 8192
  storage:
    - id: "disk0"
  network:
    - id: "net0"
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: metadata.schema_version
  reason: is required and must be a non-empty string
  line: 2
  context:
     2 | metadata:
     3 |   vm_name: "win11-prod"
  remediation: Provide a non-empty string value for metadata.schema_version
```

**How to Fix:**

Add the missing field with a semantic version:

```yaml
metadata:
  schema_version: "1.0.0"  # Add this line
  vm_name: "win11-prod"
```

**Why This Matters:**

`schema_version` allows backward-compatible evolution of the YAML format. Tools can handle multiple schema versions correctly.

---

## Duplicate Storage ID

**Problem:** Two storage entries have the same ID, causing topology ambiguity.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "ubuntu-20"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 4096
  storage:
    - id: "system"
    - id: "data"
    - id: "system"  # Duplicate
  network:
    - id: "eth0"
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.storage[2].id
  reason: duplicate id 'system'
  line: 14
  context:
    13 |     - id: "data"
    14 |     - id: "system"
  remediation: Change the id to a unique value; 'system' is already used in storage
```

**How to Fix:**

Rename the duplicate entry to have a unique ID:

```yaml
storage:
  - id: "system"
  - id: "data"
  - id: "backup"  # Changed from 'system'
```

**Why This Matters:**

Each storage device must have a unique identifier so rendering can create deterministic command-line arguments. Duplicates would create ambiguous references.

---

## Invalid Chipset for Machine Family

**Problem:** Chipset value is not supported for the specified machine family.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "test-vm"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "virt"  # Invalid for pc family
    cpu:
      model: "qemu64"
    memory:
      min: 2048
  storage:
    - id: "disk0"
  network:
    - id: "net0"
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.system.machine.chipset
  reason: must be one of [q35, i440fx] when machine family is 'pc'
  line: 8
  context:
     7 |     family: "pc"
     8 |     chipset: "virt"
  remediation: Change chipset to 'q35' or 'i440fx' for pc family machines
```

**How to Fix:**

Choose a valid chipset for x86 PC machines:

```yaml
system:
  machine:
    family: "pc"
    chipset: "q35"  # Modern, UEFI-capable; recommended for Windows 11, modern Linux
```

OR use the legacy option:

```yaml
system:
  machine:
    family: "pc"
    chipset: "i440fx"  # Legacy BIOS-only; for older systems
```

**Decision Basis:**

The canonical machine model is authoritative for guest topology. Only `q35` and `i440fx` are currently supported for x86 PC machines. See [Squad Decisions](../../planning/project-plan.md).

---

## Filename Does Not Match vm_name

**Problem:** Configuration filename stem does not match the vm_name value.

**File:** `/config/production-db.yaml`

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "staging-db"  # Does not match filename 'production-db'
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 16384
  storage:
    - id: "disk0"
  network:
    - id: "eth0"
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: metadata.vm_name
  reason: must match filename stem 'production-db'
  line: 3
  context:
     2 |   schema_version: "1.0.0"
     3 |   vm_name: "staging-db"
  remediation: Rename the file to staging-db.yaml or update vm_name to 'production-db'
```

**How to Fix — Option 1: Rename the File**

```bash
mv /config/production-db.yaml /config/staging-db.yaml
```

**How to Fix — Option 2: Update vm_name**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "production-db"  # Match the filename
```

**Why This Matters:**

Filename and vm_name must be synchronized to prevent confusion when managing configurations. A single vm_name should have exactly one config file.

**Imported Proxmox Corpus Note:**

When validating an imported Proxmox `.conf` document, use the canonical output filename rather than the original numeric source filename. For example, a source file named `108.conf` may intentionally import to `metadata.vm_name: "wakiza"`; validating that imported document as `108.conf` is expected to fail this rule, while validating it as `wakiza.yaml` is expected to pass.

---

## Type Mismatch: String Instead of Integer

**Problem:** A numeric field contains a string value instead of a number.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "debian-11"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: "4096"  # String (quoted) instead of integer
  storage:
    - id: "disk0"
  network:
    - id: "eth0"
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.system.memory.min
  reason: must be an integer
  line: 11
  context:
    10 |     memory:
    11 |       min: "4096"
  remediation: Change virtual_machine.system.memory.min to an integer value
```

**How to Fix:**

Remove quotes from numeric values in YAML:

```yaml
memory:
  min: 4096  # No quotes; YAML interprets as integer
```

**Common YAML Gotchas:**

| Input | YAML Type | Correct? |
|---|---|---|
| `4096` | integer | ✓ |
| `"4096"` | string | ✗ |
| `4.5` | float | ✗ (must be integer) |
| `0x1000` | integer | ✓ (hex notation) |

---

## Malformed YAML Is Rejected Before Validation

**Problem:** The document cannot be parsed as YAML, so structural and semantic validation never start.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "broken-vm"
virtual_machine:
  system:
    memory:
      min: [8192
```

**Observed Result:**

The validator returns a YAML parse error (`ParseError::Yaml`) instead of a field-level `ValidationIssue` list.

**How to Fix:**

- Repair the YAML syntax first
- Re-run validation after the document parses cleanly

**Why This Matters:**

Parse-boundary failures do not include field-path aggregation because the document shape could not be constructed safely.

---

## Filename Stem Unavailable Warning

**Problem:** Validation is invoked without a usable filename stem, so `vm_name` cannot be checked against the source name.

**Observed Result:**

```
Validation Report: 1 issue(s)

WARNINGS (1):
  • metadata.vm_name
    [WARNING] cannot validate vm_name because filename stem is unavailable
    Fix: Ensure the YAML file has a valid filename stem
```

**How to Fix:**

- Validate using a real file path with a normal filename stem
- Avoid passing anonymous or extensionless paths into canonical validation

**Why This Matters:**

The canonical contract treats `metadata.vm_name` and the config filename as a paired identity check.

---

## Multiple Errors in One Document

**Problem:** Configuration has several validation issues that should be fixed together.

**Input YAML:**

```yaml
metadata:
  schema_version: ""  # Empty string
  vm_name: "dev-app"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "arm-virt"  # Invalid for pc
    cpu:
      model: "host"
    memory:
      min: "2048"  # String not integer
  storage:
    - id: "disk0"
    - id: "disk0"  # Duplicate
  network:
    - id: "eth0"
```

**File:** `/configs/test-vm.yaml` (filename doesn't match vm_name)

**Error Output:**

```
Validation failed with 4 issue(s):
  path: metadata.schema_version
  reason: is required and must be a non-empty string
  ---
  path: metadata.vm_name
  reason: must match filename stem 'test-vm'
  ---
  path: virtual_machine.system.machine.chipset
  reason: must be one of [q35, i440fx] when machine family is 'pc'
  ---
  path: virtual_machine.system.memory.min
  reason: must be an integer
  ---
  path: virtual_machine.storage[1].id
  reason: duplicate id 'disk0'
```

**How to Fix — Batch Remediation:**

1. **Add schema_version:**
   ```yaml
   metadata:
     schema_version: "1.0.0"
   ```

2. **Fix chipset:**
   ```yaml
   machine:
     family: "pc"
     chipset: "q35"
   ```

3. **Fix memory type:**
   ```yaml
   memory:
     min: 2048
   ```

4. **Remove duplicate storage ID:**
   ```yaml
   storage:
     - id: "disk0"
     - id: "data"
   ```

5. **Rename file to match vm_name:**
   ```bash
   mv /configs/test-vm.yaml /configs/dev-app.yaml
   ```

**Corrected YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "dev-app"
virtual_machine:
  system:
    machine:
      family: "pc"
      chipset: "q35"
    cpu:
      model: "host"
    memory:
      min: 2048
  storage:
    - id: "disk0"
    - id: "data"
  network:
    - id: "eth0"
```

---

## Testing: Copy-Paste Validation

To validate your learning, update the following template and test:

**Template YAML** (fix the errors):

```yaml
metadata:
  schema_version: 1.0      # Error: not a string
  vm_name: my-test-vm      # File: test-config.yaml
virtual_machine:
  system:
    machine:
      family: pc
      chipset "q35"        # Error: missing ':'
    cpu:
      model: host
    memory:
      min: -512            # Error: negative
  storage:
    - id: "disk"
    - id: "disk"           # Error: duplicate
  network:
    - id: "net0"
```

**Expected Issues (at least 5):**
- VM name mismatch with file
- Type error on schema_version
- Negative memory value
- Duplicate storage ID
- YAML syntax error in chipset

For actual validation, use the ezkvm test suite:
```bash
cargo test --lib runtime_config::validation -- --nocapture
```

---

## Related Documentation

- [Validation Rules Reference](../requirements/validation-rules.md) — formal rule definitions
- [Canonical Schema Contract](../requirements/canonical-yaml-schema-contract.md) — authoritative spec
- [Coding Guidelines](../architecture/coding-guidelines.md#validation-layer-implementation) — implementation patterns
