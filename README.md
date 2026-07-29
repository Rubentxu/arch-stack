# `archctl`

OpenCode Architecture Diagrammer — C4 + UML diagrams by reverse-engineering
a repository, with a `archctl` CLI sidecar that owns persistence,
extraction and rendering.

> The authoritative spec is under [`docs/`](docs/).
> Start with [`docs/README.md`](docs/README.md) and
> [`docs/ROADMAP.md`](docs/ROADMAP.md) (M0 → M11).

## Status

- **M0 — Validación de OpenCode (in progress).** This scaffold ships the
  OpenCode profile and a minimal `archctl` CLI (`doctor`, `project
  resolve`, `render`). Persistent graph, full Rust binary, and the rest
  of the milestones land in M1 → M11.

## Layout

```
.
├── docs/                       v2 authoritative spec (README, ROADMAP M0–M11, ADRs, data model, schema)
├── profile/                    OpenCode profile source (installed by scripts/install.sh)
│   ├── opencode.jsonc
│   ├── agents/                 diagram-architect + 4 subagents
│   ├── commands/               /diagram dispatcher
│   ├── skills/                 c4-context, plantuml-sequence (M0 skeleton)
│   └── plugins/                archctl-env.ts (env injection)
├── archctl/                    M0 minimal CLI (TypeScript; replaced by Rust in M2)
│   ├── src/
│   │   ├── cli.ts              dispatcher
│   │   ├── doctor.ts           env check
│   │   ├── render.ts           DSL/PUML → local Kroki
│   │   └── resolve.ts          project resolve (SourceIdentity stub)
│   └── package.json
├── scripts/
│   └── install.sh              copies profile/ to $XDG_CONFIG_HOME/opencode-architecture
├── CHANGELOG.md
├── CONTEXT.md
├── ROADMAP.md                  redirect to docs/ROADMAP.md
└── .opencode-version           OpenCode pin (1.18.x)
```

## Install

```bash
./scripts/install.sh             # installs profile/ to ~/.config/opencode-architecture/
cd archctl && npm install        # archctl CLI dependencies
```

## Run

```bash
export OPENCODE_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/opencode-architecture"
opencode
```

Inside OpenCode, `/diagram c4 context` and `/diagram sequence` reach the
right subagent, which delegates to `archctl`.

Outside OpenCode:

```bash
cd archctl
npx tsx src/cli.ts doctor
npx tsx src/cli.ts render ./docs/schema/001_initial_schema.cypher
```

## Milestones

The complete milestone plan is in [`docs/ROADMAP.md`](docs/ROADMAP.md):
M0 → M11. The first useful MVP per the v2 plan is `M0 → M1 → M2 → M3 →
M4 → M5 → M6`.
