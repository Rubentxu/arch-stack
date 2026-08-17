# ADR-060 — `architecture` CLI surface: drop the `snapshot` intermediate (Path B deviation)

> **Cycle:** `p-38e02210a9f14317/p2-02-followup`
> **Status:** Draft (spec phase, pending apply-phase acceptance)
> **Date:** 2026-08-17
> **Baseline:** `f34fee0` (feat/p2-02-followup HEAD)
> **Resolves:** p2-01 WARNING #1 (CLI surface deviation in `verify-report.md:97`)
> **Related:** ADR-045 (capability registry single source of truth),
> `docs/arch-stack-proposals-2026-08-13/specs/architecture-snapshots.md` (line 21 — to be edited)

## Contexto

p2-01 (Snapshot MVP) shipped the `architecture` subcommand with handlers named
`architecture_snapshot_create_cmd`, `architecture_snapshot_list_cmd`,
`architecture_snapshot_gc_cmd` (`archctl/src/cli.rs:2476-2592`) wired to
`Command::Architecture { ArchitectureAction::{Create,List,Gc} }` via dispatch
(`archctl/src/cli.rs:800-827`). The proposed spec
(`docs/arch-stack-proposals-2026-08-13/specs/architecture-snapshots.md:21`)
documented the public surface as
`archctl architecture snapshot {create,list,gc}` — a `snapshot` intermediate
between `architecture` and the action verb.

p2-01's apply phase collapsed the intermediate to ship the MVP:
`archctl architecture {create,list,gc}` was the actual CLI. The deviation went
unnoticed until p2-01's verify-report flagged it at line 97:

> "User-facing invocation `archctl architecture snapshot list` returns
> `error: unrecognized subcommand 'snapshot'`."

Two paths to resolve:

- **Path A — Literal spec compliance.** Introduce `SnapshotAction` enum,
  restructure `ArchitectureAction` and dispatch in `cli.rs`. Public CLI changes
  from `archctl architecture {create,list,gc}` to
  `archctl architecture snapshot {create,list,gc}`. ~50 LOC, 1 file.
- **Path B — ADR-documented deviation.** Update the spec line 21 to match the
  shipped CLI. Document the decision here. 0 LOC; 1-line spec edit + this ADR.

Path B is adopted (see `proposal.md:10–11`, `explore-report.md:60-62`).

## Decisión

### D1 — Final public surface is `archctl architecture {create,list,gc}`

The `snapshot` intermediate is **not** added. The `architecture` top-level
subcommand exposes the three actions directly:

```
archctl architecture create   # creates a new snapshot
archctl architecture list     # lists snapshots (with optional filters)
archctl architecture gc       # garbage-collects old snapshots (dry-run by default)
```

### D2 — Spec line 21 is updated to match

`docs/arch-stack-proposals-2026-08-13/specs/architecture-snapshots.md:21`
changes from:

> `archctl architecture snapshot create/list/gc`; SnapshotRepository.

to:

> `archctl architecture create/list/gc`; SnapshotRepository.

Single-line text edit, no scenario rewrite. The three p2-01 Given/When/Then
scenarios (same tuple → idempotent; incompatible schema → diff rebuild/migration;
GC → pins remain) remain valid because they reference the actions, not the path
shape.

### D3 — Handler naming convention is preserved

The internal handlers keep the `architecture_snapshot_*` naming
(`architecture_snapshot_create_cmd`, etc.). The name is historical and
describes the **domain** (snapshot), not the **path**. Renaming them is
out-of-scope; no behavioral gain; churn.

### D4 — Manifest gate updated to match

`manifests/cli.toml` (`public_symbols`, `must_hold`) gains
`Command::Architecture` and `ArchitectureAction`. **No** `SnapshotAction` is
added because no such type exists after Path B.

## Rationale

### R1 — Public CLI surface stability

