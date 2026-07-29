# ADR-0008: Supply-Chain Pinning / Sandbox Policy

- **Status**: Accepted
- **Date**: 2026-07-29
- **Decides**: How untrusted external code (skills, CLIs) is admitted and confined.
- **Accepted by**: orchestrator, per user directive on 2026-07-29.

## Context

The platform *adapts* external skills (lmammino/c4, cheriftj, bitsmuggler) and runs external
CLIs. These are community-authored code of unknown provenance and changing content. The
analyzed repository is treated as **untrusted read input**, and XDG is the **trusted store**.
Without a supply-chain policy, a wrapped skill could exfiltrate source, write into the
analyzed repo, or silently change behaviour on update.

The source doc also stores source snippets by default — which, combined with shared export
bundles, is a confidentiality risk for closed-source repos.

## Decision

A layered supply-chain and confinement policy:

| Control | Implementation |
|---|---|
| **Pinning** | `skills.lock.json` records a commit/SHA per external skill; `archctl skills verify` validates the upstream hash |
| **License** | Explicit license check before activation. **Allowed list (v1):** MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, MPL-2.0 (with file-level exception acknowledgement), and Unicode-DFS-2016 (for fonts). Anything else is blocked at activation; the operator can explicitly allow a non-listed license via `archctl skills allow-license <SPDX>` and that decision is logged in the activity audit. |
| **Sandboxing** | Wrapped skills cannot write outside XDG; enforced by the plugin write-guard (`tool.execute.before`) **and** config `permission` rules (belt-and-suspenders) |
| **Snippet confidentiality** | `store-source-snippets: false` by default (per-project override); the ledger stores only path/lines/commit/hash, never the code text |
| **Offline render** | Local renderers only; public Kroki/PlantUML servers blocked (ADR-0005) |
| **CLI allowlist** | The Extractor's bash is an allowlist (`ast-grep`, `ctags`, `git`, build tools), not free shell |
| **Promotion gate** | Any external-skill update requires a fixture re-test before it is promoted into the lock |

**Write-guard scope:** the plugin hook + config rules reject any write outside
`$ARCHCTL_PROJECT_DIR` for archctl agents. Containment **resolves canonical paths**: the
target and the allowed root are both `realpath`-resolved through symlinks, and any target
whose canonical path escapes the allowed root is **rejected** (symlink-escape defense).
Writes use **atomic temp+rename where supported** (write to a sibling temp file inside the
allowed root, then `rename()`) so a partial write can never land outside confinement. If
either guard layer fails open, the other still holds.

**Local MCP / tool-executable inventory:** local MCP servers and tool executables the
pipeline invokes are **untrusted code, same as skills** — they join the pin/version/license
inventory alongside `skills.lock.json` (version +, where feasible, hash + license per
executable). This scope covers the plugin-first MVP; it is not a speculative enterprise
platform.

**Export-bundle safety:** export archives contain model + evidences (no source snippets by
default) plus the `skillset.lock`, so a bundle is shareable without leaking code.

## Consequences

- **Positive**: Malicious or rotted skills are pinned and confined; closed-source repos are
  not exfiltrated via snippets or public renderers; rollback is clean (XDG deletion).
- **Negative**: Skill updates require a manual promote-and-retest step (intentional friction);
  `store-source-snippets: false` means some deep auditing must re-read the file at its path.
- **Neutral**: The trust boundary is explicit — repo = untrusted read, XDG = trusted store,
  external skills = untrusted code, pinned.

## Alternatives considered

- **Trust external skills by default** — rejected: unknown provenance, mutating content; an
  architecture tool that ingests untrusted code must not run untrusted code unsandboxed.
- **Store source snippets by default** — rejected: confidentiality risk for closed-source
  repos and shared bundles; the path/lines/hash triplet is sufficient for traceability.
- **Single write-guard layer only** — rejected: a single hook failing open would breach the
  boundary; defense in depth (hook + config permission) is required.
