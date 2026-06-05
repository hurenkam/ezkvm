# ezkvm_v3 Copilot Instructions

When working with project documentation, treat [doc/README.md](../doc/README.md) as the canonical map for document structure and purpose.

- Always consult `doc/README.md` before reviewing, importing, or creating documentation files.
- Keep documentation organized according to the `dev`, `user`, and `planning` structure described there.
- If a requested document does not clearly fit, propose or document placement rationale relative to `doc/README.md`.

When working in `src/**`, treat `README.md` files in the `src` directory tree as canonical design and requirement inputs for the directory subtree in which they are found.

- Read the nearest in-scope `README.md` under `src/**` before designing or changing implementation within that subtree.
- Treat `src/**/README.md` files as manually authored and read-only by default.
- Edit a `src/**/README.md` only when the user explicitly instructs that edit and confirms it.

## Helper Registry

- `Coding Guidelines`: `doc/dev/architecture/coding-guidelines.md`
	- Use when shaping Rust modules, adding new helper abstractions, or checking file/module scope.
- `Documentation Workflow Guardrails`: `.github/instructions/documentation.instructions.md`
	- Use when reviewing, importing, or creating documentation.
- `Docs Sync Enforcement`: `.github/instructions/docs-sync.instructions.md`
	- Use when implementation or configuration changes may require documentation updates.
- `Planning And Backlog Sync`: `.github/instructions/planning-sync.instructions.md`
	- Use when editing planning backlog or feature lifecycle files.
- `Rust Guideline Enforcement`: `.github/instructions/rust-guidelines.instructions.md`
	- Use when editing Rust source/tests to enforce baseline safety, validation, and reporting expectations.

## Agent Registry

- `Docs Sync`: `.github/agents/docs-sync.agent.md`
	- Use for a final pass that maps changed behavior to documentation updates and reports a concise delta.
- `Planning Sync`: `.github/agents/planning-sync.agent.md`
	- Use for a final pass that reconciles planning status, dependencies, and lifecycle placement.
- `Rust Guidelines Monitor`: `.github/agents/rust-guidelines-monitor.agent.md`
	- Use for a findings-first audit of Rust code quality, regressions, and validation hygiene.

## Prompt Registry

- `Sync Summary`: `.github/prompts/sync-summary.prompt.md`
	- Use to run docs/planning sync pass(es) and return a consistent final summary format.
- `Module Design`: `.github/prompts/module-design.prompt.md`
	- Use when shaping a new module or stage boundary before implementation.

## Skill Registry

- `Design Patterns`: `.copilot/skills/design-patterns/SKILL.md`
	- Use when designing new stage modules, adapter scaffolds, or deterministic render paths.
- `Coding Guideline Conformance`: `.copilot/skills/coding-guideline-conformance/SKILL.md`
	- Use when Rust files are added or modified to run a focused conformance check against coding guidelines before finalizing.
- `Rustdoc Writing`: `.copilot/skills/rustdoc-writing/SKILL.md`
	- Use when documenting Rust code with module headers, struct and enum docs, and function or method rustdoc comments.

## Code Quality Expectations

- Prefer small, single-responsibility modules.
- Keep stage boundaries explicit in names and file layout.
- Consult `doc/dev/architecture/coding-guidelines.md` when introducing new Rust modules or abstractions.
- Before finalizing Rust changes, run a coding-guideline conformance pass on all touched Rust files and report the status.
