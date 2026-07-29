# M2 — MVP Plugin-First (TS plugin + 4 roles + ops + drift guard)

## Locked

- Implementation language: **TypeScript for M0–M2** (ADR-0001).
- Runner: Node or Bun, decided by Gate Zero.
- Forbidden-extension policy: the `scripts/check-language-drift.ts`
  script is the **single source of truth** for what counts as a violation.
  Editing prose never overrides the script.

## Acceptance gates

- `archctl doctor` exits 0.
- Probe + Gate Zero remain PASS.
- 4-role agent topology exists under `.opencode/agents/` and is loaded
  by the vendored OpenCode 1.18.x schema snapshot.
- Write-guard rejects escape-symlink attempts and writes are atomic
  (temp + rename).

## Drift guard

The full guard replaces the WU1 placeholder:

| Severity | Trigger | Action |
|---|---|---|
| `ok` | no forbidden language detected | exit 0 |
| `warn` | any forbidden extension under `packages/` | exit 0 (logged) |
| `fail` | any forbidden source signature detected | exit 1 |

CI runs `--fail-on=fail`. The 2.16 task is the **first concrete reference**
to the script; earlier tasks marked it TBD.
