---
name: architecture-discovery
description: Drives the reverse-engineering loop. Use when the user opens the architecture profile in a fresh repo, asks to "discover" or "scan" the architecture, or after a refactor that needs re-ingestion. Wraps `c4-codebase-architecture`.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  wraps: c4-codebase-architecture
  output-schema: architecture-evidence-v1
---

# Objective

Inventory the repo and feed `archctl` enough facts to populate the
canonical graph. The skill does NOT classify, name, or decide — it
extracts and records with provenance.

# Required process

1. `archctl doctor` — refuse to proceed if `renderer.plantuml` is not
   OK.
2. `archctl project resolve` — capture the `project_id` and
   `sourceIdentity` once per session.
3. Call the upstream `c4-codebase-architecture` procedure for the
   inventory pass, but redirect every read to an `archctl` adapter:
   - directory tree → `archctl inventory tree <path>`,
   - language detection → `archctl inventory languages <path>`,
   - dependency edges → `archctl inventory depends <path>`.
4. Reject any fact that lacks evidence. Persist with
   `archctl evidence put --kind semantic --file <json>`.
5. Hand back the element counts and the coverage report.

# Forbidden

- Adding an element or relationship without an evidence file.
- Reusing a `project_id` across unrelated worktrees.
- Calling out to `plantuml.com` / `kroki.io` — only local renderers
  (ADR-011).
- Editing the upstream `SKILL.md` in place; the wrapper stays
  separate.
- `archctl run start` / `archctl run close` — session system not
  available; facts are written directly to the graph.
