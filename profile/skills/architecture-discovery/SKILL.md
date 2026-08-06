---
name: architecture-discovery
description: Reverse-engineer a repository into the canonical architecture graph. Use when starting work on a repo, after a refactor, or when asked to "discover", "scan", or "map" the architecture. Drives `archctl code c4-discover` + evidence lifecycle.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: c4-discover-report-v1
---

# Objective

Inventory the repo and populate the canonical LadybugDB graph with
evidence-backed C4 containers. The skill does NOT classify, name, or
decide — it extracts and records with provenance.

# Required process

1. Resolve the project:
   ```bash
   archctl project resolve --cwd <dir>
   ```
2. Inventory the repo to understand the stack:
   ```bash
   archctl inventory languages --cwd <dir>
   archctl inventory tree --cwd <dir> --max-depth 3
   ```
3. Run C4 discovery (dry-run first, then apply):
   ```bash
   # Dry-run: see candidates without persisting
   archctl code c4-discover --cwd <dir> --json
   # Persist: writes inferred Containers + drafted evidences
   archctl code c4-discover --cwd <dir> --apply
   ```
   Target strategies explicitly when the repo is mono-language:
   ```bash
   archctl code c4-discover --cwd <dir> --strategy cargo --apply
   ```
4. Verify the graph was populated:
   ```bash
   archctl graph query --cwd <dir> "MATCH (e:Element) RETURN e.category, e.kind_id, count(e) ORDER BY e.kind_id"
   ```
5. List drafted evidences that need human/agent acceptance:
   ```bash
   archctl evidence list --cwd <dir> --status drafted
   ```

# Coverage gate

- Each persisted Element must have at least one evidence record
  (`archctl evidence list --path <id>`).
- Evidences with status `drafted` are candidates for acceptance; the
  agent decides acceptance via the evidence-lifecycle skill.
- Never invent containers: only what strategies detect.

# Forbidden

- Creating Elements by hand (no `evidence put` with invented IDs).
- Persisting without `--apply` and claiming the graph changed.
- Running extractors against network services (ADR-011).