p2-01 shipped v1.49.0 with `architecture {create,list,gc}`. Any user with
scripts invocations on the current shape would break under Path A. Mid-release
breaking changes for zero behavioral gain violate
ADR-019 (performance budget discipline) by creating churn without value, and
the AGENTS.md "Compromisos deliberados" rules call out "renderers locales,
nunca públicos por defecto" as a similar pattern of "don't churn user-facing
contracts without a real reason."

### R2 — Precedent: top-level actions expose verbs directly

All other `archctl` top-level commands follow this pattern:

| Command | Shape | Source |
|---|---|---|
| `archctl doctor` | top-level + flags | `cli.rs:558-565` |
| `archctl evidence put\|list\|accept\|supersede` | top-level + verbs | capability registry |
| `archctl inventory` | top-level + flags | capability registry |
| `archctl render plantuml\|structurizr` | top-level + renderer name | `cli.rs:120-166` |
| `archctl architecture {create,list,gc}` | top-level + verbs | this ADR |

The `snapshot` intermediate would be the **only** `archctl` subcommand to nest
an action one level below a noun that is already the top-level subcommand.
The `snapshot` concept lives inside `architecture/snapshot.rs`; promoting it
to the CLI surface creates an asymmetry with the codebase structure.

### R3 — The handlers' `architecture_snapshot_*` naming tells the story

The p2-01 author wrote `architecture_snapshot_create_cmd` etc. — keeping
`snapshot` as a domain marker in the function name while collapsing the CLI
intermediate. This is consistent with how the codebase treats `snapshot` as
the implementation noun (module path `architecture/snapshot.rs`,
`SnapshotRepository`, `Snapshot` struct in `store.rs`). The CLI top-level
`architecture` is the noun; the action is the verb; the implementation noun
is `snapshot`.

### R4 — Idempotency across cycles

Path A would force every existing user invocation
(`archctl architecture create`) into a deprecation window. Path B requires no
deprecation because the CLI did not change between p2-01 and p2-02.

## Alternatives considered

### Path A (literal spec compliance)

| Aspect | Impact |
|---|---|
| Public CLI change | Breaking — `archctl architecture create` → `archctl architecture snapshot create` |
| Code change | `cli.rs` dispatch restructure + new `SnapshotAction` enum |
| Manifest change | `cli.toml` gains `SnapshotAction` |
| Test change | Integration tests rewrite command shapes |
| User cost | High — script rewrites required |

**Rejected** — zero behavioral gain; high user cost; violates CLI-surface
stability principle.

### Path C (Path B + global `UnitOfWork` refactor)

Could be combined with WARNING #4's deeper fix (refactor `UnitOfWork` to
remove `use crate::store::LbugStore` from all 6 bounded contexts). This would
touch `archctl/src/{diagram,code,architecture}/*.rs` and is **out-of-scope** for
p2-02 per `explore-report.md:30, 274`. Tracked as a future cycle candidate.

**Rejected for p2-02** — scope creep.

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Future cycle wants to nest `snapshot` anyway | Low | Path B does not preclude a future nested redesign via a new ADR |
| Spec drift (text vs code) recurs | Low | Capability registry (`cli.architecture`) + manifest gate (`Command::Architecture`) catch future divergences |
| Users with `architecture snapshot <verb>` invocations in scripts (the verify-report failure mode) | Very Low | Failure was already the as-shipped state; no users could be relying on it |

## Rollback

Revert this ADR via a new ADR-XXX proposing Path A. The single-line spec edit
in D2 is trivial to revert. No data migration, no schema bump, no API
deprecation.

## Open follow-ups

- **WARNING #4 asymmetry.** `manifests/architecture.toml` gains
  `must_not_contain = ["use crate::store::LbugStore"]` (apply phase). The other
  5 bounded contexts (`diagram/apply.rs`, `code/{state_machine,c4_discover,
  class_diagram,call_graph}.rs`) keep the import. Intentional asymmetry; the
  architecture module is the strictest. Documented in the apply commit message.
- **T5.2 vault specs** (`docs/specs/{architecture-snapshot,snapshot-repository,
  architecture-cli}.md`) deferred from p2-01 and not picked up by p2-02.
  Tracked separately.