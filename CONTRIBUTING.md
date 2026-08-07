# Contributing to `archctl`

> Practical handbook for new contributors (human or AI agent).
> Single source of truth for "how do I do a cycle?".

This file is the entry point for working on `archctl`. It sits next to
[`AGENTS.md`](AGENTS.md) (which is the AI-agent contract) and
[`CONTEXT.md`](CONTEXT.md) (the project executive summary). Read this
file first; refer to `AGENTS.md` for the binding rules.

---

## 1. What is `archctl`?

A **local CLI sidecar** that assists an OpenCode agent to produce C4
and UML diagrams from a repository. It persists, queries, normalizes,
and projects an architecture graph — it does **not** interpret the
architecture by itself.

Five priorities, in order:

1. **Persistence outside the repo** (ADR-004). The user's repo is
   sacred — `archctl` writes to XDG (`~/.local/share/archctl/`,
   `~/.config/archctl/`) and never to the project tree.
2. **Renderers local by default** (ADR-011). `kroki.io` and
   `plantuml.com` are **blocked** without explicit opt-in.
3. **Wrap, do not reimplement** (ADR-006). Use `ast-grep`, `ctags`,
   `terraform show -json`, etc. via `std::process::Command`.
4. **Performance** (ADR-019): `diagram export` p99 < 2s for < 10K
   nodes; cold start < 100ms; RSS < 50MB.
5. **Evidence per node and edge** (ADR-005). No claim without a
   `file:line` pointer.

---

## 2. The cycle workflow

Every change to `archctl` is a **cycle**. A cycle has 4 phases:

```
1. Code + tests       2. Verify       3. Release     4. Sync main
   (branch + PR)         (cargo test)    (tag + merge)   (HEAD == origin/main)
```

### 2.1 Local lock

Before opening a cycle, acquire the vault lock:

```bash
VAULT="$HOME/.sddk-knowledge/arch-stack"
LOCK="$VAULT/milestones/_active.md"

if grep -q "LOCKED" "$LOCK" 2>/dev/null; then
    echo "Cycle in progress — close it first (/sddk-release)"
    exit 1
fi
```

If available, mark the lock for your milestone before starting work.

### 2.2 Code

1. **Branch from `main`** with the convention `<type>/<short-slug>`:
   - `feat/<slug>` — new capability or behaviour.
   - `fix/<slug>` — bug fix.
   - `refactor/<slug>` — internal change, no API shift.
   - `docs/<slug>` — pure docs cycle (no tag bump).
   - `chore/<slug>` — tooling, deps, CI.
2. **1 task = 1 commit.** Conventional commits only. **No
   `Co-Authored-By:`** trailers.
3. **Update the matching manifest** (see §3 below).
4. **Update CHANGELOG.md** if the change is user-facing.
5. **Run the cheap verification gate** before pushing:
   ```bash
   bash scripts/verify-local.sh
   ```
   This runs `cargo build`, `cargo test`, `cargo clippy --all-targets
   -D warnings`, `cargo fmt --check`, `cargo build --release`, and the
   `code` scope of `archctl doctor`.

### 2.3 Verify

The local gate catches the common bugs. Full CI runs the same gate plus
release-binary embedding.

### 2.4 Release

When CI is green:

1. **Squash- or merge-merge** to `main`.
2. **Tag the squash commit** with the next semver (`vMAJOR.MINOR.PATCH`):
   - **Major** = breaking CLI/API change.
   - **Minor** = user-facing capability, refactor with public surface,
     schema migration.
   - **Patch** = bug fix, internal refactor with no surface shift,
     docs-only cycle (M62 is the precedent — **pure docs skip the
     tag**).
3. **Open a ROADMAP companion PR** (`docs/<cycle>-roadmap`) with one
   row appended to the cycle log table in `docs/ROADMAP.md`.
4. **Update the vault** — release-report phase creates the cycle node
   in `~/.sddk-knowledge/arch-stack/cycles/`.

### 2.5 Sync main

After release, `git checkout main && git pull`. Verify:

```bash
git rev-parse HEAD == git rev-parse origin/main
git tag --points-at HEAD  # shows the new tag
```

### 2.6 Record to Engram

Before closing the session, `mem_save` with the discoveries and the
final result contract. This is mandatory — it is how the next session
learns what you did.

---

## 3. Manifest hygiene (the recurring trap)

`manifests/<scope>.toml` declares what a module exposes and what it
must (and must not) contain. The `gate_public_symbols_exist` check
verifies that every `pub` symbol listed in `public_symbols` actually
exists, and the `gate_must_hold` check fails the build if a `must_hold`
pattern is missing.

