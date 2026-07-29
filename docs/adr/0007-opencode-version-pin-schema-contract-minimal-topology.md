# ADR-0007: OpenCode Version Pin / Schema-Contract and Minimal Agent Topology

- **Status**: Proposed
- **Date**: 2026-07-29
- **Decides**: OpenCode integration discipline and the agent topology size.

## Context

The exploration report flagged **OpenCode version drift** as a medium-high risk: hooks and
config keys shift across releases, and the source doc mixed `mcp` vs `mcpServers` and
treated `experimental.session.compacting` as a config key when it is actually a plugin hook.

Independent verification against the **live** `https://opencode.ai/config.json` (fetched
2026-07-27) confirms the canonical shapes for the current version:

- `mcp` is the top-level MCP key (`McpLocalConfig { type:"local", command:string[],
  environment }`) — **NOT `mcpServers`**.
- `subagent_depth` is a top-level integer (default 1).
- `skills` has `paths` and `urls`; `references` (not the deprecated `reference`); `plugin` is
  an array of `string | [string, options]`; `permission` covers `read/edit/bash/task/skill/lsp/…`.
- `compaction` is top-level (`auto/prune/tail_turns/preserve_recent_tokens/reserved`);
  `experimental` contains **no** `session.compacting` key — it is a plugin hook.
- Agents are configured via `agent` (with `mode: subagent|primary|all`, `steps`, `permission`).

The keys are confirmed **present** for the live version. Their *exact hook signatures* (event
destructuring for `tool.execute.before`, `shell.env`) are **not** guaranteed stable across
versions — that requires a pinned release + build-time verification.

Separately, the source doc proposes 9 agents. Exploration found this is a 10× scope expansion
that front-loads complexity before the core hypothesis is validated.

## Decision

**Version-pin OpenCode and enforce a schema-contract test in CI.** The CI fetches
`config.json` for the pinned version and fails if any used key (`mcp`, `subagent_depth`,
`plugin`, `skills.paths`, `references`, `permission`, agent `mode`) drifts. This converts an
invisible version risk into a loud, build-time failure.

**Always use `mcp`, never `mcpServers`.** Compaction config is top-level `compaction`;
session-compacting is a plugin hook, not config.

**Minimal agent topology: 4 roles maximum for the MVP** — orchestrator, extractor/cartographer,
synthesizer/modeler, auditor/falsifier. **Rendering is deterministic tooling, not an agent.**
`subagent_depth: 2` (orchestrator → specialist → bounded sub-task). No 9-agent fantasy by
default.

## Consequences

- **Positive**: Version drift becomes a CI signal, not a silent runtime break; the topology is
  small enough to validate cheaply and reverse easily.
- **Negative**: A schema-contract test is a maintenance artifact that must track OpenCode
  releases; hook signatures still need a pinned-release check beyond the config schema.
- **Neutral**: The 4-role topology can grow laterally (a distinct falsifier agent, Phase 4)
  via additive agent files without restructuring.

## Alternatives considered

- **9-agent topology (source doc)** — rejected for MVP: 10× scope, unvalidated complexity,
  high cohesion cost.
- **No schema-contract test, trust the live schema** — rejected: "confirmed now" is not
  "stable forever"; the drift risk is real and must be gated.
- **`mcpServers` key** — rejected: factually wrong for OpenCode; verified against the live
  schema.
