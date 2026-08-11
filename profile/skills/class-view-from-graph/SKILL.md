---
name: class-view-from-graph
description: Extract and project UML class diagrams from source. Use when the user asks for a "class diagram", "class structure", "interface contracts", "module boundaries", or "UML class". Drives `archctl code class-diagram` (Rust/TypeScript/Python).
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: uml-class-spec-v1
---

# Objective

Extract class structure from source and project it as a UML class
diagram. Extraction is AST-pure (tree-sitter); projection is
deterministic from the graph.

# Required process

1. Determine scope: whole project, a module, or a single file.
2. Extract (dry-run first):
   ```bash
   # Whole project (MVP languages auto-detected)
   archctl code class-diagram --cwd <dir> --json
   # Scoped: a module or file
   archctl code class-diagram --cwd <dir> --selector module:<id> --json
   archctl code class-diagram --cwd <dir> --selector file:src/domain/model.rs --json
   # Persist to the graph
   archctl code class-diagram --cwd <dir> --selector module:<id> --apply
   ```
3. Project to DSL:
   ```bash
   archctl diagram project --view class:<scope> --format plantuml --output classes.puml --cwd <dir>
   ```
   Supported formats: `plantuml`, `mermaid`, `structurizr`.
4. Validate the bundle if one was exported:
   ```bash
   archctl diagram validate <bundle-dir> --cwd <dir>
   ```

# Scope discipline

- UML views are scoped: never dump the entire repository. Prefer
  module/aggregate/component scope over `class:*`.
- Cross-file inheritance is not supported by the MVP extractor —
  surface this to the user instead of fabricating relationships.

# Forbidden

- Inventing attributes/operations the source does not contain.
- Reporting unsupported languages as extracted (check `--json` output
  for per-language results).
