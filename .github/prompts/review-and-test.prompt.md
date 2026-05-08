---
name: Review and Test Change
description: "Review Rust code, YAML configuration, serde-backed config changes, or QEMU/KVM behavior in ezkvm, then run targeted validation before concluding."
argument-hint: "What file, diff, module, or change should be reviewed and tested?"
agent: agent
---
Review the requested change in ezkvm and run the most appropriate targeted validation before concluding.

Use workspace skills as needed:
- `review` for correctness, regressions, and missing tests
- `yaml-expert` for YAML and configuration review
- `serde-schema` for serde-backed schema and YAML mapping changes
- `qemu-kvm` for guest, passthrough, networking, storage, and generated QEMU behavior
- `config-doc-sync` for checking whether documentation matches implementation changes
- `rust-programmer` for deeper Rust implementation constraints
- `design-pattern-expert` only when the change is architectural

## Workflow
1. Inspect requested files, diff, or module and identify actual behavior being changed.
2. Produce a findings-first review focused on correctness, regressions, schema risk, runtime behavior, docs drift, and missing tests.
3. Check docs impact: if user-facing config shape/semantics changed, verify `doc/user/config/` and `README.md` updates were made or explicitly deferred in backlog docs.
4. Choose the narrowest useful validation path.
5. Run validation before concluding when feasible.
6. Report findings first, then validation result, then docs impact, then residual risk.

## Validation Rules
- Prefer focused test execution when a relevant test file or test name is identifiable.
- Use `cargo test` when broader validation is justified and targeted tests are insufficient.
- Use `cargo check` when the task is primarily compile-safety or full tests are excessive.
- If the change is docs-only or clearly not testable in this environment, state that explicitly instead of implying validation happened.
- Do not run unrelated broad validation when a narrower command will answer the question.

## Output Format
1. Findings, ordered by severity, with file references when available
2. Validation performed and result
3. Documentation impact (`updated`, `not needed`, or `deferred` with reason)
4. Open questions or assumptions
5. Brief overall risk summary

If there are no findings, say so directly and still report validation status or why validation was not run.