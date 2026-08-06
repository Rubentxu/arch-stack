---
name: diagram-review
description: Validate a generated diagram against its source spec and the canonical graph. Use as the final gate of any diagram invocation. Drives `archctl diagram validate` + `archctl diagram apply` for view-level corrections.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: diagram-review-v1
---

# Objective

Confirm that a bundle renders, matches its view specification, and
that no element or relationship is fabricated. Persist the verdict.

# Required process

1. Validate the bundle (schema + required files):
   ```bash
   archctl diagram validate <bundle-dir> --cwd <dir> --json
   ```
   Exit 0 + `valid: true` = schema-compliant. Non-zero = reject.
2. Cross-check members against the graph:
   ```bash
   archctl graph query --cwd <dir> "MATCH (e:Element) RETURN e.id, e.label"
   ```
   Every bundle node id must resolve to a graph Element.
3. Check evidence backing for each member:
   ```bash
   archctl evidence list --cwd <dir> --path <id>
   ```
   Fail closed: member without accepted evidence → flag.
4. Apply view-level corrections via changeset (if the user requests
   layout/visibility edits — NOT semantic changes):
   ```bash
   archctl diagram apply --changes changeset.json --cwd <dir> --json
   ```
   Changeset must conform to `changeset.schema.json` (move/set-label/
   collapse commands). Stale base revisions are rejected.

# Verdict contract

- PASS: bundle valid + all members graph-resolved + evidence accepted.
- PASS_WITH_WARNINGS: minor (labels, positions) — list them.
- FAIL: any fabricated element/relationship, schema violation, or
  evidence gap. Do NOT ship a FAIL diagram.

# Forbidden

- Rejecting a valid bundle for cosmetic reasons.
- Applying a changeset that renames/removes graph elements (apply is
  cosmetic-only, ADR-013).
