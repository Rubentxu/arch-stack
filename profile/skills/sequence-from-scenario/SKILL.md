---
name: sequence-from-scenario
description: Produce a UML sequence diagram for a use case, scenario, endpoint, test, or symbol. Use when the user asks for runtime trace, call path, or inter-service choreography. Drives `archctl code call-graph` + `archctl code sequence`.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: uml-sequence-spec-v1
---

# Objective

Trace a call chain from an entry point and project it as an ordered
interaction list / sequence diagram. Extraction is static
(tree-sitter); async/HTTP flows are not traced in MVP — surface that.

# Required process

1. Identify the entry point: function name, `file:line`, or canonical
   key from the graph.
2. Trace the call chain (read-only, no persistence):
   ```bash
   archctl code sequence --from <entry> --cwd <dir> --json
   # Control depth and volume
   archctl code sequence --from <entry> --depth 3 --max-interactions 100 --cwd <dir>
   ```
3. If the graph lacks the symbol, extract the call graph first:
   ```bash
   archctl code call-graph --cwd <dir> --apply
   ```
4. Optionally project the sequence to DSL:
   ```bash
   # class-diagram / sequence share the projection pipeline
   archctl diagram project --view sequence:<entry> --format plantuml --output trace.puml --cwd <dir>
   ```

# Output contract

- Return the ordered interaction list with participants and message
  kinds (sync/async/return) from `--json`.
- State the call depth used; note when `--max-interactions` truncates.

# Forbidden

- Claiming runtime behavior: the trace is static call resolution,
  not execution.
- Inventing participants or messages the extractor did not emit.
