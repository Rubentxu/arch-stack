---
name: use-cases-from-graph
description: Derive UML use cases from actors and confirmed goals in the graph. Use when the user asks for use cases, actor mapping, or system landscape analysis. Combines `code state-machine` extraction with `evidence put` for semantic facts.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: experimental
  output-schema: uml-usecase-spec-v1
---

# Objective

Identify actors and use cases, distinguish candidates inferred from
the codebase from use cases confirmed by tests or docs, and relate
each use case to the scenarios that realise it.

# Required process

1. Extract state machines (behavioral states = candidate use-case
   flows):
   ```bash
   archctl code state-machine --cwd <dir> --json
   # Persist states/transitions
   archctl code state-machine --cwd <dir> --apply
   ```
2. Ingest confirmed semantic facts (from docs, tests, user input) as
   evidence — never as Elements:
   ```bash
   # Single fact via file or stdin (--json reads stdin array)
   archctl evidence put --cwd <dir> --json --kind usecase <<< '{"claim":"User can place an order","source_origin":"UserInput"}'
   ```
3. Query the graph for actors (external systems / entry points):
   ```bash
   archctl graph query --cwd <dir> "MATCH (e:Element) WHERE e.category='c4' AND e.kind_id CONTAINS 'mt.container' RETURN e.id, e.label"
   ```
4. Project a use-case view if a UML projection is requested:
   ```bash
   archctl diagram project --view usecase:<scope> --format plantuml --output uc.puml --cwd <dir>
   ```

# Discipline

- Candidates (inferred) vs confirmed (evidence-accepted): label them
  differently in the output.
- `evidence put` records facts with provenance — it does NOT create
  Elements in the graph (ADR-027).
- Never promote a candidate to confirmed without an accepted evidence.

# Forbidden

- Fabricating actors or use cases without graph/evidence backing.
- Using `evidence put` to inject invented architecture.
