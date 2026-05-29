# ezkvm_v3 Copilot Instructions

When working with project documentation, treat [doc/README.md](../doc/README.md) as the canonical map for document structure and purpose.

- Always consult `doc/README.md` before reviewing, importing, or creating documentation files.
- Keep documentation organized according to the `dev`, `user`, and `planning` structure described there.
- If a requested document does not clearly fit, propose or document placement rationale relative to `doc/README.md`.

## Helper Registry

- `Documentation Workflow Guardrails`: `.github/instructions/documentation.instructions.md`
	- Use when reviewing, importing, or creating documentation.
- `Docs Sync Enforcement`: `.github/instructions/docs-sync.instructions.md`
	- Use when implementation or configuration changes may require documentation updates.
- `Planning And Backlog Sync`: `.github/instructions/planning-sync.instructions.md`
	- Use when editing planning backlog or feature lifecycle files.

## Agent Registry

- `Docs Sync`: `.github/agents/docs-sync.agent.md`
	- Use for a final pass that maps changed behavior to documentation updates and reports a concise delta.
- `Planning Sync`: `.github/agents/planning-sync.agent.md`
	- Use for a final pass that reconciles planning status, dependencies, and lifecycle placement.

## Prompt Registry

- `Sync Summary`: `.github/prompts/sync-summary.prompt.md`
	- Use to run docs/planning sync pass(es) and return a consistent final summary format.
