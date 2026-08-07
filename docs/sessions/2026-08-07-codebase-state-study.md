# `archctl` codebase state study + improvement proposals (M55)

> **Author**: post-session-close audit, 2026-08-07
> **Source of truth**: v1.24.0 (`361e52c`)
> **Cycle**: CYC-2026-08-07-m55-codebase-state-study-and-roadmap-proposals
> **Status**: study-only; no code changes

This document captures the **current state** of `archctl` at the close of
the M0–M54 marathon session, and proposes **bounded, prioritized improvements**
for future cycles. The goal is to give the next session a concrete starting
agenda instead of an open-ended "what should we do?".

---

## 1. Current state (snapshot at v1.24.0)

### 1.1 Codebase size

| Region | Lines |
|---|---|
| `archctl/src/**/*.rs` | **31,254** |
| `archctl/tests/**/*.rs` | **6,560** |
| `archctl/benches/**/*.rs` | **790** |
| Total Rust | **~38,600** |

- **30 source modules** organized in 8 bounded contexts (cli, code, cognitive, diagram, render, store, graph, etc.).
- **201 tests** (unit + integration).
- **8 bounded contexts** with manifest gates (`manifests/*.toml`).
- **26/26 doctor scopes pass.**

### 1.2 Largest files

| Lines | File | Note |
|---|---|---|
| 2,383 | `src/store.rs` | GraphStore port + LbugStore adapter + 2,300 lines of trait/impl + tests. The single biggest file in the codebase. |
| 2,308 | `src/cli.rs` | clap dispatch for all subcommands. Bound to grow as new commands land. |
| 2,276 | `src/code/call_graph.rs` | Call-graph extractor + applier. |
| 1,572 | `src/code/class_diagram.rs` | Class diagram extractor + applier. |
| 1,397 | `src/code/state_machine.rs` | State machine extractor + applier. |
| 1,313 | `src/evidence.rs` | Evidence persistence. |
| 1,081 | `src/scope.rs` | Manifest gate engine (doctor). |
| 1,004 | `src/code/c4_discover.rs` | C4 vertical extraction. |

**Observation**: store.rs (2,383 LOC) is the single biggest file. It contains:
- 4 traits (`GraphStore`, `EvidenceOps`, `SourceOps`, `DiagramOps`).
- 1 adapter (`LbugStore`).
- ~150 lines of M51 prep/exec plumbing.
- ~400 lines of `row` conversion helpers.
- ~700 lines of test code.

**Risk**: a single 2,400-LOC file is hard to navigate. Future refactors might split it.

### 1.3 TODO/FIXME count

```
archctl/src/code/strategies/dockerfile.rs:139:    // TODO: read the Dockerfile and parse LABEL org.opencontainers.image.title
archctl/src/code/class_diagram.rs:1067:        members: Vec::new(), // TODO: Python methods (block/function_definition inside class)
```

Only **2** TODO markers. Both are explicit deferrals with no time pressure. Acceptable.

### 1.4 Test coverage

- **201 tests** total.
- Largest test files:
  - `tests/code_class_diagram.rs` — 859 LOC.
  - `tests/c4_components_integration.rs` — 669 LOC.
  - `tests/code_c4_discover.rs` — 590 LOC.
  - `tests/diagram_project_integration.rs` — 520 LOC.
- Unit tests are inline in source files (`src/code/call_graph.rs` has 6 test functions, etc.).
- E2E tests in `archctl/tests/` exercise full projector + backend chains.

**Observation**: e2e coverage is heavy on diagram/render pipelines. The `code::*` extractors have good test counts. **Gap**: the cognitive layer (`archctl/src/cognitive/`) has minimal test coverage — M55 audit didn't find a test count for it. Worth auditing.

### 1.5 Public surface area

`manifests/*.toml` declares **~250+ public symbols** across 26 scopes. Top contributors:

- **code**: 40 public_symbols (call_graph, class_diagram, state_machine, c4_discover, sequence, etc.).
- **diagram**: 20 public_symbols (export, apply, validate, project, etc.).
- **cli**: 16 public_symbols (clap enum variants).
- **row**: 16 public_symbols.
- **evidence**: 14 public_symbols.
- **skills**: 11 public_symbols.

**Observation**: archctl exposes a stable, documented public surface. The manifest gate ensures the surface stays accurate.

### 1.6 Recent activity (last 21 cycles, v1.4.1 → v1.24.0)

- 21 cycles closed.
- 42 PRs merged (this repo).
- 70+ commits.
- 25 tags.

Patterns observed:
- Each cycle = 1 code PR + 1 docs PR (ROADMAP companion) + 1 tag + 1 vault milestone update.
- Cycle duration: 5–15 minutes including vault/ROADMAP/PR overhead.
- 1 open PR (M23 Action Proposal, from 2026-08-04, separate concern).

### 1.7 Open PRs

```
#32 feat(cognitive): expand ActionProposal v1.0 — types + backward compat (M23 Phase 1/6)
   branch: feat/m23-action-proposal-policy-phase1
   status: OPEN (since 2026-08-04)
```

This is a **stale open PR** (3+ days open at session close). Worth checking in next session: merge, close, or update.

