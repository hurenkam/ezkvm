---
name: "design-patterns"
description: "Reusable stage and adapter patterns for ezkvm_v3's import, model, runtime, and render pipeline"
---

## Context

ezkvm_v3 is organized as a staged pipeline:

- import adapters normalize source-specific VM definitions
- `runtime_config` holds the canonical VM model and validation
- `runtime_resolution` applies host-bound runtime facts
- `render_stage` emits deterministic QEMU arguments

Use this skill when designing a new module, trait boundary, or source adapter inside that pipeline.

For config importer stage contracts and naming, consult `src/config_importer/README.md` before proposing API changes.
Treat `src/config_importer/README.md` as manually authored design documentation: do not modify it unless the user explicitly asks for that file to be changed and confirms the change.

## Patterns

### Trait-Based Stage Boundary

Use a small object-safe trait when a stage needs to be selected behind an interface.

- keep request and response types narrow
- prefer an associated error type
- avoid generic methods, `async fn`, and `Self` in return positions
- validate at the boundary before handing data to the next stage

### Adapter Scaffold

Give each import source its own module under `src/config_importer/`.

- parse source payload plus import-host context
- map source terms into canonical model fields
- emit structured diagnostics with source locations when available
- do not probe runtime-host state or mutate runtime-only paths

### Deterministic Rendering

The render stage must be pure and repeatable.

- accept an effective runtime model only
- produce a stable, ordered QEMU argument vector
- keep quoting and ordering canonical
- avoid side effects and host probes inside render

### Validation Before Execution

Validate early, then fail fast on contract violations.

- config importer validates source syntax and source-local semantics
- runtime resolution validates host-bound assumptions
- render stage assumes it receives a valid, effective runtime model
- warnings are acceptable for ignorable source fields; silent drops are not

## Design Checklist

Before proposing a new stage or helper, confirm:

1. Which stage owns the responsibility?
2. What is the smallest valid request/response shape?
3. What is validated here versus the next stage?
4. Is deterministic output required?
5. Does the change need a decision note because it affects shared conventions?

## Anti-Patterns

- combining import, runtime resolution, and rendering in one module
- letting adapters infer runtime-host facts
- making render depend on filesystem or environment discovery
- introducing broad shared abstractions before the pipeline has multiple implementations

---

## Reference: Rust Idiomatic Design Patterns

These are established Rust idioms. Use them as a first vocabulary before reaching for heavier abstractions.

### Newtype

Wrap a primitive or foreign type in a single-field tuple struct to give it a distinct type identity, enforce invariants, and implement `Display`/conversion traits without orphan-rule conflicts.

```rust
struct VmId(u32);
struct MemoryMiB(u64);
```

Use when: a raw type would be passed incorrectly (e.g., swapping two `u32` arguments), or when you need to own trait implementations on an external type.

### Builder

Construct a complex value step-by-step through owned `self`-consuming setters that return `Self`, then a terminal `build()` that validates and returns the finished value.

```rust
QemuCommandBuilder::new()
    .machine(machine)
    .memory(mem)
    .build()?
```

Use when: a struct has many optional or interdependent fields, or when staged validation is needed before the value becomes usable.

### Type State

Encode a value's lifecycle stage into its type parameters so that illegal transitions are rejected at compile time.

```rust
struct Config<S: ConfigState> { inner: Inner, _state: PhantomData<S> }
struct Unvalidated;
struct Validated;

impl Config<Unvalidated> { fn validate(self) -> Result<Config<Validated>, Error> { ... } }
impl Config<Validated>   { fn render(&self) -> Vec<String> { ... } }
```

Use when: a value must pass through distinct lifecycle phases and calling a later-phase method on an earlier-phase value should be a compile error.

### RAII Guard

Pair resource acquisition with `Drop` so that cleanup is guaranteed regardless of the call path.

Use when: a resource must be released, a lock must be unlocked, or a temporary mutation must be rolled back.

### Extension Trait

Add convenience methods to a foreign type or to any type implementing a base trait, without modifying the original.

```rust
pub trait ResultExt<T>: Sized {
    fn or_warn(self, msg: &str) -> Option<T>;
}
impl<T, E: fmt::Debug> ResultExt<T> for Result<T, E> { ... }
```

Use when: you want ergonomic helpers that belong to a coherent concept but should not pollute the primary API.

### Sealed Trait

Prevent external crates from implementing a trait you intend to keep stable by adding a private supertrait.

```rust
mod private { pub trait Sealed {} }
pub trait StageOutput: private::Sealed { ... }
```

Use when: a trait is part of a public API but its implementor set must remain controlled.

### `From` / `Into` Conversions

Express lossless, infallible type conversions through the standard `From<T>` trait; prefer `From` implementations and let `Into` derive for free. Use `TryFrom`/`TryInto` for fallible conversions.

Use when: converting between domain types at stage boundaries (e.g., source model → canonical model).

### Iterator Adapter Pipeline

Build lazy, composable transformations with standard iterator combinators (`map`, `filter_map`, `flat_map`, `fold`, `collect`) instead of explicit loops.

Use when: transforming or validating collections of config entries, CLI arguments, or diagnostic messages.

### Interior Mutability (`Cell` / `RefCell` / `Mutex`)

Allow mutation through a shared reference when ownership rules otherwise prevent it.

- `Cell<T>` — `Copy` values, single-threaded, zero overhead.
- `RefCell<T>` — non-`Copy` values, single-threaded, runtime borrow check.
- `Mutex<T>` / `RwLock<T>` — multi-threaded access.