**What `public_symbols` covers:**

- Functions, structs, enums, traits, type aliases declared with `pub`
  at module top level or in `impl` blocks for traits.

**What `public_symbols` does NOT cover (documented blind spot):**

- **Enum variants** — a new variant on an existing public enum does
  not require a manifest update. The check scans the type, not the
  variants.
- **Struct fields** — adding a field to a public struct does not
  require a manifest update.
- **Removed functions** — when you delete a public function, the
  check does **not** flag a stale entry. Audit by hand.

**When to add an entry to `public_symbols`:**

- You introduce a new top-level `pub fn`, `pub struct`, `pub enum`,
  `pub trait`, or `pub type` that is **part of the module's public
  contract**. Skip `pub(crate)` symbols and skip helper functions.
- Reference M46 for the 26 stale entries that were removed in one
  sweep (commit `bc8cbbc`).

**When to update `must_hold`:**

- You add a new **architectural rule** the module enforces (e.g. "no
  imports of `crate::render` from `crate::diagram`"). Document the
  reason in the commit body.
- Avoid `must_hold` entries that capture trivial style rules — those
  belong in `cargo clippy` or `cargo fmt`.

**When to update `must_not_contain`:**

- A module must never import from another module (e.g.
  `diagram` → `render`). Add the rule with a justification that names
  the dependency direction.

---

## 4. The bounded contexts

The `archctl/src/<context>/` modules are **bounded contexts** with a
clear dependency direction:

| Layer | Modules | May import |
|---|---|---|
| CLI | `cli.rs`, `main.rs` | application |
| Application | `diagram/`, `evidence.rs`, `evaluation.rs` | ports + domain |
| Ports | `store`, `clock`, `filesystem` | (defined here) |
| Adapters | `store::LbugStore`, `clock::SystemClock`, `filesystem::StdFilesystem`, `astgrep.rs`, `source.rs`, `tsg.rs` | ports |
| Domain | `graph.rs` | stdlib only |

**Verify a change respects the direction** by running the doctor:

```bash
cargo run --bin archctl -- doctor --scopes <scope-id> --cwd .
```

A clean run means no `must_not_contain` rules fired.

---

## 5. Documentation rules

- **`docs/README.md`** is the index. Add new files there.
- **`CONTEXT.md`** is the executive summary; it changes rarely.
- **`docs/adr/<NNNN>-<slug>.md`** for any decision that affects the
  API, the data model, the schema, or the build. Format follows the
  ADR template. A new ADR that contradicts an existing one
  **supersedes** it (do not edit the old one; add a `Superseded by
  ADR-NNNN` footer).
- **`docs/specs/<context>/`** for Given-When-Then scenarios.
- **Comments** document the **why**, not the what.
- **Diagram sources** live in `docs/diagrams/` (or inline in the spec);
  render via `archctl` itself.

---

## 6. Testing

- **Unit tests** in `#[cfg(test)] mod tests` next to the code they
  cover. 1 test per branch of behaviour + 1 regression test per bug.
- **Integration tests** in `archctl/tests/<name>.rs`. One file per
  bounded context or per scenario cluster.
- **Benchmarks** in `archctl/benches/` when you touch a hot path
  (>1K nodes or >100ms latency).
- **Golden fixtures** in `archctl/tests/fixtures/<context>/`. Re-run
  the test with `UPDATE_GOLDEN=1` only when the change is
  intentional; commit the regenerated fixture with a body that names
  the field(s) that changed.

---

## 7. What NOT to do

- ❌ Commit secrets, credentials, or `/tmp/` artefacts.
- ❌ Add a `Co-Authored-By:` trailer.
- ❌ Force-push to `main`. Force-push only to your feature branch
  before opening the PR.
- ❌ Bypass the manifest gate (`#[allow(...)]` without a comment that
  names the upstream bug).
- ❌ Add a new dependency without an ADR evaluating its maintenance
  cost (AGENTS.md § Dependencies).
- ❌ Reimplement what an adapter already wraps (AGENTS.md § 2).
- ❌ Silence a failing test with `#[ignore]` — either fix the test
  or the code; if pre-existing, mark in the commit body and link
  the issue.

---

## 8. Getting help

- **Vault** (`~/.sddk-knowledge/arch-stack/`): decisions, ADRs,
  requirements, cycle logs.
- **OpenSpec archive** (`openspec/`): historical cycles from the
  pre-vault era.
- **Engram**: `mem_search` past work and decisions.
- **Human**: open a GitHub issue with a repro.

Welcome to the project. Read the code, run the tests, ship small
cycles.