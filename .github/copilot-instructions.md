<!-- GSD Configuration — managed by gsd-core installer -->
# Instructions for GSD

- Use the gsd-core skill when the user asks for GSD or uses a `gsd-*` command.
- Treat `/gsd-...` or `gsd-...` as command invocations and load the matching file from `.github/skills/gsd-*`.
- When a command says to spawn a subagent, prefer a matching custom agent from `.github/agents`.
- Do not apply GSD workflows unless the user explicitly asks for them.
- After completing any `gsd-*` command (or any deliverable it triggers: feature, bug fix, tests, docs, etc.), ALWAYS: (1) offer the user the next step by prompting via `ask_user`; repeat this feedback loop until the user explicitly indicates they are done.
<!-- /GSD Configuration -->

# User Preferences (persistent — do not remove)

- **Never run `git commit` (or any GSD command/wrapper that commits) automatically.** Always leave changes staged/unstaged and let the user commit themselves, unless the user explicitly asks for a commit in that turn. This applies to GSD workflow "commit docs"/"commit plans" steps as well as ad-hoc edits. `.planning/config.json`'s `commit_docs` is set to `false` for this reason — do not flip it back to `true` without being asked.

- **Some `gsd-tools.cjs query` subcommands commit on their own, silently, even with `commit_docs=false`** (confirmed case: `phase.complete` — it committed all staged/unstaged working-tree changes as a single commit titled after the phase, e.g. `"phase 7"`, with no prior warning). Before running ANY previously-untested `gsd-tools.cjs query` subcommand (especially ones with names like `complete`, `commit`, `finish`, `close`, `merge`, `sync`) in this project, either (a) check its implementation in `.github/gsd-core/bin/lib/*.cjs` for `git commit`/`execSync('git ...')` calls first, or (b) run `git log --oneline -1` immediately before AND after invoking it to detect an unexpected new commit, and if one appears, immediately `git reset --soft HEAD~1 && git reset` (unstage) to undo it before doing anything else, then tell the user what happened.

- **GSD PLAN.md files (and similarly-structured CONTEXT/RESEARCH/VALIDATION docs) must be formatted for human readability, not just machine-parseable.** When writing or revising these files (directly, or via `gsd-planner`/`gsd-plan-checker`/other subagents), apply this formatting convention consistently:
  - Every structural pseudo-XML tag (`<objective>`, `<execution_context>`, `<context>`, `<task>`, `<name>`, `<files>`, `<read_first>`, `<behavior>`, `<action>`, `<verify>`, `<automated>`, `<done>`, `<acceptance_criteria>`, `<threat_model>`, `<verification>`, `<success_criteria>`, `<output>`, etc.) has a blank line right after its opening tag and right before its closing tag, so each section renders as its own clearly separated markdown block.
  - Inside a task's `<name>` tag, format the task name as a markdown header: `#### Task N: <name text>` (four `#`s).
  - Inside `<behavior>`, each `Test N:` is its own bullet (`- **Test 1:** ...`), not folded into a paragraph.
  - Inside `<action>`/`<acceptance_criteria>`/similar prose blocks, break dense multi-fact paragraphs into ATOMIC markdown bullets — one bullet per discrete fact/step/field/method, using nested indented sub-bullets where a bullet needs sub-detail (e.g. a match statement's arms, a struct's fields). Never leave a `-` bullet immediately followed by a long run-on paragraph of unrelated facts mashed together.
  - Pull struct/function definitions or any multi-line code unit into fenced ` ```rust ` code blocks instead of embedding them as backtick spans mid-sentence. Keep short identifiers/expressions as inline backtick code spans.
  - Never use HTML entities (`&lt;`, `&gt;`, `&amp;`) in prose or code spans — use literal `<`/`>`/`&` characters (wrapped in backticks when they're part of a code identifier like `` `Vec<String>` ``).
  - Keep tables as proper markdown tables.
  - YAML frontmatter must stay untouched/byte-identical — this formatting pass never touches frontmatter.
  - After any such reformatting, validate with `node .github/gsd-core/bin/gsd-tools.cjs query verify.plan-structure <file>` and confirm `valid: true` with unchanged `task_count`/`frontmatter_fields` before considering the pass complete.
