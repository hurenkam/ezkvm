---
name: rust-programmer
description: 'Use when working on Rust code in ezkvm, applying Rust design patterns, async Tokio workflows, refactoring, or Rust-specific architecture decisions.'
---

# rust-programmer

**Scope**: Workspace skill for Rust programming expertise, design patterns, and Tokio async patterns in the ezkvm project.

## Description

This skill captures expert Rust programming knowledge with deep understanding of established Rust design patterns, Gang of Four (GoF) design patterns applied to Rust, and Tokio async runtime patterns. Use it when working on Rust code changes, architecture decisions, or implementing new features in the ezkvm project that require sophisticated Rust patterns.

## Use when

- implementing new Rust modules or features in ezkvm
- applying design patterns to solve architectural problems
- working with Tokio async/await patterns and concurrency
- refactoring code for better Rust idioms and performance
- designing error handling and type safety improvements
- implementing CLI command handlers or configuration parsing
- optimizing memory usage or async performance

## Workflow

1. **Understand the current Rust architecture**
   - Review existing modules: `config/`, `qemu/`, `cli.rs`, `state.rs`
   - Identify current patterns: async handlers, error propagation, configuration structs
   - Check for established conventions: `anyhow::Result`, serde traits, clap commands

2. **Choose appropriate Rust patterns**
   - **Creational**: Builder pattern for complex QEMU command construction
   - **Structural**: Adapter pattern for different storage/network backends
   - **Behavioral**: Command pattern for CLI subcommands, Strategy pattern for different execution modes
   - **Concurrency**: Actor pattern with Tokio channels for state management

3. **Apply Tokio async patterns**
   - Use `tokio::spawn` for background tasks (VM monitoring, daemon mode)
   - Implement proper async error handling with `?` operator
   - Use channels for inter-task communication
   - Apply timeouts and cancellation with `tokio::time`

4. **Implement with Rust best practices**
   - Leverage ownership and borrowing for memory safety
   - Use trait objects or enums for polymorphism
   - Implement proper error types with `thiserror`
   - Follow zero-cost abstractions principle
   - Use iterators and functional programming where appropriate

5. **Validate and optimize**
   - Run `cargo clippy` for linting and style checks
   - Use `cargo test` to verify functionality
   - Profile with `cargo flamegraph` if performance is critical
   - Ensure compile-time guarantees are maintained

## Decision points

- **Ownership vs Borrowing**: Choose based on data lifetime requirements and performance needs
- **Trait objects vs Enums**: Use enums for closed sets, traits for extensibility
- **Async vs Sync**: Use async for I/O operations, sync for CPU-bound work
- **Error handling**: Use `anyhow` for application errors, custom types for library APIs
- **Concurrency model**: Channels for message passing, mutexes for shared state
- **Memory management**: Rc/Arc for shared ownership, Box for heap allocation

## Quality criteria

- code compiles without warnings from `cargo clippy`
- follows Rust API guidelines and naming conventions
- proper error handling with meaningful messages
- efficient memory usage and minimal allocations
- async code is cancellation-safe and deadlock-free
- tests cover both success and error paths
- documentation follows rustdoc conventions

## Gang of Four Patterns in Rust

### Creational Patterns
- **Abstract Factory**: Trait-based factory functions for different QEMU backends
- **Builder**: Fluent API for VM configuration building
- **Factory Method**: Creating different types of storage or network devices
- **Prototype**: Cloning VM configurations with modifications
- **Singleton**: Global state management (use with caution)

### Structural Patterns
- **Adapter**: Wrapping different storage APIs (ZFS, LVM, NFS)
- **Bridge**: Separating abstraction (VM management) from implementation (QEMU/LXC)
- **Composite**: Hierarchical configuration structures
- **Decorator**: Adding features to VM configurations dynamically
- **Facade**: Simplified API over complex QEMU operations
- **Flyweight**: Sharing common VM configuration data
- **Proxy**: Lazy initialization of expensive resources

### Behavioral Patterns
- **Chain of Responsibility**: Configuration validation pipeline
- **Command**: CLI subcommand pattern with undo capability
- **Interpreter**: Parsing complex configuration expressions
- **Iterator**: Traversing configuration hierarchies
- **Mediator**: Coordinating between different VM components
- **Memento**: Saving/restoring VM states
- **Observer**: Monitoring VM lifecycle events
- **State**: VM state machine (stopped, running, paused)
- **Strategy**: Different execution strategies (dry-run, daemon, interactive)
- **Template Method**: Common VM lifecycle with customizable steps
- **Visitor**: Operations on different VM configuration types

## Example prompts

- `Use the rust-programmer skill to implement a Builder pattern for complex QEMU command construction.`
- `Use the rust-programmer skill to apply the Strategy pattern for different VM execution modes.`
- `Use the rust-programmer skill to design async error handling with Tokio for VM monitoring.`
- `Use the rust-programmer skill to refactor configuration parsing using the Visitor pattern.`
- `Use the rust-programmer skill to implement the Command pattern for undoable CLI operations.`
- `Use the rust-programmer skill to optimize memory usage in configuration structures.`

## Next customization ideas

- Add a workspace instruction for Rust coding standards and clippy rules
- Create a prompt template for Rust refactoring tasks (`rust-refactor.prompt.md`)
- Add `copilot-instructions.md` for Rust async patterns and error handling
