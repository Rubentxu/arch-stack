# ROADMAP — Architecture Intelligence Platform (`archctl`)

> Outcome-oriented roadmap. Lead with the next action; everything else is context.
> **Implementation language lock (ADR-0001):** TypeScript for M0–M2. No Go core, no Rust core. Runner (Node vs Bun) probed in 0.2, neither assumed. The drift-guard script is **TBD until task 2.16** — do not describe it as existing.
> **All eight ADRs are `Accepted`** as of 2026-07-29 (see [ADR index](adr/README.md)). Confidence calibration is an open Phase-1 experiment, not a solved design.

---

## Next action

**Run Phase 0 (3–4 days):** scaffold the minimal TS workspace, pin the current OpenCode release with a vendored `config.json` snapshot, run the smoke probe (0.2) to validate runner (Node vs Bun) + hook firing + write-confinement ordering + renderer prerequisites, author the evidence-discipline skill (0.3), then **Gate Zero (0.4)** — adapt exactly one external skill end-to-end and run it against a tiny 5-file non-Git fixture with a hand-labelled gold set; normalize the produced output (not hand-authored) to a minimal IR; project to `workspace.dsl`; validate/export with a pinned headless Structurizr command and inspect through the `local` viewer. **Stop on any of:** skill adaptation failure, write outside `$ARCHCTL_PROJECT_DIR`, unsupported high-confidence claim, render failure. Do not invest in Phase 1 until Gate Zero passes.

---

## Milestones at a glance

| # | Milestone | Window | Exit criterion | Kill criterion | ADRs |
|---|-----------|--------|----------------|----------------|------|
| M0 | **Skill-only baseline + Gate Zero** (4 tasks: scaffold+pin, smoke probe, skill content, two-part Gate Zero test) | **3–4 days** | Adapted skill runs end-to-end on 5-file non-Git fixture; IR normalized from produced (not hand-authored) output; `workspace.dsl` passes pinned headless validation/export and can be inspected with the `local` viewer; zero unsupported high-confidence claims; all writes confined to XDG | Any of {skill fails, runner missing/incompatible, write confinement breach, unsupported high-confidence, render fail} → stop | ADR-0002, ADR-0005, ADR-0008 |
| M1 | **Discovery Spike** (2 real repos + adversarial fixtures, TS path) | **2–4 weeks** | `unsupported_claims_high_confidence=0` on small Rust repo; `evidence-coverage≥0.90`; `render-success=100%`; Jaccard≥0.95; adversarial fixtures (symlink-escape, prompt-injection) all green | Any metric < kill threshold (see gates) → halt at Phase 0/1 | ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0005, ADR-0006, ADR-0007, ADR-0008 |
| M2 | **MVP Plugin-First** (TS plugin + 4 roles + ops + drift guard creator) | **4–6 weeks** | `precision≥0.85`, `recall≥0.80`, `cost<50k tokens`, `lead-time<10m`; XDG isolated; `skills.lock.json`; doctor covers MCP/extractor binary inventory; data-not-instructions in synthesizer+auditor; 2.16 drift-guard script created and green | `precision<0.70` OR `recall<0.60` OR `cost>200k tokens` OR `>30m` → iterate fast profile, no Phase 3 | ADR-0001, ADR-0006, ADR-0007, ADR-0008 |
| M3 | **Rust core (conditional, deferred)** | TBD | TS-normalization overhead measured > X AND M2 fully passes | Overhead not justified OR M2 fails → keep TS, document simplicity as feature | ADR-0001 |
| M4 | **Evolution** (temporal model, drift diff, CI gate, separate falsifier) | TBD | Drift diff CI catches >0 unsupported changes per release | Cost > value → defer further | (new ADRs) |
| M5 | **Architectural twin** (observed graph, Joern/CodeQL on-demand, headless SDK) | TBD | Observed graph adds falsifiable value; deep analysis license-gated | Telemetry/observability not available in target repos → defer | (new ADRs) |

---

## Current planning state

**Active change:** `architecture-intelligence-platform` · **planning approved and ADRs accepted** (ADR-0001..ADR-0008 all Accepted 2026-07-29, OpenCode paths plural, drift guard TBD until task 2.16, Phase 0 restructured to 4 tasks, security/ops acceptance integrated into existing tasks).
**Artifacts approved:** proposal (C0, auto-grill resolved), spec (R1–R13, S1–S19), design (4-role topology, discriminated SourceIdentity, uniform Adapter seam), coherence gate **1.6 PASS (score 97)**.
**ADRs accepted on 2026-07-29:**