### 1.8 ADRs status

- **37 ADRs** total (ADR-000 through ADR-037).
- Most ADRs are **accepted**.
- **Open/proposed** ADRs: 5 (014, 017, 023, 027, 036).
- Open ADR 014 (SparrowDB adapter) — still on the roadmap but no SparrowDB adapter exists.

### 1.9 Performance baseline (from ADR-036 + benches)

| Pipeline | Threshold | Status |
|---|---|---|
| `archctl diagram export` (graph < 10K nodes) | < 2s p99 | ✅ met |
| Cold start of binary | < 100ms | ✅ met |
| RSS idle | < 50MB | ✅ met (typical 144MB) |
| Bundle export (C4 standard) | < 1MB | ✅ met |
| Call-graph apply `echo` (1307 elements) | < 10s (D1) / < 3s (D1+D2) | ✅ met |
| Call-graph apply `zustand` (212 elements) | < 5s | ✅ met |

Performance is on target. The M32 D1+D2+D3+D4+D5 chain (transactions, bulk UNWIND, prepared statements, doc fixes, sibling writers) is fully shipped (ADR-036 complete).

### 1.10 Test ergonomics

- `bash scripts/verify-local.sh` is the canonical pre-merge gate. Cheap mode runs `archctl doctor --scopes code` + clippy.
- `cargo test --quiet`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` are required green.
- All gates pass at v1.24.0.

### 1.11 Documentation

- **27 milestone nodes** in vault (one per cycle).
- **12 spec files** in `docs/specs/`.
- **37 ADRs** in `docs/adr/`.
- **CHANGELOG.md** fully up to date (v1.4.1 → v1.24.0).
- **ROADMAP.md** has 21 cycle entries.
- **`docs/Librerías-visualización-grafos-BI.md`** remains **untracked** (per AGENTS.md invariant).

### 1.12 Open architecture questions

- ADR-013 (viewer orthogonal) — archview is a separate repo (`/var/home/rubentxu/Proyectos/agentesIA/archview`, v0.21.3, M17 series complete). Next: M18 reactive runtime or M19 wgpu renderer (2.0 horizon).
- ADR-014 (SparrowDB adapter) — `SparrowStore` not implemented. Optional.
- ADR-018 (reactive runtime), ADR-019 (custom wgpu renderer) — defer to 2.0 horizon.

---

## 2. Improvement proposals (prioritized)

Each proposal is **bounded** (single cycle or short series), **justified**
(cites specific code/test/data), and **traceable** (links to a milestone
or ADR when one exists).

### 🟢 Quick wins (single bounded cycle each, M56–M60)

#### M56 — DRY the skip-on-missing-backend helper

**Problem**: 5 e2e test files (`plantuml_render_e2e.rs`, `usecase_view_plantuml_e2e.rs`,
`sequence_view_plantuml_e2e.rs`, `state_view_plantuml_e2e.rs`, `c4_view_plantuml_e2e.rs`)
all duplicate the same `backend_available()` helper.

**Scope**: 1 file extracted (e.g. `archctl/tests/common/plantuml_backend.rs`), 5 import
sites updated. Net: -50 LOC.

**Risk**: low. Pure refactor.

#### M57 — CONTRIBUTING.md with manifest hygiene conventions

**Problem**: The `gate_public_symbols_exist` check has a documented blind spot (enum
variants, struct fields, removed functions). M46 fixed 26 stale entries but the
convention isn't documented.

**Scope**: 1 file, ~80 lines. Documents:
- What `public_symbols` covers.
- What `must_hold` covers.
- When to add an entry vs leave blank.

**Risk**: low. Pure docs.

#### M58 — `docs/specs/index.md` table-of-contents

**Problem**: 12 spec files in `docs/specs/` but no index. `docs/README.md` has a
"View specs" section but it's not exhaustive.

**Scope**: 1 file, ~50 lines. Each spec listed with one-line summary + audience.

**Risk**: low. Pure docs.

#### M59 — Close stale PR #32 (M23 Action Proposal phase 1)

**Problem**: PR #32 has been OPEN since 2026-08-04. Either merge, close, or update.

**Scope**: 0–1 PR action. Either:
- Review the diff and merge if green.
- Comment with the outcome of M55 audit (current state) and ask whether the
  PR is still aligned.

**Risk**: medium. PR may have merge conflicts with main after 4+ days.

#### M60 — Fix the 2 TODO markers

**Problem**: `src/code/strategies/dockerfile.rs:139` and `src/code/class_diagram.rs:1067`
both have explicit TODO comments. Either implement or remove.

**Scope**: 0–2 file changes. Each is bounded.

**Risk**: low. Pure cleanup.

---

### 🟡 Medium-effort (1–3 cycles each, M61–M65)

#### M61 — Add a cognitive-layer test audit

**Problem**: The cognitive layer (`archctl/src/cognitive/`) has 14 sub-modules
(agents, audit, context, delta, descriptor, dispatcher, escalation, event, mcp,
mod, observer, output, policy, subscriptions) but minimal test coverage.

**Scope**: 1–2 cycles. Audit + add unit tests for the most-critical cognitive
components (likely `policy/`, `dispatcher/`, `escalation/`).

**Risk**: medium. Cognitive layer may have implicit contracts not documented in
the source.

#### M62 — Add a `docs/STATE.md` refresh

**Problem**: `docs/STATE.md` is dated 2026-08-07 (last touched in v1.1.0). After 21
cycles, the "current state" section is stale.

**Scope**: 1 cycle. Update STATE.md to reflect v1.24.0.

**Risk**: low. Pure docs.

#### M63 — Split `src/store.rs` (2,383 LOC)

**Problem**: `store.rs` is the biggest file in the codebase. It contains the GraphStore
port, the LbugStore adapter, M51 prepare/exec plumbing, and 700+ lines of row
conversion helpers. Splitting would improve navigability.

**Scope**: 2–3 cycles. Suggested split:
- `src/store/mod.rs` — GraphStore trait + sub-traits.
- `src/store/lbug.rs` — LbugStore adapter + prepare/exec.
- `src/store/convert.rs` — lbug_value → Cell, Row, etc.
- `src/store/error.rs` — StoreError enum.

**Risk**: medium. Many imports reference deep paths. Mechanical refactor.

#### M64 — Add CONTRIBUTING.md + add pre-push hook for new docs

**Problem**: There's no CONTRIBUTING.md. New contributors don't know:
- How to write a cycle (vault lock + milestone + ROADMAP entry).
- Manifest hygiene conventions.
- How to run the test suite cheaply.

**Scope**: 1 cycle. Add the file + reference it from AGENTS.md.

**Risk**: low. Pure docs.

#### M65 — Investigate the ADR-018/019 2.0 horizon

**Problem**: Reactive runtime (M18, ADR-018) and wgpu renderer (M19, ADR-019) are
2.0-horizon features with no current design work. Worth a design spike.

**Scope**: 1 cycle for a design spike (no code). Document the architecture in
an ADR or design doc.

**Risk**: low. Design only.

---

### 🔴 Longer-term (3+ cycles each, M66+)

#### M66 — Migrate `call_graph::apply` to use `prepare/execute` (M51 deferred)

**Problem**: M51 wired the port but call_graph::apply still uses per-element
`store.query(&cypher)`. Expected 2–5x perf on top of M32 D1+D2.

**Scope**: 2–3 cycles. Either:
- Use typed `lbug::Value::String(...)` bindings (not JSON-wrapped).
- Or `CAST($id AS STRING)` in MATCH clauses.

**Risk**: medium. Need to handle the lbug JSON-vs-typed-value quirk.

#### M67 — Implement SparrowDB adapter (ADR-014)

**Problem**: ADR-014 documents the port + SparrowDB as a 2nd adapter. No
`SparrowStore` exists.

**Scope**: 3–5 cycles. Substantial work.

**Risk**: high. SparrowDB crate maturity is unclear (ADR-014 says `sparrowdb = "0.1.16"`).

#### M68 — Migrate archview to v0.22.0 (separate repo)

**Problem**: archview (`/var/home/rubentxu/Proyectos/agentesIA/archview`) is at v0.21.3
(M17 series). Could continue with M18 (reactive runtime) or M19 (wgpu renderer).

**Scope**: separate repo, separate cycles. Out of scope of `archctl` here.

---

## 3. Recommended next-cycle agenda

If the next session picks 1–3 cycles to execute, I'd recommend:

1. **M56 (DRY skip-on-missing-backend)** — fastest win, removes 50 LOC.
2. **M59 (close PR #32)** — resolves a 4-day stale PR.
3. **M62 (STATE.md refresh)** — keeps the docs in sync with v1.24.0.

These 3 together = ~30 minutes, pure cleanup, no behavior change. A good
"tidy up after the marathon" session.

If more ambitious work is desired:

4. **M61 (cognitive-layer test audit)** — adds test coverage where it's most needed.
5. **M66 (call_graph prepare/execute migration)** — closes the M51 deferred work.

---

## 4. Anti-patterns to avoid (learned from this session)

For future cycles:

1. **Don't ship a verification-only cycle without first `git grep`-ing what already
   exists.** M52 and M53 both reduced scope because the work was already done.
2. **Don't assume a "writer" function exists.** M53 found sequence.rs is read-only.
   Always `grep "pub fn apply"` before planning migrations.
3. **Don't refactor without a test that proves the new design is correct.** M39
   found a 10-cycle latent bug because substring unit tests masked the real
   bug. End-to-end tests (real backend → SVG) catch what substring tests can't.
4. **Don't add public_symbols entries for enum variants or struct fields.** They
   will fail doctor. Use `must_hold` for design-intent checks.

---

## 5. References

- v1.24.0 commit: `361e52c`.
- ROADMAP.md: 21 cycle entries.
- CHANGELOG.md: v1.4.1 → v1.24.0.
- Engram session summary: `arch-stack/session-close-2026-08-07-m54`.
- Vault: `~/.sddk-knowledge/arch-stack/milestones/` (27 nodes).
- ADR list: `docs/adr/ADR-000-...` to `docs/adr/ADR-037-...`.