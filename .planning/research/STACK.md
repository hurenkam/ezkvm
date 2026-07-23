# Technology Stack

**Project:** ezkvm — QEMU/Proxmox VM config conversion tool
**Researched:** 2026-07-22
**Overall confidence:** HIGH (formats reverse-engineered from real corpus; crate versions verified against crates.io)

---

## What Already Exists (Keep As-Is)

These dependencies are already in `Cargo.toml` and are the right choices. Do not replace them.

| Dependency | Version (current) | Latest | Verdict |
|------------|-------------------|--------|---------|
| `serde` | 1.0.228 | 1.0.229 | ✅ Keep — minor patch behind, not urgent |
| `saphyr` | 0.0.11 | 0.0.11 | ✅ Keep — `serde_yaml` is officially deprecated; saphyr is the successor |
| `hashlink` | 0.12.1 | 0.12.1 | ✅ Keep — ordered HashMap required for round-trip conf fidelity |
| `ordered-float` | 5.3.0 | 5.3.0 | ✅ Keep — already current |
| `derive-getters` | 0.5.0 | 0.5.0 | ✅ Keep |
| `derive-new` | 0.7.0 | 0.7.0 | ✅ Keep |

**Why saphyr and NOT serde_yaml:** `serde_yaml` 0.9.34 is officially deprecated by its author (David Tolnay). It round-trips through a JSON intermediate representation, which corrupts YAML features like ordering, tags, and multi-line strings. `saphyr` is a fork with proper YAML 1.2 compliance and no JSON intermediary — exactly what ezkvm needs for the schema layer.

---

## What to Add

### 1. `shlex` — QEMU commandline tokenization

```toml
shlex = "2.0.1"
```

**Why:** The `.qemu.cmd` files are 1–2 line flat shell commandlines (e.g., `108.qemu.cmd` is literally one line, `201.qemu.cmd` two). QEMU arguments include quoted strings and special characters. `shlex` correctly handles POSIX shell quoting rules (matching what `/usr/bin/kvm` receives), producing a `Vec<String>` of tokens you can iterate as `(-flag, value)` pairs.

**Why NOT `shell-words`:** Both work, but `shlex` (663M downloads) is 5× more popular and has a more ergonomic API for this use case (`shlex::split(&line) -> Option<Vec<String>>`). Either is fine; `shlex` is marginally simpler.

**Why NOT hand-rolling:** QEMU's `-args` values include spaces inside quoted strings (e.g., `-name wakiza,debug-threads=on`). Naive `split_whitespace()` breaks on those edge cases. `shlex` handles them correctly in one call.

**Confidence:** HIGH

---

### 2. `thiserror` — Promoted to direct dependency

```toml
thiserror = "2.0.19"
```

**Why:** `thiserror` is already a *transitive* dependency (pulled in by `saphyr`). Promoting it to a direct dependency makes the version explicit and enables first-class error enums for the parser modules. The parsing phases need structured error types: `ProxmoxParseError`, `QemuParseError`, `ConversionError` — these are library errors (not application errors), so `thiserror` is the right tool, not `anyhow`.

**Why NOT `anyhow`:** `anyhow` is for application-level error handling (CLI main functions, test harnesses). ezkvm is a library first. `thiserror` generates `std::error::Error` implementations that library consumers can match on.

**Confidence:** HIGH

---

### 3. `winnow` — Sub-option value parsing (conditional)

```toml
winnow = "1.0.4"
```

**When to use it:** Only add this if the sub-option parsing inside values becomes complex. The Proxmox value format has irregular structure:

```
# Simple key=value chain:
smbios1: uuid=04d064c3-66a1-4aa7-9589-f8b3ecf91cd7

# Positional arg + key=value chain:
scsi0: vm1-pool:vm-108-boot,discard=on,size=256G,ssd=1

# Nested colons + commas (requires care):
meta: creation-qemu=8.1.5,ctime=1713817056

# QEMU sub-options with boolean flags (no value):
cpu: host,hv_ipi,hv_relaxed,hv_reset,...,kvm=off,+kvm_pv_eoi
```

The positional-arg + key=value form (storage refs like `vm1-pool:vm-108-boot`) cannot be parsed by pure `split(',').split('=')` — the first token has a colon but is not a key=value pair.

**If adding:** `winnow` 1.0.4 (latest, winnow 1.0.0 released 2026-03-17). It is zero-copy, byte-oriented, and composable. Its combinator style maps cleanly to "parse optional positional arg, then zero-or-more key=value pairs." It is the evolution of `nom` (better error messages, cleaner API).

**Why winnow over nom:** `winnow` (same author, Epage at Rust project) supersedes `nom` for new code in 2025. `nom` 8.0.0 is still maintained but winnow is the recommended path forward. winnow's `separated(0.., key_value, ',')` combinator is more ergonomic than nom equivalents.

**Why NOT pest:** `pest` uses PEG grammars defined in `.pest` files — introduces a grammar file asset, adds build complexity, and is overkill for what is essentially "split on comma, split on equals." `pest` makes sense for full programming language parsers, not config sub-options.

**Alternative:** If you want zero added dependencies, the sub-option value parsing is achievable hand-rolled with careful use of `split_once` and a small state machine. Start hand-rolled; migrate to winnow if you hit edge cases in more than 2–3 device types.

**Confidence:** MEDIUM (winnow is the right library if you need it; whether you need it depends on how many irregular sub-option forms you encounter)

