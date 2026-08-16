# arch-stack — User Manual

> A complete guide to installing, configuring, and using arch-stack for architecture diagram generation from code.

**Latest version:** v1.38.0 — [github.com/Rubentxu/arch-stack](https://github.com/Rubentxu/arch-stack)

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Core Concepts](#2-core-concepts)
3. [Installation](#3-installation)
4. [IDE Integration](#4-ide-integration)
5. [CLI Reference](#5-cli-reference)
6. [Workflows](#6-workflows)
7. [The Knowledge Graph](#7-the-knowledge-graph)
8. [Skills and Agents](#8-skills-and-agents)
9. [The Workbench](#9-the-workbench)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Introduction

### What is arch-stack?

`arch-stack` is a **local-first architecture diagram tool** that extracts facts from your source code, persists them in a local graph database (LadybugDB), and projects that graph as C4 and UML diagrams.

The key idea: **the diagram is a view of the graph, not the source of truth**. The graph is. This means diagrams are always consistent with the code — they are derived, not manually maintained.

### What it is not

- A documentation tool that you update by hand
- A service that sends data to the cloud
- A diagram editor (drag-and-drop)
- A code generator from diagrams

### The two components

| Component | Language | Role |
|---|---|---|
| **archctl** | Rust | CLI: extracts code facts, manages the graph, serves the workbench |
| **archview** | TypeScript + G6 | Workbench: renders interactive diagrams in the browser |

`archview` is embedded inside `archctl` — you never install it separately.

### What it supports today

| Feature | Status |
|---|---|
| C4 Context / Container / Component | ✅ Stable |
| UML Class / Sequence / State / Use Case | ✅ Stable |
| Call graph extraction | ✅ Multi-language (see [docs/CAPABILITIES.md](../docs/CAPABILITIES.md)) |
| Evidence provenance (`file:line`) | ✅ Every node and edge |
| Local rendering (Mermaid, PlantUML, SVG) | ✅ |
| Interactive workbench (`archctl view`) | ✅ Embedded in binary |
| Multi-IDE integration | ✅ OpenCode, Claude Code, ZCode, Codex |
| Multi-version CLI management | ✅ `archctl self *` |

---

## 2. Core Concepts

Understanding these five concepts makes everything else clear.

### 2.1 The Canonical Graph

The **canonical graph** is a LadybugDB database stored at:

```
~/.local/share/archctl/projects/<project-hash>/architecture.lbdb
```

Every extracted fact (a function call, a class definition, a state transition) is stored as a **node** or **edge** with:

- A stable identity (hash-based, not name-based — renaming doesn't break identity)
- A confidence score (1.0 = extracted from code; <1.0 = inferred or user-declared)
- Evidence references pointing to `file:line` in your source

The graph is **append-only** for facts. Cosmetic changes (label, position) are separate.

### 2.2 Project Identity

`archctl` identifies projects by the hash of their Git remote URL:

```
~/.local/share/archctl/projects/a3f8b2c1.../
```

If your remote changes (new origin URL), `archctl` treats it as a new project. This is intentional — different remotes are different projects.

### 2.3 Projections (Views)

A **projection** is a deterministic transform of the graph into a diagram format. The same graph can produce many projections:

| Projection | Format | Renderer |
|---|---|---|
| C4 Context | Mermaid / PlantUML / JSON bundle | archview, SVG, PNG |
| C4 Container | Mermaid / PlantUML / JSON bundle | archview, SVG, PNG |
| UML Class | Mermaid / PlantUML / JSON bundle | archview, SVG, PNG |
| UML Sequence | Mermaid / PlantUML | SVG |
| UML State | Mermaid / PlantUML | SVG |

Projections are **read-only** views of the graph. If you want to change a diagram, you change the graph (by accepting/rejecting evidence or fixing the code).

### 2.4 Evidence

**Evidence** is the link between a graph element and its source of truth. Every node and edge carries `evidence_refs` — a list of `file:line` pointers.

Example:

```
Container: payment-service
  evidence_refs:
    - src/services/payment.rs:42    ← defines the struct
    - src/api/payment.rs:15        ← exposes the HTTP handler
```

If `src/services/payment.rs:42` changes, the evidence may become stale. `archctl` reports this on export.

### 2.5 The Five Invariants

`arch-stack` guarantees five things (per ADR-038):

1. **Canonical graph**: the single source of truth for all architecture facts
2. **Evidence per node/arista**: every fact links to `file:line` provenance
3. **XDG-only persistence**: nothing in the user's repository (no `.archctl/`, no `.opencode/`)
4. **Cosmetic-only apply**: `archctl diagram apply` changes positions and labels, not the graph
5. **Local-first renderers**: no data leaves the machine by default

---

## 3. Installation

### 3.1 Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| OS | Linux, macOS | Windows support deferred (see ADR-039 anti-roadmap) |
| Rust (source build) | ≥ 1.91 | Only if building from source |

No runtime dependencies required for core functionality. Optional tools are listed in [External Dependencies](#external-dependencies-optional).

### 3.2 Option A — Homebrew (recommended)

```bash
brew install Rubentxu/arch-stack/archctl
```

Verify:

```bash
archctl --version
# archctl v1.38.0
```

Update:

```bash
brew upgrade Rubentxu/arch-stack/archctl
```

### 3.3 Option B — Binary release

Find your platform in [Releases](https://github.com/Rubentxu/arch-stack/releases/latest). Assets include:

```
archctl-x86_64-unknown-linux-gnu.tar.gz      # Linux x86_64
archctl-aarch64-apple-darwin.tar.gz          # macOS Apple Silicon
archctl-x86_64-apple-darwin.tar.gz          # macOS Intel
SHA256SUMS                                   # verification file
```

Install:

```bash
# Replace VERSION and PLATFORM as needed
VERSION=$(curl -s https://api.github.com/repos/Rubentxu/arch-stack/releases/latest | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/')
PLATFORM="x86_64-unknown-linux-gnu"

curl -L "https://github.com/Rubentxu/arch-stack/releases/download/v${VERSION}/archctl-${PLATFORM}.tar.gz" \
  -o /tmp/archctl.tar.gz

# Optional: verify
curl -L "https://github.com/Rubentxu/arch-stack/releases/download/v${VERSION}/SHA256SUMS" -o /tmp/SHA256SUMS
sha256sum -c /tmp/archctl.tar.gz.sha256 /tmp/archctl.tar.gz

# Extract
tar -xzf /tmp/archctl.tar.gz -C ~/.local/bin/

# Add to PATH if needed (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

### 3.4 Option C — From source

```bash
# Clone the repository
git clone https://github.com/Rubentxu/arch-stack.git
cd arch-stack

# Build (requires Rust ≥ 1.91)
cd archctl
cargo build --release

# The binary is at target/release/archctl
./target/release/archctl --version
```

### 3.5 Option D — Self-update (once installed)

```bash
# Install latest stable
archctl self install

# Install a specific version
archctl self install 1.35.0

# List installed versions
archctl self list

# Switch to a different version
archctl self use 1.35.0

# Update to latest
archctl self update

# Check for updates without installing
archctl self update --check

# Uninstall current version
archctl self uninstall

# Uninstall and remove all data
archctl self uninstall --purge
```

### 3.6 Verify your installation

```bash
archctl doctor
```

Expected output: `✅ All checks passed` with a list of detected tools and their status.

---

## 4. IDE Integration

`archctl` can install its skills and agents into supported IDEs. The process copies the profile to the IDE's native discovery directory.

### 4.1 Supported IDEs

| IDE | Discovery path | Adapter |
|---|---|---|
| OpenCode | `~/.config/opencode/{skills,agents,plugins}/` | OpenCodeAdapter |
| Claude Code | `~/.claude/plugins/arch-stack/{skills,agents}/` | ClaudeCodeAdapter |
| ZCode | `~/.config/zcode-{skills,agents}/` | ZCodeAdapter |
| Codex | `~/.codex/prompts/<name>.toml` | CodexAdapter |

### 4.2 OpenCode setup

```bash
archctl ide install opencode
```

This copies `profile/{skills,agents,plugins}` to `~/.config/opencode/`.

Then launch OpenCode:

```bash
opencode
```

Or set the config directory explicitly:

```bash
OPENCODE_CONFIG_DIR=~/.config/opencode opencode
```

### 4.3 Claude Code setup

```bash
archctl ide install claude-code
```

### 4.4 Multiple IDEs

You can have skills installed in multiple IDEs simultaneously. Each IDE's adapter handles the installation path correctly.

### 4.5 Verify IDE integration

```bash
# List supported IDEs and installation status
archctl ide list --installed

# Run diagnostics for a specific IDE
archctl ide doctor opencode
```

---

## 5. CLI Reference

### 5.1 Global flags

```bash
archctl [--version] [--help] [--json] [--cwd <dir>]
```

- `--json` — machine-readable output for scripting
- `--cwd` — override working directory (default: current directory)

### 5.2 `archctl doctor`

Verifies the environment: required tools, permissions, and configuration.

```bash
archctl doctor
archctl doctor --scopes diagram,evidence  # specific scopes only
```

Exit code: `0` if all checks pass, `1` otherwise.

### 5.3 `archctl project resolve`

Detects the current project identity from the Git remote URL.

```bash
archctl project resolve
```

Output:
```json
{
  "project_id": "a3f8b2c1d4e5f6...",
  "root": "/home/user/my-project",
  "remote": "https://github.com/user/my-project.git"
}
```

### 5.4 `archctl code` — Code extraction

```bash
archctl code call-graph        # extract call graph (all languages)
archctl code call-graph --lang rust   # specific language
archctl code call-graph --lang go
archctl code call-graph --lang python
archctl code call-graph --lang java
archctl code call-graph --lang kotlin

archctl code class-diagram    # extract class/struct/module definitions
archctl code sequence         # extract call chains / sequences
archctl code state-machine     # extract state machine candidates
```

All commands write to the project graph at `~/.local/share/archctl/projects/<hash>/`.

### 5.5 `archctl diagram` — Projection

```bash
# C4 views
archctl diagram export c4-context
archctl diagram export c4-container
archctl diagram export c4-component

# UML views
archctl diagram project --view sequence:*
archctl diagram project --view sequence:src/orders/create.rs::create_order
archctl diagram project --view class:*
archctl diagram project --view class:order-domain
archctl diagram project --view usecase:*
archctl diagram project --view state:*
archctl diagram project --view state:Order

# Render formats
archctl diagram export c4-context --format mermaid    # default
archctl diagram export c4-context --format plantuml
archctl diagram export c4-context --format svg
archctl diagram export c4-context --format json       # bundle for archview

# Selector (filter what appears in the view)
archctl diagram export c4-container --selector container:payments
```

### 5.6 `archctl view` — Workbench

Serves the embedded archview workbench as a local HTTP server.

```bash
archctl view              # random available port
archctl view --port 9000  # fixed port
```

Opens `http://127.0.0.1:<port>` in your default browser.

The workbench is served from assets embedded in the binary — no separate installation.

### 5.7 `archctl graph` — Graph introspection

```bash
archctl graph list-elements                       # all nodes
archctl graph list-elements --kind container       # filter by kind
archctl graph list-relations                      # all edges
archctl graph list-relations --from <element-id>  # edges from a node
```

### 5.8 `archctl evidence` — Evidence management

```bash
archctl evidence list                              # all evidence
archctl evidence list --element <id>               # evidence for a node
archctl evidence list --relation <rel>           # evidence for an edge
archctl evidence accept <id>                      # promote drafted → accepted
archctl evidence reject <id>                      # mark as rejected
archctl evidence supersede <old-id> --by <new-id> # replace old with new
```

### 5.9 `archctl self` — Version lifecycle

```bash
archctl self install [version]     # install (default: latest)
archctl self list                  # installed versions
archctl self list --json           # machine-readable
archctl self use <version>         # switch active version
archctl self update                # update to latest
archctl self update --channel rc    # update to RC channel
archctl self update --check        # check without installing
archctl self uninstall             # remove current version
archctl self uninstall --purge     # remove + all data
```

### 5.10 `archctl ide` — IDE management

```bash
archctl ide install <ide>          # install stack to IDE
archctl ide list                   # list supported IDEs
archctl ide list --installed       # list installed IDEs
archctl ide doctor <ide>           # diagnose IDE integration
archctl ide remove <ide>           # remove from IDE
archctl ide update <ide>           # reinstall (alias for install)
```

### 5.11 `archctl config` — Configuration

```bash
archctl config get <key>          # read a config value
archctl config set <key> <value> # write a config value
archctl config list               # list all config
```

Example:
```bash
archctl config set view.default_port 9000
archctl config set rendering.backend plantuml
```

---

## 6. Workflows

### 6.1 First-time setup on a project

```bash
# 1. Navigate to the project
cd /path/to/your/project

# 2. Verify the environment
archctl doctor

# 3. Connect to OpenCode (if using)
archctl ide install opencode

# 4. Open OpenCode
opencode

# 5. Discover what archctl sees
/diagram discover
```

### 6.2 Generate a C4 Context diagram

```bash
# From CLI
archctl diagram export c4-context --format json > /tmp/context.json

# From OpenCode
/diagram c4 context
```

The diagram shows external actors (people and software systems) and their interactions with your system.

### 6.3 Generate a C4 Container diagram

```bash
archctl diagram export c4-container

# Scoped to a specific area
archctl diagram export c4-container --selector container:payments
```

Shows the major containers (applications, databases, message queues) within your system.

### 6.4 Explore component internals

```bash
# From CLI
archctl diagram export c4-component --selector container:payments

# From OpenCode
/diagram c4 component payments
```

Shows the components inside a container and their relationships.

### 6.5 Understand a call chain

```bash
# Extract call graph first
archctl code call-graph

# Then project sequence
archctl diagram project --view sequence:src/orders/create_order

# From OpenCode
/diagram sequence "create_order"
```

### 6.6 View class structure

```bash
archctl code class-diagram
archctl diagram project --view class:*

# Specific module
/diagram class order-domain
```

### 6.7 Check evidence for a node

```bash
# Find the node ID first
archctl graph list-elements --kind container

# Then check its evidence
archctl evidence list --element container:payment-service
```

This shows which files and lines back up the existence of `payment-service`.

### 6.8 Refresh diagrams after code changes

```bash
# Re-extract from source
archctl code call-graph

# Re-project all views
archctl diagram project --view c4-container
```

### 6.9 Open the interactive workbench

```bash
archctl view
```

This serves archview at a local port and opens your browser. You can:

- Pan and zoom the diagram
- Click a node to see its evidence
- Filter by element type
- Change layout (dagre, force-directed)
- Export as SVG or PNG

---

## 7. The Knowledge Graph

### 7.1 What gets extracted

The extraction phase walks your source code and produces facts:

| Extractor | Facts produced |
|---|---|
| `call-graph` | Functions that call other functions (edges), function definitions (nodes) |
| `class-diagram` | Classes, structs, interfaces, fields, methods, inheritance |
| `sequence` | Call chains for specific entry points |
| `state-machine` | State variables and their transition candidates |

### 7.2 Node kinds

```
person         ← external human actor
system        ← external software system
container     ← application, service, database, queue
component     ← internal module / class group
function     ← extracted from call graph
class         ← extracted from class diagram
interface     ← extracted from class diagram
state         ← extracted from state machine
use_case      ← from use case analysis
```

### 7.3 Edge kinds

```
calls              ← function A calls function B
implements        ← class implements interface
extends           ← class extends parent
depends_on        ← container depends on container
part_of           ← component belongs to container
interacts_with    ← person/system interacts with container
participates_in   ← actor participates in use case
```

### 7.4 Confidence scoring

Every extracted fact has a confidence score:

| Score | Meaning | Source |
|---|---|---|
| `1.0` | Certain | Extracted directly from code |
| `0.8–0.9` | High confidence | Heuristic with strong signal |
| `0.5–0.7` | Medium confidence | Heuristic with weak signal |
| `< 0.5` | Low confidence | Inference or user-declared |

Facts with confidence < 1.0 are marked as `drafted`. An agent or user must review them with `/diagram review` before they are promoted to `accepted`.

### 7.5 Where data lives

```
~/.local/share/archctl/           # XDG data directory
├── projects/
│   └── <hash>/                  # one per project (hash of Git remote)
│       └── architecture.lbdb     # the LadybugDB graph

~/.config/archctl/               # XDG config directory
├── config.toml                 # global configuration
├── plugins/                    # installed plugins
└── taps/                       # plugin tap definitions
```

---

## 8. Skills and Agents

This section explains the agent system for users who want to understand how it works internally.

### 8.1 Agent hierarchy

```
diagram-architect (orchestrator)
├── c4-modeler
│   └── skill: c4-from-graph
├── uml-modeler
│   ├── skill: class-view-from-graph
│   ├── skill: sequence-from-scenario
│   └── skill: use-cases-from-graph
├── architecture-evidence
│   └── skill: architecture-discovery
└── diagram-reviewer
    └── skill: diagram-review
```

### 8.2 diagram-architect

The orchestrator. Receives requests like `/diagram c4 container payments`, decides what to extract and project, delegates to specialists, and ensures evidence backs every claim.

Key rules:
- Never reads source files directly (only `archctl` does)
- Never invents relationships (only what evidence supports)
- Never writes to the graph directly (only `archctl` does)

### 8.3 Skills reference

| Skill | Triggered by | What it does |
|---|---|---|
| `architecture-discovery` | `discover`, `explain`, `evidence` | Extracts facts and returns evidence |
| `c4-from-graph` | `c4 *` | Projects C4 views from the graph |
| `class-view-from-graph` | `class` | Projects UML class diagrams |
| `sequence-from-scenario` | `sequence` | Projects UML sequence from call chains |
| `use-cases-from-graph` | `usecase` | Projects UML use case diagrams |
| `diagram-review` | `review` | Validates diagram against graph |

### 8.4 How the `/diagram` command works

When you type `/diagram c4 container payments`:

```
1. OpenCode matches "c4" → routes to diagram-architect
2. diagram-architect parses "container payments"
3. diagram-architect calls c4-modeler
4. c4-modeler calls skill: c4-from-graph
5. c4-from-graph calls archctl diagram export c4-container --selector container:payments
6. archctl reads from LadybugDB and produces a JSON bundle
7. The bundle is rendered in archview
8. diagram-architect requests diagram-review to validate
9. Result returned to user
```

---

## 9. The Workbench

### 9.1 What is archview

`archview` is the interactive web UI for exploring architecture diagrams. It is embedded in the `archctl` binary and served locally via `archctl view`.

### 9.2 Starting the workbench

```bash
archctl view
```

This:
1. Starts an HTTP server on `127.0.0.1:<port>`
2. Opens your default browser at that URL
3. Loads the archview SPA from embedded assets (no network required)

### 9.3 Features

| Feature | Description |
|---|---|
| **Pan / Zoom** | Mouse drag to pan, scroll to zoom |
| **Node inspection** | Click a node to see its evidence (`file:line`) |
| **Filtering** | Filter nodes by kind (container, component, etc.) |
| **Layouts** | Switch between dagre, force-directed, and indent layouts |
| **Search** | Find nodes by label |
| **Export** | Export current view as SVG or PNG |

### 9.4 Architecture panel

When you click a node, the **inspector panel** shows:
- Element ID and kind
- Label and description
- Evidence references (clickable `file:line` links)
- Incoming and outgoing relationships

### 9.5 Bundle endpoint

The workbench fetches diagram data via:

```
GET /api/export?selector=c4-container:payments
```

This runs `archctl diagram export c4-container --selector container:payments` server-side and returns the bundle JSON.

---

## 10. Troubleshooting

### `archctl: command not found`

The binary is not in your PATH. Add `~/.local/bin` to your `PATH`:

```bash
# Bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### `archctl doctor` reports missing tools

These tools are optional. If missing, certain features are disabled:

| Missing tool | Affected feature | Fix |
|---|---|---|
| `ast-grep` | Multi-language code extraction (see [docs/CAPABILITIES.md](../docs/CAPABILITIES.md)) | `cargo install ast-grep` |
| `tree-sitter-graph` | Advanced extraction | `cargo install tree-sitter-graph` |
| Java | PlantUML rendering | `sudo apt install default-jre` |
| PlantUML | PlantUML export | [plantuml.com/download](https://plantuml.com/download) |

### `archctl view` won't open in browser

Some terminal configurations block browser opening. Manually open the URL shown in the output:

```bash
archctl view
# Server running at http://127.0.0.1:4321
# Open http://127.0.0.1:4321 in your browser
```

### Empty diagram / no elements found

The graph may be empty. Run extraction first:

```bash
archctl project resolve
archctl code call-graph
archctl diagram export c4-context
```

### Stale evidence after refactoring

Evidence points to lines that moved or were deleted. Run re-extraction:

```bash
archctl code call-graph
archctl evidence list --element <id>
# Check for "MISSING FILE:line" warnings
```

### Version conflict between projects

Use per-project version pinning:

```bash
# In the project root
echo "1.35.0" > .arch-version

# Or via environment variable
ARCHCTL_VERSION=1.35.0 archctl diagram export c4-context
```

### `archctl self update` fails

Check network connectivity and GitHub API access:

```bash
archctl self update --check
```

If behind a proxy, set the `HTTPS_PROXY` environment variable.

### IDE integration not working

Run diagnostics:

```bash
archctl ide doctor opencode
```

Check that the skills directory exists:

```bash
ls ~/.config/opencode/skills/ | grep -E "architecture|c4|diagram"
```

### Build from source fails

Ensure Rust ≥ 1.91 is installed:

```bash
rustc --version  # must be ≥ 1.91
cargo --version
```

If using an older Rust version:

```bash
rustup update stable
rustup default stable
```

---

## Appendix A: External Dependencies (Optional)

| Tool | Required for | Install |
|---|---|---|
| Java (JRE) | PlantUML rendering | `sudo apt install default-jre` |
| PlantUML | PlantUML diagram output | Download from plantuml.com |
| ast-grep | Rust + TypeScript extraction | `cargo install ast-grep` |
| tree-sitter-graph | Advanced structural extraction | `cargo install tree-sitter-graph` |

---

## Appendix B: Configuration Reference

`archctl` stores configuration in `~/.config/archctl/config.toml`:

```toml
[view]
default_port = 4321          # port for archctl view
default_layout = "dagre"     # layout: dagre, force, indent

[rendering]
default_backend = "mermaid"  # mermaid, plantuml, svg
svg_font_family = "IBM Plex Sans"

[extraction]
max_file_size_kb = 10240      # skip files larger than this
include_tests = false         # include test files in extraction

[evidence]
min_confidence = 0.5         # minimum confidence to auto-accept
```

---

## Appendix C: Selector Grammar

Selectors filter which elements appear in a view:

```
container:payments         ← elements with ID containing "payments"
kind:container             ← all containers
kind:component,kind:function  ← union of two kinds
container:payments,kind:external  ← intersection
```

---

## Appendix D: Bundle Format

The JSON bundle produced by `archctl diagram export --format json`:

```json
{
  "schemaVersion": "1.0",
  "baseRevision": "blake3:abc123...",
  "projection": {
    "kind": "c4-container",
    "selector": "container:*",
    "nodes": [...],
    "edges": [...],
    "metadata": {}
  },
  "evidence": [...],
  "styles": [...]
}
```

The `schemaVersion` follows semver. `baseRevision` is the blake3 hash of the project state at extraction time — used to detect drift.

---

## Appendix E: Glossary

| Term | Definition |
|---|---|
| **Canonical graph** | The single source of truth for all architecture facts (LadybugDB) |
| **Projection** | A deterministic view of the graph (C4, UML, etc.) |
| **Evidence** | `file:line` pointer proving a fact exists in source code |
| **Confidence** | Score 0.0–1.0 indicating extraction certainty |
| **Selector** | Filter expression for scoping a view |
| **Extractor** | Tool that walks source code and produces graph facts |
| **Workbench** | Interactive browser UI (archview served by archctl) |
| **Bundle** | JSON snapshot of a projection + evidence + styles |
| **LadybugDB** | Embedded graph database (SQLite-backed) |

---

## Appendix F: Related Documentation

| Document | Topic |
|---|---|
| [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md) | Complete graph data model |
| [`docs/adr/ADR-038-one-product-five-invariants.md`](docs/adr/ADR-038-one-product-five-invariants.md) | Product identity and invariants |
| [`docs/adr/ADR-033-archctl-view-embedded-workbench.md`](docs/adr/ADR-033-archctl-view-embedded-workbench.md) | Workbench embedding design |
| [`docs/adr/ADR-004-persistencia-externa-xdg.md`](docs/adr/ADR-004-persistencia-externa-xdg.md) | XDG persistence design |
| [`docs/adr/ADR-005-ladybugdb-grafo-canonico-y-evidencias.md`](docs/adr/ADR-005-ladybugdb-grafo-canonico-y-evidencias.md) | Graph + evidence model |
| [`docs/adr/ADR-039-renderer-reality-anti-roadmap.md`](docs/adr/ADR-039-renderer-reality-anti-roadmap.md) | Renderer stack and deferred decisions |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Product roadmap with H0–H3 horizons |
| [`docs/STATE.md`](docs/STATE.md) | Current shipped state and capabilities |
