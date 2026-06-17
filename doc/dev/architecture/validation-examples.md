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
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 8589934592
  devices: []
resources: []
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

> **Note:** The `resources` top-level list in the runtime config carries typed `Resource` variants
> (Storage, Network, PciDevice, etc.) using keyed entries (for example `- storage: {...}` and
> `- network: {...}`). Duplicate resource-id validation is implemented and duplicate ids are
> rejected with `ConformanceError::Validation` issues on path `resources`.

---

## Missing IDE Device Resource

**Problem:** An IDE device references a storage resource that doesn't exist.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "workstation"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 8589934592
  devices:
    - ide:
        bus: 0
        address: 0
        type: ssd
        resource: "missing_disk"
resources: []
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.devices[0].ide.resource
  reason: references missing storage resource id 'missing_disk'
  remediation: Add the referenced storage resource under resources or update the device resource id
```

**How to Fix:**

Add the missing storage resource:

```yaml
resources:
  - storage:
      id: "missing_disk"
      block_device: "/dev/vm/workstation-disk"
```

**Why This Matters:**

IDE devices (HDD, SSD, CDROM variants) require a storage resource to be defined. The resource id in the device must match a storage resource id in the top-level resources list. This separation allows devices and resources to be managed independently while enforcing referential integrity.

---

## Invalid Chipset for Machine Family

**Problem:** Chipset value is not supported for the specified machine family.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "test-vm"
virtual_machine:
  machine:
    family: "pc"
    chipset: "virt"  # Invalid for pc family
  memory:
    size: 2147483648
  devices: []
resources: []
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.machine.chipset
  reason: must be one of [q35, i440fx] when machine family is 'pc'
  line: 7
  context:
     6 |   family: "pc"
     7 |   chipset: "virt"
  remediation: Change chipset to 'q35' or 'i440fx' for pc family machines
```

**How to Fix:**

Choose a valid chipset for x86 PC machines:

```yaml
machine:
  family: "pc"
  chipset: "q35"  # Modern, UEFI-capable; recommended for Windows 11, modern Linux
```

OR use the legacy option:

```yaml
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
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 17179869184
  devices: []
resources: []
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
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: "4294967296"  # String (quoted) instead of integer
  devices: []
resources: []
```

**Error Output:**

```
Validation failed with 1 issue(s):
  path: virtual_machine.memory.size
  reason: expected usize
  line: 10
  context:
    9  |   memory:
    10 |     size: "4294967296"
  remediation: Change virtual_machine.memory.size to an integer value (bytes)
```

**How to Fix:**

Remove quotes from numeric values in YAML:

```yaml
memory:
  size: 4294967296  # No quotes; YAML interprets as integer (bytes)
```

**Common YAML Gotchas:**

| Input | YAML Type | Correct? |
|---|---|---|
| `4294967296` | integer | ✓ |
| `"4294967296"` | string | ✗ |
| `4.5` | float | ✗ (must be integer) |
| `0x100000000` | integer | ✓ (hex notation) |

---

## Malformed YAML Is Rejected Before Validation

**Problem:** The document cannot be parsed as YAML, so structural and semantic validation never start.

**Input YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "broken-vm"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: [8589934592
  devices: []
resources: []
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
  machine:
    family: "pc"
    chipset: "arm-virt"  # Invalid for pc
  memory:
    size: "2147483648"  # String not integer
  devices: []
resources: []
```

**File:** `/configs/test-vm.yaml` (filename doesn't match vm_name)

**Error Output:**

```
Validation failed with 3 issue(s):
  path: metadata.schema_version
  reason: is required and must be a non-empty string
  ---
  path: metadata.vm_name
  reason: must match filename stem 'test-vm'
  ---
  path: virtual_machine.machine.chipset
  reason: must be one of [q35, i440fx] when machine family is 'pc'
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

3. **Remove string-quoted memory size** (YAML type error, caught at parse time):
   ```yaml
   memory:
     size: 2147483648  # integer bytes
   ```

4. **Rename file to match vm_name:**
   ```bash
   mv /configs/test-vm.yaml /configs/dev-app.yaml
   ```

**Corrected YAML:**

```yaml
metadata:
  schema_version: "1.0.0"
  vm_name: "dev-app"
virtual_machine:
  machine:
    family: "pc"
    chipset: "q35"
  memory:
    size: 2147483648
  devices: []
resources: []
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
  machine:
    family: pc
    chipset "q35"          # Error: missing ':'
  memory:
    size: "536870912"      # Error: string not integer
  devices: []
resources: []
```

**Expected Issues (at least 3):**
- VM name mismatch with file
- Type error on schema_version (must be a string)
- YAML parse error on chipset (missing `:`)
- Type error on memory.size (string instead of integer)

For actual validation, use the ezkvm test suite:
```bash
cargo test --lib runtime_config::validation -- --nocapture
```

---

## Related Documentation

- [Validation Rules Reference](../requirements/validation-rules.md) — formal rule definitions
- [Canonical Schema Contract](../requirements/canonical-yaml-schema-contract.md) — authoritative spec
- [Coding Guidelines](../architecture/coding-guidelines.md#validation-layer-implementation) — implementation patterns