| ADR | Title | One-line |
|-----|-------|----------|
| [ADR-0001](adr/0001-plugin-first-no-rust-first.md) | Plugin-First / No-Rust-First | TS for M0–M2; Rust only after M2 + measurable thresholds (>2× adapter overhead OR >30s normalisation OR >2× memory) |
| [ADR-0002](adr/0002-neutral-ir-truth-structurizr-projection.md) | IR = truth, Structurizr = C4 projection | IR is the only source; diagrams are pure functions |
| [ADR-0003](adr/0003-xdg-runtime-state-export-bundle.md) | XDG + export bundle | `~/.local/share/archctl/` default; portable `projectId` (UUIDv4) + rebind default-reject; discriminated SourceIdentity (git OR directory); Git optional |
| [ADR-0004](adr/0004-evidence-ontology-confidence-provenance.md) | Evidence ontology + provenance | `fact/inference/hypothesis/unknown/conflict` + mandatory `method` enum; `unsupported_claims_high_confidence==0` is HARD FAIL |
| [ADR-0005](adr/0005-renderer-routing-local-first.md) | Renderer routing / local-first | Structurizr `local` viewer + pinned headless CLI; PlantUML local; Mermaid excluded by default; no public servers |
| [ADR-0006](adr/0006-reuse-over-rebuild-capability-adapters.md) | Reuse + uniform Adapter | No custom parsers; OCP seam (Shape B + declarative ShellAdapter) |
| [ADR-0007](adr/0007-opencode-version-pin-schema-contract-minimal-topology.md) | OpenCode pin + schema-contract + 4 roles | `mcp` (not `mcpServers`); initial pin `1.18.x`; CI schema-test; max 4 agent roles |
| [ADR-0008](adr/0008-supply-chain-pinning-sandbox.md) | Supply-chain pinning + sandbox | `skills.lock.json` pinning + SPDX allow-list + canonical-root write-guard + MCP/tool inventory |

---

## Gates (falsable thresholds)

| Gate | Phase | Accept | Kill |
|------|-------|--------|------|
| `gate_zero_two_part` | M0 | Adapted skill on 5-file non-Git fixture: zero unsupported high-confidence claims + zero write-confinement breaches + local render succeeds | Any single failure → stop |
| `unsupported_claims_high_confidence` | M1 | = 0 (small Rust) | > 0 → HARD FAIL |
| `evidence_coverage` | M1 | ≥ 0.90 | < 0.70 |
| `render_success` | M1, M2 | = 100% | < 80% |
| `stability_jaccard` (2 runs, same anchor) | M1 | ≥ 0.95 | < 0.80 |
| `semantic_precision` (manual sample) | M2 | ≥ 0.85 | < 0.70 |
| `semantic_recall` (fixture gold) | M2 | ≥ 0.80 | < 0.60 |
| `cost_per_recovery` | M2 | < 50k tokens, < 5 min | > 200k tokens OR > 30 min |
| `lead_time_first_diagram` | M2 | < 10 min (medium TS) | > 30 min |
| `repo_pollution` | M2, M3, M4, M5 | 0 files in analyzed repo | > 0 → fail |
| `language_lock_ts` | M0, M1, M2 | zero non-TS sources under `packages/` (2.16 creates the guard; forbidden extensions live in that script) | any violation → fail |
| `prompt_injection_resistance` | M1, M2 | adversarial prompt-injection fixture produces zero IR contamination | any contamination → fail |
| `symlink_escape_resistance` | M1, M2 | adversarial symlink-escape fixture is rejected by write-guard (realpath containment) | any escape → fail |
| `unsupported_claims_medium` | M1, M2 | registered as `unknown` (auditable) | hidden in IR → fail |

---

## Decision points

