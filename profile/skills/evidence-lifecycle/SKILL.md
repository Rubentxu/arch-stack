---
name: evidence-lifecycle
description: Manage the evidence lifecycle in the canonical graph — extract, list, accept, supersede. Use when asked to "accept the evidence", "supersede evidence", "review drafted evidence", "audit evidence", or "evidence lifecycle". Drives `archctl evidence *`.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: evidence-lifecycle-v1
---

# Objective

Move evidence through its lifecycle (drafted → accepted →
superseded) with provenance, so projections only reflect facts the
agent/human has vetted.

# Required process

1. See what exists:
   ```bash
   # All evidence for a path
   archctl evidence list --cwd <dir> --path <id>
   # Filter by lifecycle status
   archctl evidence list --cwd <dir> --status drafted
   archctl evidence list --cwd <dir> --status accepted
   ```
2. Extract new structural facts (ast-grep patterns):
   ```bash
   archctl evidence extract --cwd <dir> --lang rust --pattern 'fn (\\w+)' --claim "function definition"
   ```
   Supported langs: rust, type-script, java-script, python, go, java,
   kotlin. Kinds: structural, semantic, behavioral (see `--help`).
3. Ingest semantic facts from JSON (no file source, ADR-027):
   ```bash
   archctl evidence put --cwd <dir> --json --kind behavioral
   # stdin: JSON array of facts, e.g. [{"claim":"...","source_origin":"UserInput"}]
   ```
4. Accept drafted evidence (promotes to canonical):
   ```bash
   archctl evidence accept --id <evidence-id> --cwd <dir>
   ```
5. Replace stale facts (supersede keeps audit trail):
   ```bash
   archctl evidence supersede --id <evidence-id> --cwd <dir>
   ```

# Lifecycle rules

- `drafted`: exists but not vetted (UserInput/ToolOutput provenance).
- `accepted`: canonical, contributes to projections.
- `superseded`: replaced, retained for audit, excluded from projections.
- Acceptance is a decision: accept only what you can defend from the
  source. Never accept to "make the diagram work".

# Forbidden

- Accepting evidence without reading its claim + source.
- Deleting evidence (there is no delete — supersede instead).
- Fabricating `evidence put` payloads that misrepresent the code.
