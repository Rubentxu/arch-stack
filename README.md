# arch-stack

**Architecture diagrams from code — C4, UML, sequence, and more.**

`arch-stack` is a local-first CLI + workbench that reverse-engineers your repository into an architecture knowledge graph and projects it as interactive C4 and UML diagrams. It runs entirely on your machine; nothing leaves your environment by default.

[![Latest Release](https://img.shields.io/github/v/release/Rubentxu/arch-stack?logo=github&label=latest)](https://github.com/Rubentxu/arch-stack/releases/latest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Build](https://img.shields.io/github/actions/workflow/status/Rubentxu/arch-stack/release.yml?logo=github)](https://github.com/Rubentxu/arch-stack/actions)
[![rust-version](https://img.shields.io/badge/rust-1.91%2B-blue.svg?logo=rust)](archctl/Cargo.toml)

---

## At a Glance

```
$ archctl doctor                              # verify setup
$ archctl ide install opencode                 # connect to OpenCode
$ /diagram c4 context                          # C4 System Context
$ /diagram c4 container                       # C4 Container
$ /diagram class order-domain                # UML Class diagram
$ /diagram sequence "create order"            # UML Sequence
$ archctl view                               # open the workbench
```

---

## Features

| Capability | Description |
|---|---|
| **C4 Diagrams** | Context, Container, Component levels from code extraction |
| **UML Diagrams** | Class, Sequence, State, Use Case |
| **Capabilities** | Full capability matrix by language and maturity: see [docs/CAPABILITIES.md](../docs/CAPABILITIES.md) |
| **Local-first** | All data stays in `~/.local/share/archctl/` (XDG) |
| **Evidence-backed** | Every node and edge links to `file:line` provenance |
| **Embedded workbench** | `archctl view` serves archview from the binary (no separate install) |
| **IDE integration** | OpenCode, ZCode, Claude Code, Codex via `archctl ide` |
| **Reproducible** | Deterministic projections from the same code base |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Your IDE (OpenCode / Claude Code)         │
│                                                                  │
│   /diagram c4 container                                            │
│        │                                                         │
│        ▼                                                         │
│   diagram-architect  (orchestrator agent)                         │
│   ├── c4-modeler        → c4-from-graph skill                   │
│   ├── uml-modeler        → class/sequence/usecase skills         │
│   ├── architecture-evidence → architecture-discovery skill        │
│   └── diagram-reviewer    → diagram-review skill                │
└────────────────────────────┬────────────────────────────────────┘
                             │ archctl code / archctl diagram
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                       archctl (Rust CLI)                         │
│                                                                  │
│   code call-graph   → extract facts from source code            │
│   code class-diagram                                             │
│   code sequence                                                   │
│   code state-machine                                              │
│   diagram export   → project views (C4 / UML)                    │
│   view             → serve embedded archview workbench           │
│                                                                  │
│   LadybugDB (graph) ← persists in ~/.local/share/archctl/       │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                        archview (Embedded Workbench)              │
│   G6 canvas renderer · pan/zoom · evidence inspector · filters   │
│   (served by archctl view — no separate installation needed)     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Installation

### Option 1 — Homebrew (recommended)

```bash
brew install Rubentxu/arch-stack/archctl
```

### Option 2 — Binary release

```bash
# Download the latest release for Linux x86_64
curl -L https://github.com/Rubentxu/arch-stack/releases/latest/download/archctl-x86_64-unknown-linux-gnu.tar.gz \
  -o /tmp/archctl.tar.gz

# Extract to a directory in your PATH
tar -xzf /tmp/archctl.tar.gz -C ~/.local/bin/

# Verify
archctl --version
```

For other platforms, see [Releases](https://github.com/Rubentxu/arch-stack/releases/latest).

### Option 3 — From source

```bash
git clone https://github.com/Rubentxu/arch-stack.git
cd arch-stack/archctl
cargo build --release
./target/release/archctl --version
```

### Option 4 — With self-update (once you have archctl)

```bash
archctl self install          # install latest stable
archctl self update          # update to the latest
archctl self update --check  # check without installing
```

---

## IDE Integration

After installing `archctl`, connect it to your IDE:

```bash
# OpenCode
archctl ide install opencode

# Claude Code
archctl ide install claude-code

# ZCode
archctl ide install zcode

# Codex
archctl ide install codex

# Check what's installed
archctl ide list --installed

# Diagnose an IDE
archctl ide doctor opencode
```

---

## CLI Reference

### Diagnosis and setup

```bash
archctl doctor              # verify environment and dependencies
archctl project resolve      # detect current repository identity
```

### Code extraction

```bash
archctl code call-graph        # extract call graph (all supported languages)
archctl code class-diagram    # extract class / struct definitions
archctl code sequence         # extract sequence / call chains
archctl code state-machine    # extract state machine candidates
```

### Diagram projection

```bash
archctl diagram export c4-context             # C4 System Context view
archctl diagram export c4-container          # C4 Container view
archctl diagram export c4-component          # C4 Component view
archctl diagram project --view sequence:*   # UML Sequence view
archctl diagram project --view class:*      # UML Class view
archctl diagram project --view usecase:*    # UML Use Case view
archctl diagram project --view state:*      # UML State Machine view
```

### Workbench

```bash
archctl view                # start embedded workbench (random port)
archctl view --port 9000   # start on a fixed port
```

### Graph introspection

```bash
archctl graph list-elements                # list all nodes
archctl graph list-relations               # list all edges
archctl evidence list --element <id>       # evidence for a node
archctl evidence list --relation <rel>      # evidence for an edge
```

### Lifecycle management

```bash
archctl self install [version]   # install a version
archctl self list                 # list installed versions
archctl self use <version>       # switch active version
archctl self update              # update to latest
archctl self uninstall           # remove current version
```

---

## OpenCode Commands

When integrated with OpenCode, the `/diagram` command is the entry point:

| Command | Description |
|---|---|
| `/diagram c4 context` | C4 System Context diagram |
| `/diagram c4 container` | C4 Container diagram |
| `/diagram c4 container <scope>` | C4 Container scoped to a module |
| `/diagram c4 component <module>` | C4 Component diagram |
| `/diagram class <module>` | UML Class diagram |
| `/diagram sequence <function>` | UML Sequence for a call chain |
| `/diagram usecase <name>` | UML Use Case diagram |
| `/diagram state <entity>` | UML State Machine |
| `/diagram explain <element-id>` | Show evidence for a node |
| `/diagram evidence <relation-id>` | Show evidence for a relation |
| `/diagram update` | Refresh diagrams after code changes |
| `/diagram review` | Validate diagram quality against the graph |

---

## External Dependencies (optional)

Some commands require external tools. `archctl` reports a clear error if they are missing.

| Tool | Needed for | Install |
|---|---|---|
| Java | PlantUML rendering | `sudo apt install default-jre` |
| PlantUML | PlantUML diagram export | [plantuml.com](https://plantuml.com/download) |
| ast-grep | Multi-language code extraction (see [docs/CAPABILITIES.md](../docs/CAPABILITIES.md)) | `cargo install ast-grep` |
| tree-sitter-graph | Advanced extraction | `cargo install tree-sitter-graph` |

---

## Documentation

| Document | Description |
|---|---|
| [`docs/README.md`](docs/README.md) | Full documentation index |
| [`docs/STATE.md`](docs/STATE.md) | Current ship state, shipped capabilities |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Horizons H0–H3 + milestone history |
| [`docs/adr/`](docs/adr/) | Architecture Decision Records (041 ADRs) |
| [`docs/specs/`](docs/specs/) | View specifications and contracts |
| [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md) | Canonical graph data model |
| [`MANUAL.md`](MANUAL.md) | Complete user guide |

---

## Data Persistence

All data lives outside your repository, following [ADR-004](docs/adr/ADR-004-persistencia-externa-xdg.md):

```
~/.local/share/archctl/
└── projects/<hash>/
    └── architecture.lbdb      # the canonical graph

~/.config/archctl/
├── config.toml                 # global configuration
└── plugins/                   # installed plugins
```

Your source code is **never modified**. `archctl` only reads.

---

## License

`arch-stack` is distributed under **MIT OR Apache-2.0**. See [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE) for details.