| Point | When | Question | Outcome |
|-------|------|----------|---------|
| **DP1** | After M0 | Did the adapted skill pass the Gate Zero two-part test AND a TS runner (Node or Bun) prove compatible with the pinned OpenCode AND the tiny RE semantic micro-test on the 5-file non-Git fixture succeed (zero unsupported high-confidence + zero write-confinement breach + render OK)? | No → archive platform work, retain skill-only |
| **DP2** | After M1 | Did the 2-repo spike meet accept thresholds AND pass adversarial fixtures (symlink-escape + prompt-injection) in the TS path? | No → keep Phase 0/1, write `spike-report.md` with falsifying evidence |
| **DP3** | After M2 | Does the TS path meet precision/recall/cost AND does the TS-normalization overhead justify Rust? | No+No → keep TS, document simplicity as feature · No+Yes → revisit M3 · Yes+Yes → M3 |
| **DP4** | Mid-M4 | Does temporal/drift-diff add measurable value? | No → defer to M6 (post-roadmap) |
| **DP5** | Mid-M5 | Is observed-graph telemetry available in target repos? | No → defer M5 indefinitely |

---

## Explicitly deferred (out of MVP scope)

- **Rust core.** Conditional on M2 + measured TS-normalization overhead. ADR-0001. Locked out of M0–M2 by the language-drift guard (task 2.16 creates the script).
- **Go core.** Not on the roadmap. Not discussed as a future option. ADR-0001 forbids non-TS cores until M3 evaluation.
- **Temporal evidence model** (`validFrom/validTo`, history store). IR v1 keeps the seam (`schemaVersion` + `meta` bag) but no history store until value is proven. Phase 4.
- **Drift-diff CI gate** (declared vs static). Phase 4.
- **Falsifier as a separate agent.** Folded into Auditor in MVP. Phase 4.
- **Deep analysis adapters** (Joern, CodeQL). On-demand, license-gated, never per-run. Phase 5.
- **Observed graph** (OpenTelemetry / runtime traces). Requires telemetry most repos lack. Phase 5.
- **Headless SDK orchestration** (`archctl orchestrate`, SSE). Phase 5, the most complex option.
- **Confidence calibration method.** Open experiment in Phase 1; v1 uses heuristic assignment with explicit provenance. Not a solved design.
- **Full resume from arbitrary stage checkpoint.** MVP resume is **stage-level** (re-run a stage from its durable artifact; no mid-stage resume). Beyond this is Phase 3+.

---

## Out of scope (non-goals)

- Custom parsers/indexers for any language. ast-grep + ctags + build tools + dependency-cruiser.
- Per-run deep analysis. License and cost; on-demand only.
- Mermaid as canonical C4 representation. Experimental; non-canonical preview.
- Public PlantUML/Kroki servers. Local jars or internal container only.
- `structurizr/lite` (EOL). Use the Structurizr `local` viewer plus a pinned headless validation/export command.
- `mcpServers` as OpenCode config key. Canonical is `mcp` (top-level).
- In-repo `.architecture/` storage. XDG only; explicit export bundle.
- Source snippet storage. Path/lines/hash only by default (`store-source-snippets: false`).
- Non-TypeScript core languages (Go, Rust) in M0–M2. Enforced by language-drift guard (script TBD until 2.16).
- Mid-stage resume from arbitrary checkpoint. MVP = stage-level only.

---

## Reversibility

- **M0 failure:** delete `~/.local/share/archctl/` + wrapped skill. Repo untouched. Residual value: disciplined manual diagramming.
- **M1 failure:** remove `~/.local/share/archctl/` + plugin TS + skills. Repo untouched.
- **M2 rollback:** revert to M1 (plugin + ledger + IR without capability router). Router is additive.
- **M3 rollback (if Rust activated prematurely):** keep `archctl` TS path primary; Rust migration is opt-in and isolated behind IPC seam.

---

## Open questions (not blocking; surfaced for visibility)

- Confidence calibration method — Phase-1 experiment; v1 heuristic.
- Exact OpenCode hook signatures across versions — pin + schema-contract CI (vendored snapshot) resolves before expansion.
- Claude-Code ↔ OpenCode skill adaptation surface — Gate Zero M0.4 resolves before registry commitment.
- `subagent_depth: 2` real context-budget behavior — confirmed in schema; runtime validated in M0/M1.
- **TS runner (Node vs Bun)** — probed in M0.2 against pinned OpenCode; both are acceptable, neither is assumed.

---

*This is a planning artifact. No branches or commits created. Next recommended action: ask user to (a) accept Proposed ADRs (ADR-0001..ADR-0008), (b) confirm chain strategy (feature-branch-chain vs stacked-to-main) and start WU1, (c) resolve whether confidence calibration is in-scope for the planning window or stays as an open experiment, (d) confirm TS-only M0–M2 lock with 2.16 drift-guard creator.*
