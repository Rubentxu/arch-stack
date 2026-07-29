# ADR-0003: XDG Runtime State + Explicit Export Bundle

- **Status**: Accepted
- **Date**: 2026-07-29
- **Decides**: Where durable architecture state lives and how it is shared.
- **Accepted by**: orchestrator, per user directive on 2026-07-29.

## Context

The source design document contains **two unreconciled storage designs**: an in-repo
`.architecture/` directory (which pollutes Git and rots) and an XDG `~/.local/share/`
location (clean but not directly shareable). The user's explicit constraint is "don't
pollute Git." Storing durable knowledge only in conversation history is also rejected — it
is lost on compaction and is not queryable.

A second contradiction (coherence gate 1.5): the earlier wording derived project identity
solely from Git (`BLAKE3(remote + root_commit)`) and therefore made Git a *universal*
prerequisite — yet the planning workspace for this very project has no Git. Identity must
support non-Git directories, Git must be an *optional enrichment* adapter rather than a
product gate, and export/import must stay portable across machines where neither the Git
remote nor the local `realpath` will match.

## Decision

**Default: durable runtime state in XDG.** `~/.local/share/archctl/projects/<id>/` holds
the evidence ledger, IR, and audit reports. The analyzed repository is **never** written to.

Sharing is explicit, via an **export bundle**: `archctl project export` produces an archive
(model + evidences, with `store-source-snippets: false` so no sensitive source code, plus a
`skillset.lock`). For teams, a separate sidecar Git repo (`<project>-architecture`) is the
curated shareable form.

**Project identity is a discriminated `SourceIdentity`.** Git is an *optional capability
adapter for richer history*, not a universal product prerequisite — the platform works on
plain directories (including this planning workspace, which has no Git).

```
SourceIdentity =
  | { type: "git",       repositoryId: BLAKE3(normalized_remote + root_commit),
                       worktreeId:   BLAKE3(repositoryId + realpath(show_toplevel)) }
  | { type: "directory", directoryId:  BLAKE3(canonical_realpath) }   // LOCAL-ONLY stability
```

- **Git mode**: stable and *sharable* — `repositoryId` is identical for the same repo on
  any machine. A branch is **not** part of identity (a worktree may change branches).
- **Directory mode**: `directoryId = BLAKE3(canonical_realpath)` is stable **only on one
  machine** (realpath differs across hosts). It exists so non-Git workspaces are first-class,
  not an error. It is explicitly **not portable** without an explicit rebind.

**Export / import (portability).** Because identity anchors are machine-specific
(`realpath`) or remote-specific (`normalized_remote`), a bundle carries a **portable
projectId** — a stable UUIDv4 assigned at first export (deterministic from
`SHA256(SOURCE_IDENTITY_CONTENT + firstExportTimestamp)`), decoupled from the local anchor.
On import, the importer recomputes the local `SourceIdentity` and **explicitly rebinds** it
to the portable `projectId`. Import is never a silent identity match; the rebind is recorded.

**Rebind collision policy:** if the recomputed local `SourceIdentity` already corresponds to
a *different* portable `projectId` in the local store, the import is **rejected by default**
and the user is asked to choose one of three explicit actions:

1. **Replace** the existing local project with the imported one (destructive; requires
   `--yes`); the previous project's `state/` and `share/` are archived under
   `~/.local/state/archctl/_archive/<previousProjectId>/`.
2. **Keep both**: the imported bundle is rebound to a *fresh* portable `projectId` so two
   unrelated local projects coexist.
3. **Abort** — neither project is modified.

The default (no flag) is **reject + ask** — the operator decides explicitly, never silently.

**Evidence source revision (discriminated).** Every evidence pins what it was observed
against via `source.revision`:

- `{ type: "git-commit", value: "<sha>" }` — when Git is present (richer, history-sharable);
- `{ type: "content-hash", value: "blake3:<snippet-range>" }` + `observedAt` snapshot
  timestamp — when there is no Git (the snapshot timestamp is the anchor).

Either way no evidence is ever left without a traceable anchor.

**Selective destruction:** deleting `~/.cache/archctl` loses nothing regenerable;
`~/.local/state/archctl` loses in-flight executions, not the model;
`~/.local/share/archctl` loses the persistent architectural memory.

## Consequences

- **Positive**: Clean analyzed repos (enterprise-friendly); non-Git directories are
  first-class (no product gate on Git); rollback = one directory deletion; no
  chat-history dependency; identity is portable across machines via the explicit rebind.
- **Negative**: Directory-mode identity (`directoryId`) is **local-only** — it does not
  survive a path move or a different host without an explicit rebind. Git-mode identity is
  sharable; directory-mode is intentionally not.
- **Neutral**: A thin plugin (`shell.env`) resolves the `SourceIdentity` and exposes
  `$ARCHCTL_PROJECT_DIR` to the agents; the write-guard (ADR-0008) confines writes there.

## Alternatives considered

- **In-repo `.architecture/`** — rejected: pollutes Git, conflicts with the user constraint,
  rots into staleness.
- **Conversation history as the store** — rejected: lost on compaction, not queryable, not
  reproducible.
- **Cloud/shared store by default** — rejected for MVP: adds an availability + privacy
  dependency before value is proven; explicit export is the safer default.
- **Require Git universally** — rejected: makes Git a product prerequisite and breaks on
  plain directories (including this project's own planning workspace). Git is richer but
  optional; in non-Git mode evidence anchors on content-hash + snapshot timestamp instead.