Use when: a cache, counter, or lazy-init field must live behind `&self` without exposing mutability in the public API.

### Error Handling (`thiserror` + `?`)

Define per-crate or per-module error enums with `#[derive(thiserror::Error)]` and propagate with `?`. Reserve `anyhow` for application-level binaries where error context matters more than type identity.

```rust
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("parse failure: {source}")]
    Parse { #[from] source: serde_json::Error },
}
```

Use when: designing any stage boundary that can fail.

---

## Reference: Gang of Four Patterns in Rust

The GoF patterns remain relevant in Rust but surface differently because ownership, traits, and zero-cost abstractions replace class hierarchies.

### Creational

| Pattern | Rust idiom |
|---------|-----------|
| **Builder** | Owned-setter builder + terminal `build()` (see above) |
| **Factory Method** | A trait with an associated constructor; each implementor is its own factory |
| **Abstract Factory** | A trait whose methods return trait objects or associated types |
| **Prototype** | `Clone` trait |
| **Singleton** | `OnceLock<T>` or `LazyLock<T>` in `std`; avoid global mutable singletons |

### Structural

| Pattern | Rust idiom |
|---------|-----------|
| **Adapter** | A newtype or wrapper struct implementing a target trait over a foreign type |
| **Bridge** | Separate a trait (abstraction) from its impl (implementation) via generics or trait objects |
| **Composite** | An enum with leaf and node variants, or a `Vec<Box<dyn Trait>>` tree |
| **Decorator** | A wrapper struct that delegates to an inner `T: Trait` and adds behavior |
| **Facade** | A single public module/struct that re-exports a simplified API over complex internals |
| **Flyweight** | Shared immutable state via `Arc<T>` or interned string IDs |
| **Proxy** | A wrapper that adds access control, logging, or lazy init, implementing the same trait |

### Behavioral

| Pattern | Rust idiom |
|---------|-----------|
| **Strategy** | A generic type parameter `S: Strategy` or a `Box<dyn Strategy>` field |
| **Observer** | A `Vec<Box<dyn Observer>>` subscriber list, or channels (`mpsc::Sender`) |
| **Command** | A closure `Box<dyn Fn()>` or a plain enum of command variants |
| **Iterator** | `std::iter::Iterator` trait; prefer custom iterators over exposing `Vec` |
| **State** | Type-state (compile-time) or an enum of state variants (runtime) |
| **Template Method** | A trait with provided methods that call required (abstract) hook methods |
| **Visitor** | A trait with `visit_*` methods; or recursive `match` over an ADT |
| **Chain of Responsibility** | An ordered `Vec<Box<dyn Handler>>` tried in sequence |
| **Mediator** | A central coordinator struct that all participants hold a reference to |
| **Memento** | A plain `Clone`-able snapshot struct |
| **Interpreter** | An AST as a recursive enum with an `eval(&self, ctx: &Context)` method |

### When to Apply GoF Patterns

Use this section as a decision aid during design reviews.

| Pattern | Apply when |
|---------|------------|
| **Builder** | Object construction has many optional fields, ordering constraints, or cross-field validation. |
| **Factory Method** | Different implementors must control creation details behind a shared trait contract. |
| **Abstract Factory** | You need to create families of related objects that must remain compatible together. |
| **Prototype** | Cloning a pre-configured baseline is simpler or faster than rebuilding from raw inputs. |
| **Singleton** | Exactly one process-wide service/state instance is required and initialization must be controlled. |
| **Adapter** | Existing type behavior is useful but its interface does not match your target API. |
| **Bridge** | Abstraction and implementation vary independently and should evolve without combinatorial coupling. |
| **Composite** | You need to treat individual elements and nested groups uniformly in tree-like structures. |
| **Decorator** | Behavior must be added orthogonally at runtime without changing the wrapped type. |
| **Facade** | Callers need a stable, simple surface over a complex subsystem with many moving parts. |
| **Flyweight** | Many objects share large immutable data and memory duplication becomes a concern. |
| **Proxy** | Access needs mediation (lazy load, remote call, authorization, logging, or caching). |
| **Strategy** | Multiple interchangeable algorithms are selected by context, config, or runtime conditions. |
| **Observer** | One-to-many notifications are needed when state changes, with loose publisher/subscriber coupling. |
| **Command** | Operations should be first-class values (queueing, retries, undo/redo, auditing). |
| **Iterator** | Traversal logic should be separated from data structure internals and exposed uniformly. |
| **State** | Behavior changes by lifecycle/state and explicit state transitions reduce branching complexity. |
| **Template Method** | You need a shared algorithm skeleton with customizable steps across implementations. |
| **Visitor** | You need to add operations over a stable object graph/AST without modifying each node type repeatedly. |
| **Chain of Responsibility** | Requests should flow through ordered handlers where each can handle, pass, or short-circuit. |
| **Mediator** | Many components interact and direct pairwise coupling would become hard to maintain. |
| **Memento** | You must capture/restore object state snapshots for rollback, checkpoints, or history. |
| **Interpreter** | A small DSL or rule language needs direct execution from a parsed syntax tree. |

### Choosing Generics vs Trait Objects

Prefer **generics** (`fn foo<T: Trait>`) when: the set of implementors is closed, performance matters, or you need associated types.

Prefer **trait objects** (`dyn Trait`) when: you need runtime polymorphism, heterogeneous collections, or the call site should be decoupled from the concrete type at compile time.