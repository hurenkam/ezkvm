# ADR-0004: Trait Seam Policy

**Date:** 2026-04-15  
**Status:** Accepted  
**Context:** ezkvm Incremental Convergence Initiative - Trait-Based Extensibility

## Question

Where and how should trait-based extensibility be introduced without compromising the current layered architecture and type safety?

## Decision

**Traits are allowed only at explicit seams: import mappers and runtime extension points. All traits must be compile-time, no runtime plugin loading. No upward calls or layer inversions.**

## Seam Locations (Approved)

### 1. **Import Mapper Seams** (`src/import/`)

**Purpose:** Enable alternative import strategies (Proxmox, KVM, libvirt, custom) without core logic changes.

**Trait Definition:**
```rust
/// Converts external VM config format to canonical VmConfig.
pub trait ConfigImporter: Send + Sync {
    /// Source format (e.g., "proxmox", "libvirt")
    fn format(&self) -> &'static str;
    
    /// Parse and map to canonical schema.
    fn import(&self, input: &str) -> Result<VmConfig>;
    
    /// Generate report of warnings/unsupported features.
    fn report(&self) -> ImportReport;
}
```

**Registry:**
```rust
pub struct ImporterRegistry {
    importers: HashMap<String, Box<dyn ConfigImporter>>,
}

impl ImporterRegistry {
    pub fn register(mut self, importer: Box<dyn ConfigImporter>) -> Self {
        self.importers.insert(importer.format().to_string(), importer);
        self
    }
}
```

**Constraints:**
- One default importer per format (no conflicts)
- All importers output canonical schema (no version negotiation)
- Importers are registered at compile time (no dynamic loading)
- Trait impl lives in `src/import/{format}/mod.rs`

### 2. **Runtime Extension Points** (TBD - reserved for future)

**Purpose:** Allow custom device behavior or auxiliary process handling.

**Reserved for Phase 2 design.** Initially, hooks (ADR-0003) cover device setup. If runtime extensions become necessary (e.g., custom qemu arg builders), they follow the same compile-time registry pattern.

### 3. **Validation Extension Points** (TBD - reserved for future)

**Purpose:** Allow project-specific validation rules beyond canonical schema.

**Reserved for later.** Currently, validation is centralized. If extensibility needed, validators register at compile time in `src/config/validation/mod.rs`.

## Seam Anti-Patterns (Forbidden)

1. **Upward Callbacks** - Traits that call into parent layers (e.g., Hook calls Config).
   - Breaks layering. Use dependency injection instead.

2. **Singleton Pattern** - Trait objects stored in global statics.
   - Use explicit dependency injection.

3. **Runtime Plugin Loading** - Dynamic .so/.dll loading.
   - Complicates security and portability. Compile-time registration only.

4. **Trait Objects in Hot Paths** - Excessive dynamic dispatch in performance-critical code.
   - Use typed structs for hot paths; traits only for extension boundaries.

5. **Impl Trait Leakage** - Returning opaque `impl Trait` that changes meaning across versions.
   - Keep trait boundaries stable; version trait impls explicitly if needed.

## Compile-Time vs Runtime Registration

**Compile-Time (Preferred):**
```rust
// src/import/mod.rs
pub fn default_importers() -> ImporterRegistry {
    ImporterRegistry::new()
        .register(Box::new(ProxmoxImporter::new()))
        .register(Box::new(LibvirtImporter::new()))
}
```

**Why:** Static registry is easier to link, test, and audit. No runtime surprises.

**Runtime (Explicitly Rejected):**
- Plugin loading from filesystem
- Dynamic trait dispatch at startup
- Configuration-driven trait selection

## Layering Guarantee

Traits must not violate unidirectional dependencies:

```
┌─────────────────────────────────────────────────┐
│ CLI                                              │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│ Config [← Trait seam: ConfigImporter]           │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│ QEMU/Runtime [← Future trait seams]              │
└──────────────────┬──────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────┐
│ OS/Device Layer                                  │
└──────────────────────────────────────────────────┘

** No downward calls; updater callbacks blocked by design **
```

## Testing Impact

**Benefit:**
- Trait mocks enable isolated testing of importers
- Config layer remains mockable without changing core logic

**Constraint:**
- All traits must be mockable (no API-only signatures)
- Mock implementations checked into test suite

## Consequences

**Positive:**
- Extension points are explicit and bounded
- Layer boundaries enforced at compile time
- No runtime overhead for unmocked cases
- Clear contract between core and extensions

**Negative:**
- Adding new extension seams requires code changes (not pure config)
- Trait impls live alongside core code
- Limited extensibility for end users (compile required)

## Alternatives Considered

### Alternative 1: Trait Objects Everywhere
- Rejected: destroys type safety, complicates testing, performance tax
- Decision: traits only at approved seams

### Alternative 2: Dependency Injection Container
- Rejected: adds framework complexity
- Decision: explicit constructor injection sufficient for current scale

### Alternative 3: Plugin Crate System
- Rejected: complicates packaging, versioning, security
- Decision: trait registry in main binary only

## Evolution Path

1. **Phase 1 (Now)** - Import mapper seams (Proxmox, libvirt patterns)
2. **Phase 2 (Future)** - Runtime and validation extension seams if needed
3. **Phase 3+ (Forward)** - User-facing plugin system (only if adoption demands it)

## Related ADRs
- [ADR-0001: Base Selection](ADR-0001-base-selection.md)
- [ADR-0002: Import Contract](ADR-0002-import-normalization-contract.md)
- [ADR-0003: Hooks Policy](ADR-0003-hooks-policy.md)

## References

- Backlog task: `doc/backlog/BACKLOG.md` - D-01 through D-04
- Current architecture: `doc/dev/ARCHITECTURE_GUIDELINES.md`
- v1 Trait System: `/home/hurenkam/Workspace/ezkvm_v1/src/vm/mod.rs` (QemuDevice trait)
- Rust best practices: https://www.youtube.com/watch?v=wJZt2LWbsY0 (Trait Object safety patterns)
