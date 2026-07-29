# Tasks: architecture-intelligence-platform

> Dependency-ordered plan. **Gate Zero (Phase 0.4) is the kill-switch**: if the 1-skill end-to-end test on a tiny non-Git fixture fails, retain skill-only baseline and stop platform investment. **Phase 2 is conditional on Phase 1 kill-gate passing**. **Phase 3+ deferred** (Rust, temporal, observed graph). ADRs are **Accepted** as of 2026-07-29.
> **Implementation language (ADR-0001):** TypeScript-only for M0–M2. Forbidden-extension policy prose is **TBD until task 2.16** creates `scripts/check-language-drift.ts` — do not describe the script as existing in earlier tasks. Node/Bun runner probed during 0.2, neither assumed.
> Spec traceability: `S1`–`S20` from `spec.md`; design seams from `design.md` §2–§6.
> ADR references: canonical four-digit (`ADR-0001`..`ADR-0008`).

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines (full MVP) | 1100–1700 LOC (heuristic, greenfield, TS) |
| Per-phase LOC | Phase 0: 120–200 · Phase 1: 550–850 · Phase 2: 500–750 |
| 400-line budget risk | **High** (any single phase approaches or exceeds) |
| Chained PRs recommended | **Yes** |
| Suggested split | WU1 skill-baseline+gate-zero → WU2 pin+resolver+schema-contract → WU3 evidence+IR+ledger → WU4 router+adapters → WU5 projection+renderers → WU6 plugin+agents → WU7 doctor+export+docs+drift-guard |
| Delivery strategy | **ask-on-risk** (planning only, no commit) |
| Chain strategy | **feature-branch-chain** (tracker `feature/archctl` accumulates; WU#N bases on WU#N-1; only tracker merges to `main` at exit gate) |
| Open risks affecting size | OpenCode hook signature drift · confidence calibration unknown · Claude→OpenCode skill adaptation surface · runner selection (Node vs Bun) probed in 0.2 |
| Language-drift guard | New task 2.16 + acceptance on 1.2 fail build on any non-TS source under `packages/` (ADR-0001; **script itself is TBD until 2.16 creates it**) |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| WU1 | Minimal TS scaffold + OpenCode pin/schema snapshot + smoke probe + skill-only baseline + Gate Zero two-part test | PR #1 → `feature/archctl` | License/pinning, evidence skill, **kill-switch deliverable**; validates runner (Node vs Bun), skill compatibility, and tiny RE semantic micro-test |
| WU2 | OpenCode pin already done in WU1; expand schema-contract CI (vendored snapshot + advisory live check) + discriminated SourceIdentity resolver + XDG layout | PR #2 → `feature/archctl` | BLAKE3 identity, git|directory, portable projectId; ADR-0003, ADR-0007 |
| WU3 | Evidence ledger v1 (JSONL, single-writer/per-run segment) + Architecture IR v1 + migration registry + idempotency tests | PR #3 → `feature/archctl` | Append-only per run, schemaVersion, meta extension point; ADR-0002, ADR-0004 |
| WU4 | Capability router (Shape B) + declarative `ShellAdapter` + 1 fast-profile adapter (ast-grep) + bash allowlist | PR #4 → `feature/archctl` | OCP seam; ADR-0006 |
| WU5 | Projections (IR→Structurizr `local`, IR→PlantUML, Mermaid preview) + local render harness | PR #5 → `feature/archctl` | Pure functions; ADR-0005; **render-success=100% gate** |
| WU6 | TS plugin (`shell.env` resolver + `tool.execute.before` write-guard) + 4 agent roles + `/archctl` slash command | PR #6 → `feature/archctl` | Permission matrix + realpath containment + symlink-escape rejection + atomic temp+rename; ADR-0007, ADR-0008 |
| WU7 | `archctl doctor` (binary/license/version/hash pin) + export/import bundle + `skills.lock.json` + cost/token budgets + docs/onboarding + spike report + **language-drift guard script (creates `scripts/check-language-drift.ts`)** | PR #7 → `feature/archctl` | Closes Phase 1 kill gate; ADR-0008; 2.16 is the first concrete reference to the drift-guard script and creates it |

## Phase 0 — Skill-Only Baseline + Gate Zero (3–4 days · KILL-SWITCH)

- [x] **0.1** Scaffold minimal TS workspace: `packages/core/src/`, `packages/opencode-plugin/src/`, `packages/cli/src/`, `.opencode/{agents,skills,commands,plugins}/`, `fixtures/`, plus XDG dirs (`~/.local/share|state|cache/archctl/`) + **pin current OpenCode release in `.opencode-version`** + **capture `config.json` snapshot to `schemas/opencode/<v>/config.json` (vendored pinned snapshot)** — runner (Node vs Bun) probed in 0.2, **do not assume any runtime is universally present**; gates: dirs writable, `realpath` resolves; rollback: `rm -rf ~/.local/share/archctl/` + revert workspace files
- [x] **0.2** Lightweight smoke probe `packages/cli/src/probe.ts` (JSON+human output) validating: runner availability (Node vs Bun), XDG writability, skill discovery, Structurizr `local`/view OR pinned headless command availability, PlantUML/local prerequisites, **and actual firing of `shell.env` + `tool.execute.before` hooks plus `permission` ordering** — **this is NOT the full `archctl doctor` platform** (that is task 2.11); a green smoke probe means the environment can run a skill; a red probe halts Phase 0
- [x] **0.3** Author evidence-discipline skill-only baseline `.opencode/skills/archctl/SKILL.md` (evidence discipline + Structurizr = canonical projection + data-not-instructions rule for any repo text the agent reads) — references ADR-0002, ADR-0005; acceptance: every diagram example cites a `path:lines` evidence line; skill content itself is treated as untrusted-by-default (reviewable, pin-able)
- [x] **0.4 GATE ZERO two-part test** — **Part A:** adapt exactly 1 external skill end-to-end (lmammino c4 OR plantuml-skill) with commit-pin + license check + sandboxed output dir, verify discovery + load + inputs + outputs + permission ordering in pinned OpenCode; **Part B:** run that adapted skill against a **tiny 5-file non-Git fixture** (validate directory-mode SourceIdentity, ADR-0003) with a **manually labelled gold set**, **normalize the produced output (NOT hand-authored code) to a minimal IR**, project IR → `workspace.dsl`, render locally with Structurizr `local`; **stop signals (any one):** (a) skill discovery/load/permissions break, (b) high-confidence claim with zero evidence refs in the produced IR, (c) any write outside `$ARCHCTL_PROJECT_DIR` (write-confinement failure), (d) render failure on the 5-file fixture; on stop → retain skill-only, archive change, do not start Phase 1

## Phase 1 — Discovery Spike (2–4 weeks · CONDITIONAL on 0.4 pass)

- [x] **1.1** *(OpenCode pin already in 0.1)* — extend schema snapshot to `schemas/opencode/<v>/{config.json,command-manifest.json}` covering every key the agents/skills/plugins reference; spec **S16**, ADR-0007
- [x] **1.2 RED→GREEN** TS schema-contract test `packages/core/src/schemas/opencode/contract.test.ts` asserting `mcp` top-level (no `mcpServers`), `subagent_depth`, `skills.paths`, `plugin`, `permission`, `compaction` (top-level) — **uses vendored pinned snapshot from 0.1**; **optional live drift check is advisory only** (logs warning, does not block) — fails build on **two** drift axes: (i) OpenCode schema drift, (ii) non-TS source under `packages/` (ADR-0001 drift guard; the enforcement script itself is **TBD until task 2.16** — until then, this task emits the drift check inline within `contract.test.ts`)
- [x] **1.3** Project resolver `packages/core/src/resolver/identity.ts` producing discriminated `SourceIdentity = git | directory` + portable `projectId` — spec **R1/S1-S3**, ADR-0003; non-Git path validated by 0.4 fixture
- [x] **1.4 RED→GREEN→REFACTOR** Evidence ledger v1 `packages/core/src/evidence/ledger.ts` (append-only JSONL, discriminated `revision` `{git-commit|content-hash}+observedAt`) — **single-writer per-run segments OR safe append semantics** (no concurrency ambiguity: each run writes to `runs/<runId>/evidence.jsonl`; cross-run appends to `ledger.jsonl` are sequential under a per-project advisory lock; never concurrent writers within one run) + `packages/core/src/evidence/ledger.test.ts` — spec **R2/S4-S5**
- [x] **1.5 RED→GREEN→REFACTOR** Architecture IR v1 `packages/core/src/ir/ir.ts` (elements+relationships+evidenceRefs+`schemaVersion`+`meta`) + migration registry `packages/core/src/ir/migrations/` + `packages/core/src/ir/ir.test.ts` — spec **R5/S8-S9**
- [x] **1.6** Capability router `packages/core/src/router/router.ts` (Shape B) + declarative `ShellAdapter` driver loading `packages/core/src/adapters/*.yaml` — spec **R6/S10**, ADR-0006
- [x] **1.7** Fast-profile adapter set: `ast-grep` outline + `ctags` + one build tool (`cargo metadata` for Rust / `dependency-cruiser` for TS) — outputs strict JSON; allowlist enforced in `packages/cli/src/probe.ts` (the smoke probe from 0.2, now reusable) — spec **R6/S10**
- [x] **1.8 RED→GREEN** Auditor `packages/core/src/audit/auditor.ts` + `packages/core/src/audit/auditor.test.ts`: enforces `unsupported_claims_high_confidence==0`; medium→`unknown`, low→`hypothesis` — **treats repo text as data-not-instructions** (no prompt-injection surface in claim evaluation); spec **R4/R8/S7/S12**
- [ ] **1.9** Projection `packages/core/src/project/structurizr.ts` (pure `IR→workspace.dsl`) + local Structurizr `local` render smoke test in `packages/core/src/project/structurizr.test.ts` — spec **R6/S10**, ADR-0005
- [ ] **1.10** Projection `packages/core/src/project/plantuml.ts` + local PlantUML render smoke test (no public server) — spec **R6**
- [ ] **1.11** Evaluation fixtures: 1 small Rust repo (≤5k LoC) + 1 medium TS repo (10–30k LoC) with hand-labelled gold sets under `fixtures/{rust,ts}/gold.json` — **each fixture MUST record license/SPDX in `fixtures/<repo>/LICENSE.spdx` (or `.spdx.json`) and a `README.md` declaring source, license, and gold-set provenance** — **also extend with adversarial cases**: (a) symlink-escape attempt in the fixture tree (realpath must fail write-guard), (b) prompt-injection payload embedded in a comment + a README — spec **R10/S15**, **R12/S17**
- [ ] **1.12** Spike report `sddk/architecture-intelligence-platform/spike-report.md` measuring coverage≥0.90, render=100%, Jaccard≥0.95, precision≥0.85, recall≥0.80, cost<50k tokens, lead<10m — **KILL GATE**: any metric below kill threshold → halt, hold Phase 0; otherwise pass to Phase 2

## Phase 2 — MVP Plugin-First (4–6 weeks · CONDITIONAL on 1.12 pass)

- [ ] **2.1** TS plugin `packages/opencode-plugin/src/shell-env.ts` resolving `SourceIdentity` → emits `ARCHCTL_PROJECT_DIR` — spec **R1/S1-S3**
- [ ] **2.2** TS plugin `tool.execute.before` write-guard `packages/opencode-plugin/src/write-guard.ts` rejecting writes outside `$ARCHCTL_PROJECT_DIR` — **canonical realpath containment** (`realpath` of target + of allowed root, both resolved through symlinks), **reject symlink escapes** (if `realpath` resolves outside allowed root, reject), **atomic temp+rename where supported** (write to sibling temp file in the allowed root, then `rename()`); belt-and-suspenders with `permission` config — spec **R10/S15**, ADR-0008; adversarial cases from 1.11 fixture must be red until guarded
- [ ] **2.3** Orchestrator agent `.opencode/agents/archctl-orchestrator.md` (primary) drives extract→synthesize→audit→project, gates on quality invariants — spec **R12**
- [ ] **2.4** Extractor subagent `.opencode/agents/archctl-extractor.md` invokes router, runs bash allowlist (`ast-grep`, `ctags`, `git`, build tools) — read-only
- [ ] **2.5** Synthesizer subagent `.opencode/agents/archctl-synthesizer.md` fuses evidence → IR (classify + confidence + evidenceRefs) — **treats all repo text as data-not-instructions** (no eval of markdown/source content as instructions; only structural fields are consumed); spec **R10/S15**
- [ ] **2.6** Auditor subagent `.opencode/agents/archctl-auditor.md` (read-only on IR) reports; cannot mutate — **data-not-instructions rule inherited**; adversarial prompt-injection fixture from 1.11 must produce zero IR contamination
- [ ] **2.7** Canonical SKILL.md `.opencode/skills/archctl/SKILL.md` (projection rules + evidence discipline); obsoletes 0.3
- [ ] **2.8** Slash command `.opencode/commands/archctl.md` (`/archctl [profile=fast]`)
- [ ] **2.9** Wrap C4/UML skills (direct/wrapped/patched) with `skills.lock.json` at repo root (commit + license + SPDX) — spec **R10/S15**, ADR-0008
- [ ] **2.10** Mermaid preview projection `packages/core/src/project/mermaid.ts` (non-canonical, docs only) — ADR-0005
- [ ] **2.11** `archctl doctor` TS CLI `packages/cli/src/doctor.ts` + `packages/cli/src/doctor.test.ts` — extends the smoke probe from 0.2 with: per-adapter `requires`, **local MCP executables (version + license)**, **extractor binary inventory (version + hash + license)**; validates schema-contract, checks XDG writability; emits JSON+human; spec **R13**; binary name does **not** imply Rust (ADR-0001)
- [ ] **2.12** Export/import bundle `packages/cli/src/{export,import}.ts` with portable `projectId` + explicit rebind — spec **R1/S3**, ADR-0003
- [ ] **2.13** Security: secret redaction in evidence (`store-source-snippets: false` default), path/line/hash only — spec **R10/S15**; pairs with adversarial fixtures from 1.11
- [ ] **2.14** Onboarding: `README.md`, `docs/quickstart.md`, `docs/architecture.md` (link `CONTEXT.md` + ADRs)
- [ ] **2.15** Cost/token instrumentation `~/.local/state/archctl/runs/<id>/<runId>.jsonl` (tokens, elapsed, unsupported, coverage) + budget assertion `<50k tokens, <5m` per run — spec **R12/S17**
- [ ] **2.16 — Language-drift guard (ADR-0001 enforcement) — CREATES the script** `scripts/check-language-drift.ts` + CI step that **fails the build** if any non-TS source file appears under `packages/`; **this task is the first concrete reference to the script** (all earlier tasks marked it TBD); forbidden-extension set lives in this file as the single source of truth (earlier tasks reference it by name only); acceptance: green on first run (zero violations), red on any drift attempt; pair with README note: "Implementation language is TypeScript for M0–M2. Rust is gated by ADR-0001 and explicitly deferred (D1)."

## Phase 3+ — DEFERRED (NOT in MVP; listed only for traceability)

- [ ] **D1** Rust core (TS→Rust migration) — **only** if M2 validation passes AND measured TS-normalization overhead justifies it. ADR-0001. **NOT** in MVP scope; 2.16 drift guard will reject any premature Rust.
- [ ] **D2** Temporal evidence (`validFrom/validTo`) + drift-diff CI gate — Phase 4. IR stays forward-compatible via `meta` + `schemaVersion`
- [ ] **D3** Observed graph (OpenTelemetry) — Phase 5. `evidence.kind` enum adds `observed` additively
- [ ] **D4** Falsifier as separate agent — Phase 4. Folded into Auditor in MVP
- [ ] **D5** Deep analysis adapters (Joern/CodeQL) on-demand, license-gated — Phase 5
- [ ] **D6** Headless SDK orchestration (`archctl orchestrate`, SSE) — Phase 5

## Forecast — Critical Path, Dependencies, PR/Commit Slices

**Critical path:** 0.1 (scaffold + pin) → 0.2 (smoke probe) → 0.3 (skill content) → 0.4 (Gate Zero two-part test) → 1.1-1.2 (schema-contract + language-drift acceptance) → 1.3 (resolver) → 1.4-1.5 (ledger+IR) → 1.6 (router) → 1.7 (adapters) → 1.8 (auditor) → 1.9-1.10 (projections) → 1.11 (fixtures incl. adversarial) → 1.12 (spike report) → 2.1-2.2 (plugin + write-guard) → 2.3-2.6 (agents + data-not-instructions) → 2.11 (doctor with binary/license inventory) → 2.16 (drift-guard script created).

**Dependency DAG highlights:** 0.4 depends on 0.1 + 0.2 + 0.3; 1.4 depends on 1.3 (XDG path); 1.5 depends on 1.4 (evidence refs); 1.6 depends on 1.4 (RawEvidence shape); 1.7-1.8 depend on 1.6; 1.9-1.10 depend on 1.5; 2.1-2.2 depend on 1.3; 2.3-2.6 depend on 2.1-2.2 and 1.6-1.10; 2.11 builds on the smoke probe from 0.2; 2.16 creates the script that 1.2 references by name.

**Likely commit slices per work unit** (Conventional Commits, work-unit shape, tests+docs with code):
- WU1: `chore(archctl): scaffold TS workspace + pin OpenCode + capture config snapshot` · `feat(probe): smoke probe runner+hooks+XDG+renderer prereqs (JSON+human)` · `docs(archctl): evidence-discipline skill` · `test(gate-zero): 1-skill adapted + 5-file non-Git fixture end-to-end` (kill-switch deliverable)
- WU2: `test(opencode): TS schema-contract CI + drift acceptance (vendored snapshot, advisory live)` · `feat(resolver): discriminated SourceIdentity + portable projectId`
- WU3: `feat(evidence): JSONL ledger v1 (single-writer per-run segments + safe append)` · `feat(ir): Architecture IR v1 + migration registry` · `test(ir): idempotency across identical anchors`
- WU4: `feat(router): uniform Adapter contract (Shape B)` · `feat(adapter): declarative ShellAdapter + fast-profile set`
- WU5: `feat(project): IR→Structurizr local` · `feat(project): IR→PlantUML` · `test(render): local render-success=100%`
- WU6: `feat(plugin): shell.env resolver + write-guard (realpath containment + symlink escape + atomic temp+rename)` · `feat(agents): 4-role topology + data-not-instructions rule` · `feat(cmd): /archctl slash command`
- WU7: `feat(doctor): TS CLI capability matrix + MCP/extractor binary inventory` · `feat(export): portable projectId bundle + rebind` · `chore(skills): skills.lock.json + license + SPDX` · `ci(archctl): language-drift guard (script created)` · `docs(archctl): onboarding + spike report`

**Stop conditions (early termination):** 0.4 fails (skill adaptation, runner, write confinement, high-confidence unsupported claim, render) → retain skill-only, archive change; 1.12 kill-gate fails → hold at Phase 1, no Phase 2; cost/budget overrun mid-Phase 2 → pause, profile, decide.

**Rollback:** delete `~/.local/{share,state,cache}/archctl/` + remove plugin + remove skills. Analyzed repo untouched (XDG isolation).

## Standard Envelope

```yaml
status: success
executive_summary: >
  TS-first dependency-ordered plan. Phase 0 is now 4 tasks (0.1 scaffold+pin,
  0.2 smoke probe of runner+hooks+prereqs, 0.3 skill content, 0.4 Gate Zero
  two-part test: adapt 1 external skill + run against tiny 5-file non-Git fixture
  with gold set, normalize to minimal IR, project, render — kill on write-confinement
  or unsupported high-confidence). Phase 1 adds adversarial fixtures (symlink escape,
  prompt-injection), per-run ledger segments, vendored schema snapshot. Phase 2 adds
  realpath containment + symlink-escape rejection + atomic temp+rename to write-guard,
  data-not-instructions to synthesizer/auditor, MCP/extractor binary+license+hash
  inventory to doctor, and 2.16 creates the drift-guard script (all earlier tasks
  marked it TBD). Canonical ADR-0001..ADR-0008. Plurals for OpenCode paths.
  Forecast: 1100-1700 LOC, 400-line risk High, chained PRs Yes (feature-branch-chain,
  ask-on-risk). Phase 0 3-4 days, Phase 1 2-4 weeks, Phase 2 4-6 weeks.
artifacts:
  - "sddk/architecture-intelligence-platform/tasks.md"
breakdown:
  total: 38
  by_phase:
    phase_0: 4
    phase_1: 12
    phase_2: 16
    deferred: 6
language_lock:
  implementation: typescript
  runner: node_or_bun_probed_in_0.2_not_assumed
  drift_guard_script: TBD_until_2.16_creates_it
  forbidden_extensions_source: scripts/check-language-drift.ts (created by 2.16)
forecast:
  estimated_lines: 1100-1700
  budget_risk: High
  chained_prs: Yes
  delivery_strategy: ask-on-risk
  decision_needed: Yes
  chain_strategy: feature-branch-chain
  windows:
    m0: 3-4 days
    m1: 2-4 weeks
    m2: 4-6 weeks
next_recommended: ask user to (a) accept Proposed ADRs, (b) confirm chain strategy, (c) confirm TS-only M0–M2 lock (ADR-0001) with 2.16 drift guard creator, (d) resolve open experiments (confidence calibration, hook signatures)
risks:
  - "0.4 Gate Zero fails any of {skill, runner, write confinement, high-confidence unsupported claim, render} → retain skill-only (designed)"
  - "1.12 kill-gate fails → hypothesis falsified, hold at Phase 0/1 (designed)"
  - "OpenCode hook drift between pin and release → mitigated by 1.2 schema-contract CI on vendored snapshot"
  - "Claude→OpenCode skill adaptation surface unknown until 0.4"
  - "TS runner (Node vs Bun) compatibility unknown until 0.2 — neither assumed"
coherence_gate: "1.6 PASS (score 97); final adversarial corrections verified; TS-first lock applied; OpenCode paths plural; ADR-0001..ADR-0008 canonical"
skill_resolution:
  - sddk-tasks (executed; final synthesis applied)
  - work-unit-commits (commit slicing refreshed)
  - cognitive-doc-design (lead-with-answer, progressive disclosure)
  - chained-pr (forecast consulted; no PRs created per plan-only instruction)
```
