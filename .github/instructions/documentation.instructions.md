---
applyTo: "doc/**/*.md,planning/**/*.md,**/*.{md,mdx,txt}"
description: "Use when reviewing, importing, or creating documentation. Start by consulting doc/README.md and align structure and placement to it."
---

# Documentation Workflow Guardrails

Before reviewing, importing, or creating documents:

1. Read `doc/README.md` and use it as the source of truth for documentation layout.
2. Place new content in the correct section (`doc/dev`, `doc/user`, `doc/planning`) unless explicitly instructed otherwise.
3. For imports or migrations, map source content into the repository structure defined in `doc/README.md` and note any assumptions.
4. If you detect conflicts with the documented structure, call them out and ask for confirmation before introducing a new layout.