---

## What NOT to Add

| Library | Why Not |
|---------|---------|
| `serde_yaml` | Officially deprecated. `saphyr` is the successor and already in use. |
| `config` (config-rs) | Designed for application configuration (env vars, TOML, JSON), not Proxmox `.conf` parsing. Wrong abstraction. |
| `ini` / `configparser` | Proxmox `.conf` is NOT ini format. The value side of `key: value` is a structured sub-language (positional args + k=v pairs), not a plain string. |
| `clap` (for now) | `main.rs` exists but ezkvm is primarily a library. Add `clap` only when building a real CLI interface. |
| `regex` | The parsing tasks are all position/delimiter based. Regex adds complexity and is slower than combinator or hand-rolled parsing. |
| `nom` | Superseded by `winnow` for new code in 2025. If you need a parser combinator, use `winnow`. |
| `anyhow` | Wrong error model for a library. Use `thiserror` for library error types, `anyhow` only in `main.rs` if needed. |

---

## Proxmox .conf Format — Parsing Strategy

The format (verified against `felucia/108.conf`, `zbp-server-mh2/201.conf`, `zbp-server-mh2/301.conf`) has these constructs:

```
# Comments (including URL-encoded: %3A = colon)
key: value
key: value-with-no-options
key: positional,opt=val,opt2=val2
key: positional:subpath,opt=val
[snapshot-name]    ← section header
```

**Recommended parsing approach:**

```
Level 1 (std library, no crates):
  BufReader::lines() → classify each line:
    - starts with `#`  → comment, skip or preserve
    - matches `[...]`  → section header → begin snapshot section
    - contains `: `    → key-value pair → split_once(": ")

Level 2 (per-value, hand-rolled or winnow):
  For each value string:
    - split_once(',') → (first_token, rest_opts)
    - if first_token contains `=` → pure k=v chain (no positional)
    - else → first_token is positional identifier
    - rest_opts.split(',') → each element: split_once('=') → (key, val) or bare flag

Level 3 (per-option typing):
  Match key strings → typed value: parse string/int/bool/enum
```

This approach requires no new crates beyond what's already in the project. Only add `winnow` if you find the Level 2 parsing has 3+ irregular forms that can't be handled cleanly with `split_once`.

---

## QEMU Commandline Format — Parsing Strategy

The `.qemu.cmd` format (verified against `felucia/108.qemu.cmd`):

```
# Optional binary path line:
/usr/bin/kvm -id 108 -name wakiza,debug-threads=on -no-shutdown ...
```

**Structure:** Single line (or line 1 = comment path, line 2 = actual args). All QEMU arguments follow:
- `-flag` (boolean, no value)
- `-flag value` (value is next token)
- `-flag key=val,key2=val2` (value contains sub-options)

**Recommended approach:**

```rust
// Step 1: tokenize
let tokens: Vec<String> = shlex::split(&cmdline)?;

// Step 2: pair up flags and values
let mut args = tokens.iter().peekable();
while let Some(token) = args.next() {
    if token.starts_with('-') {
        let value = args.peek().filter(|t| !t.starts_with('-')).cloned();
        // dispatch: match token.trim_start_matches('-') { ... }
    }
}

// Step 3: per-flag sub-option parsing (same Level 2 as above)
```

The same sub-option parsing logic from Proxmox `.conf` reuses here — extract into a shared `parse_options(value: &str) -> (Option<&str>, Vec<(String, Option<String>)>)` helper.

---

## Recommended Stack Summary

```toml
[dependencies]
# Already present — keep at these versions
derive-getters = "0.5.0"
derive-new = "0.7.0"
hashlink = "0.12.1"
ordered-float = "5.3.0"
saphyr = "0.0.11"
serde = { version = "1.0.228", features = ["derive"] }

# Add these
thiserror = "2.0.19"    # Promote from transitive; needed for library error types
shlex = "2.0.1"         # QEMU commandline tokenization

# Add conditionally (only if sub-option parsing becomes complex)
# winnow = "1.0.4"
```

---

## Confidence Assessment

| Area | Confidence | Basis |
|------|------------|-------|
| Proxmox .conf format | HIGH | Verified against 3+ real conf files from corpus; format is well-understood |
| QEMU cmdline format | HIGH | Verified against `108.qemu.cmd`; 1-line shell arg format is unambiguous |
| saphyr choice | HIGH | serde_yaml deprecated by author; saphyr is the direct community successor |
| shlex choice | HIGH | 663M downloads; unambiguous fit for shell tokenization |
| thiserror choice | HIGH | Standard Rust library error pattern; already transitive dep |
| winnow choice | MEDIUM | Right tool if needed; uncertain whether complexity warrants it |
| No Proxmox official Rust parser | HIGH | Verified: no proxmox-section-config or proxmox-schema on crates.io |

---

## Sources

- crates.io API (verified 2026-07-22): winnow 1.0.4, shlex 2.0.1, thiserror 2.0.19, nom 8.0.0, saphyr 0.0.11
- serde_yaml deprecation: confirmed via crates.io version string `0.9.34+deprecated`
- Proxmox .conf format: reverse-engineered from `input/felucia/108.conf`, `input/zbp-server-mh2/201.conf`, `301.conf`
- QEMU cmdline format: reverse-engineered from `input/felucia/108.qemu.cmd`
- Existing codebase stack: `.planning/codebase/STACK.md`
