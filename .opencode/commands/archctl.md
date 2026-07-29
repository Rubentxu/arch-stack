---
description: Run an archctl architecture recovery across the analyzed repo.
agent: archctl-orchestrator
---

# `/archctl`

The orchestrator runs the four-role pipeline against
`$ARCHCTL_PROJECT_DIR`:

1. **Resolve** the project identity (already in `ARCHCTL_*` env vars from
   `shell.env`).
2. **Extract** evidence via the capability router (`extract.outline`,
   `extract.symbols`, `extract.imports`).
3. **Synthesize** the Architecture IR (elements + relationships with
   classification + confidence + evidenceRefs).
4. **Audit** the IR. Hard-fail on any high-confidence unsupported claim.
5. **Render** the IR through the local Structurizr / PlantUML renderers.
6. **Summarize** what was recovered and what was not.

Optional flags:
- `--profile=fast` — fast profile only (ast-grep + ctags + build tools).
- `--profile=deep` — include on-demand deep analyzers (Joern/CodeQL).
- `--render-only` — skip extraction; re-render from an existing IR.
