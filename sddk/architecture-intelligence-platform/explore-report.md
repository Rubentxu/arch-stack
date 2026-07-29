# Kernel Exploration: architecture-intelligence-platform

> **Path A-full planning slice · Context C0 (greenfield) · Entropy method: heuristic · No Chronos · No UI**
> Source document: `Skills-para-agentes-IA.md` (3721 lines, read in full).
> Project state: **greenfield** — only the planning doc and `.atl/skill-registry.md` exist. No code, no ADRs, no ROADMAP.

> **Post-exploration corrections (history preserved, not rewritten).** Original exploratory uncertainty is retained; corrections are marked `✅ RESOLVED downstream`. See inline notes and the log below.
>
> | # | Original (exploratory) | Correction | Status |
> |---|---|---|---|
> | C1 | MVP: "Structurizr **Lite** container" (§6 item 6) | Use the Structurizr **`local` viewer** plus a pinned headless validation/export command; **Lite is EOL** | ✅ RESOLVED downstream ADR-0005 |
> | C2 | Identity = Git-only `BLAKE3(normalized_remote + root_commit)` (§6 item 1, §4 Pros, §8, §11 ADR-3) | **Discriminated `SourceIdentity` (`git` \| `directory`)** + **portable `projectId`** carried in export bundles, **re-bound** to a local SourceIdentity on import | ✅ RESOLVED downstream ADR-0003 |
> | C3 | OpenCode claims `subagent_depth` / `OPENCODE_CONFIG_DIR` / server-SSE = "🟡 Plausible / needs pin" (§2 row 7) | Official **live schema/docs verified**: top-level **`mcp`** (not `mcpServers`), **`subagent_depth`**, skills/references/agent/plugin/permission all supported. Implementation **still pins a release + schema-tests** | ✅ VERIFIED (implementation pins release) |
> | C4 | Risk: doc mixes `mcp` vs `mcp.servers` (§4 Risk 3) | Config key is **top-level `mcp`**; `experimental.session.compacting` is a **plugin hook**, not a config key | ✅ RESOLVED downstream ADR-0007 |

---

## Context Quality

| Field | Value |
|---|---|
| Level | **C0** — greenfield; zero implementation, zero tests, zero ADRs |
| Evidence Present | One ChatGPT-generated design document (3721 lines); many external tool/URL citations |
| Missing Context | No prototype; no validation that the proposed approach beats simpler alternatives; OpenCode version un-pinned; Claude-Code-vs-OpenCode skill compatibility unmeasured |
| Recommended Effort | **deepen + recommend-lenses** — the doc is ambitious enough to demand adversarial review before any spec |

---

## 1. Executive Verdict

**Confidence: Medium-High (0.7).**

The document is **architecturally literate, source-grounded in its citations, and contains genuinely innovative ideas** (temporal architecture twin, declared/static/observed graph separation, falsifier agent, active-learning questioning). The core research it cites is **real, not hallucinated** (both arXiv IDs resolve; all three headline skills exist on GitHub).

However, it suffers from **three structural defects** that make it unfit to implement as-written:

1. **Scope explosion by 10×.** The user asked for "skills for C4/UML diagramming with reverse-engineering." The document proposes a full **architecture intelligence platform**: a Rust core, 9 agents, 14 skills, a control-plane plugin, a capability router, evidence ledger, temporal twin, XDG storage, CI/CD drift gates. This is a multi-quarter platform, not a solution to the stated problem.
2. **Zero validation of the hardest assumption.** The cited research validates multi-agent C4 generation *from a text brief* (2510.22787) and *visualization* (Code2UML) — **neither validates reliable reverse-engineering of architecture from large real codebases**, which is the proposal's load-bearing claim and the documented failure mode of every prior tool.
3. **Technology choices front-load irreversibility.** A Rust core + TypeScript plugin boundary is committed before anyone has proven the evidence-driven multi-agent loop produces better diagrams than a single agent + existing skills. The XDG pivot (good) lives alongside unreconciled in-repo `.architecture/` designs (bad).

**Recommendation:** Do NOT implement the document as a whole. Adopt its **ideas** (evidence/IR separation, capability routing, temporal model) but build a **plugin-first, no-Rust-first MVP** that validates the core loop on 2–3 real repos before any Rust investment. The document is an excellent *design compass*, a poor *build plan*.

---

## 2. Claim / Evidence Matrix

| # | Claim (from document) | Verdict | Evidence / Note |
|---|---|---|---|
| 1 | arXiv:2605.24453 "Code2UML" 5-agent pipeline + deterministic IR compaction | ✅ **Verified** | Tavily → arxiv.org/abs/2605.24453 exists (cs.SE/cs.AI). The doc's characterization is accurate. |
| 2 | arXiv:2510.22787 multi-agent C4 with deterministic + LLM-as-judge eval | ✅ **Verified** | Szczepanik & Chudziak, accepted HICSS-59 2026. Doc's description matches. **But**: input is a *system brief*, not a codebase — the doc over-generalizes its relevance. |
| 3 | lmammino/c4-codebase-architecture-skill exists, evidence-based C4 | ✅ **Verified** | GitHub real; `skills install npm:@lmammino/...`. **Nuance**: it is a Claude-Code skill, not OpenCode-native. |
| 4 | bitsmuggler/c4-skill → Structurizr DSL, multi-format export | ✅ **Verified (with caveat)** | Real repo (~52★). Export to PlantUML/Mermaid/HTML via "Structurizr vnext"; PNG/SVG is **optional**, not core as implied. |
| 5 | cheriftj/c4-model-skill 5 modes (design/code/prose/review/update) | ✅ **Verified** | Real repo; modes match. Claude-Code plugin (`.claude-plugin`). |
| 6 | OpenCode: plugin hooks `tool.execute.before/after`, `shell.env`, `experimental.session.compacting`, custom `tool` w/ schema, `directory`/`worktree` context | ✅ **Verified** | Confirmed from OpenCode source (`@opencode-ai/plugin` Hooks interface). The doc's plugin skeletons are *conceptually* right but the exact destructuring/API differs slightly from real signatures. |
| 7 | OpenCode: `subagent_depth`, `OPENCODE_CONFIG_DIR`, server/SDK with SSE + JSON Schema | ✅ **Verified** *(corrected from 🟡)* | Official **live schema/docs subsequently verified**: top-level **`mcp`** key (not `mcpServers`), **`subagent_depth`**, and **skills/references/agent/plugin/permission** are all supported. *(Original exploratory note flagged these as "needs pin"; ✅ RESOLVED downstream ADR-0007.)* Implementation **still pins a release + schema-tests** before relying on any hook. |
| 8 | Mermaid C4 syntax is "experimental", incomplete layout | 🟡 **Plausible / outdated risk** | Historically accurate; not re-verified for Mermaid's 2026 state. Needs a check. |
| 9 | SCIP moved to community governance in 2026 | 🟡 **Unverified** | Cited to a Sourcegraph blog; plausible, not confirmed. Low impact on MVP. |
| 10 | The multi-agent evidence loop reliably reverse-engineers real codebases | 🔴 **Unsupported / needs experiment** | **No cited evidence tests this.** This is the core risk. Every prior tool the doc itself surveyed fails here. |
| 11 | Structurizr as canonical C4 model-as-code, versionable | ✅ **Verified** | docs.structurizr.com/as-code is real. Sound choice for canonical projection. **Note:** use the **`local` viewer** and a pinned headless validation/export command — **Structurizr Lite is EOL** (✅ RESOLVED downstream ADR-0005). |
| 12 | CodeQL CLI on closed-source may require commercial license | ✅ **Verified** | Accurate; GitHub CodeQL terms restrict closed-source use. Doc correctly makes it optional. |
| 13 | All CLI tools (ast-grep, ctags, SCIP, jdeps, dependency-cruiser, syft, trivy, terraform graph, helm template, kubectl, joern, semgrep) exist as described | ✅ **Verified** | All are real, well-known tools; the per-tool capability descriptions are accurate. |
| 14 | `ast-grep outline` JSON output, `--config` external rules, non-XDG config | ✅ **Verified** | Accurate; ast-grep uses `sgconfig.yml`, not XDG natively. |
| 15 | Plugin can enforce writes only under `.architecture/` via `tool.execute.before` | ✅ **Plausible** | Hook exists; pattern is sound. (Note: doc later moves storage to XDG, so this guard's path must change.) |
| 16 | opencode-background-agents persists results to disk, survives compaction, write-limited | 🟡 **Plausible / needs pin** | Referenced repo; behavior description is reasonable but unverified against current OpenCode task/delegation semantics. |

---

## 3. Feasibility by Subsystem & Maturity Stage

| Subsystem | Feasibility | Key Risk | Earliest Stage |
|---|---|---|---|
| **Renderer adapters** (Structurizr/PlantUML/Mermaid/draw.io) | 🟢 High | Render infra (servers, jars) ops burden | MVP |
| **Skill wrapping** (lmammino/bitsmuggler/cheriftj → OpenCode) | 🟡 Medium | Claude-Code ↔ OpenCode plugin/command divergence | MVP (1–2 skills) |
| **Evidence ledger** (JSONL + confidence + provenance) | 🟢 High | Schema churn; keeping it minimal | MVP |
| **Architecture IR** (neutral JSON model) | 🟡 Medium | Over-design risk; must stay thin | MVP |
| **Capability router + adapter registry** | 🟢 High (pattern) | Tool availability per language | Phase 2 |
| **Fast extractors** (ast-grep, ctags, build tools) | 🟢 High | Multi-language coverage gaps | Phase 2 |
| **Semantic resolution** (SCIP/LSP) | 🟡 Medium | Indexer availability per language; setup cost | Phase 3 |
| **Checkpoints / resume / state machine** | 🟡 Medium | Correctness vs. effort; may be YAGNI early | Phase 3 |
| **OpenCode control-plane plugin** (events, guards, compaction) | 🟢 High (hooks real) | API churn across OpenCode versions | Phase 3 |
| **Temporal architecture twin** (validFrom/validTo, history) | 🔴 High ambition | Hardest, least validated; storage/query cost | Phase 4–5 |
| **Declared/static/observed graph diff** | 🔴 High ambition | Requires runtime telemetry (often absent) | Phase 5 |
| **Falsifier agent** | 🟡 Medium | Prompting reliability; diminishing returns vs cost | Phase 4 |
| **CI/CD drift gates** | 🟡 Medium | False-positive policy tuning | Phase 4 |
| **Deep analysis** (Joern/CodeQL/Semgrep) | 🟢 High (tools real) | Cost/licensing; not per-run | Phase 5 |

---

## 4. Pros, Cons, Risks, Mitigations, Non-Goals

### Pros (genuinely strong)
- **Evidence-first ontology** (fact/inference/hypothesis/unknown/conflict) — directly addresses the documented failure mode of "draw a Mermaid" tools that invent architecture.
- **Renderer independence** via a neutral IR — correct architectural instinct; diagrams as *projections*, not source of truth.
- **Capability router** — OCP-compliant; swap tools without touching agents. De-risks tool churn.
- **Reuse-over-rebuild** (later sections) — correct; the "don't write parsers" stance is mature.
- **XDG external storage** — respects "don't pollute Git"; clean project identity model. *(Original exploratory note said "BLAKE3 of remote+root-commit"; **corrected**: identity is a **discriminated `SourceIdentity` (`git` \| `directory`)** with a **portable `projectId`** re-bound on import — ✅ RESOLVED downstream ADR-0003.)*

### Cons
- **Scope is 10× the stated need.** A platform was designed where skills were requested.
- **All headline skills are Claude-Code plugins**, not OpenCode-native. Adaptation cost is real and underestimated.
- **Unreconciled dual storage design** (in-repo `.architecture/` vs XDG) across the document's two halves.
- **Heavy dependency surface** (10+ CLIs + containers) blocks adoption for an MVP.
- **Rust committed too early** for an unvalidated product hypothesis.
- **Research cited doesn't cover the hard part** (codebase reverse-engineering reliability).

### Risks (ranked)
1. **🔴 The core loop doesn't work well enough.** Evidence-driven multi-agent RE may still hallucinate on real, large, multi-language repos. *Mitigation:* spike first; measure on fixtures before building platform.
2. **🔴 Build-before-validate.** Investing in Rust core + plugin + 9 agents before proving the loop wastes months. *Mitigation:* plugin-first MVP, defer Rust.
3. **🟡 OpenCode version drift.** Hooks/config keys shift across releases *(original note: doc mixes `mcp` vs `mcp.servers` — **corrected: config key is top-level `mcp`**, and `experimental.session.compacting` is a **plugin hook**, not a config key ✅ RESOLVED downstream ADR-0007)*. *Mitigation:* pin a release; CI-gate the config schema.
4. **🟡 Skill adaptation quagmire.** Wrapping Claude-Code skills may surface incompatibilities (command systems, plugin manifests). *Mitigation:* validate ONE skill end-to-end before committing to the registry approach.
5. **🟡 Operational burden kills adoption.** *Mitigation:* progressive tool profiles (fast/semantic/deep), containerized one-command bootstrap.
6. **🟢 Temporal model over-engineering.** *Mitigation:* defer to Phase 4+; design IR *forward-compatible* but don't build history stores early.

### Non-Goals (recommended)
- Do NOT build a custom multi-language parser/indexer (reuse CLIs).
- Do NOT run deep analysis (Joern/CodeQL) per change (on-demand only).
- Do NOT require runtime telemetry for MVP (static + declared only).
- Do NOT make Mermaid the canonical C4 representation.
- Do NOT store durable knowledge only in conversation history.

---

## 5. Architecturally Distinct Options

### Option A — "Plugin-First, No-Rust-First" (RECOMMENDED for MVP)
Validate the core loop using **OpenCode-native agents + skills + a thin TS plugin**, zero Rust.

```
OpenCode primary agent (TS plugin: project-resolve + evidence-path guard)
   ↓ delegates to
subagents (markdown-defined) calling
   existing CLIs via bash + wrapped skills (lmammino, cheriftj)
   ↓ produce
Evidence ledger (JSONL) → thin IR (JSON) → Structurizr/PlantUML projection
```
- **Pros:** Fastest iteration; no compile cycles; skills load natively; validates the hypothesis cheaply; XDG storage via a small TS/CLI shim; reversible.
- **Cons:** TS plugin is slower for heavy normalization; weaker type safety than Rust; not a "product."
- **Effort:** Low–Medium. **Irreversibility:** Low.

### Option B — "Rust Core + TS Plugin" (the document's design)
The full `archctl` Rust capability-router + normalizer + IR + state machine, with a TS OpenCode plugin shim.
- **Pros:** Fast deterministic normalization; strong types; long-term product-grade; clean CLI surface (`archctl ...`); content-addressed cache.
- **Cons:** High upfront cost; Rust↔TS IPC boundary; slower to change the IR schema; commits to a product before validation.
- **Effort:** High. **Irreversibility:** Medium-High.

### Option C — "Deliberately Simple: One Agent + Skill Stack"
No platform. Install the best 2–3 existing skills (lmammino + cheriftj + plantuml), wrap with a *single* `SKILL.md` that adds the evidence-discipline prompt + Structurizr-as-source rule. Use OpenCode's native task delegation only.
- **Pros:** Days not months; ships value immediately; minimal deps; honest to the original request.
- **Cons:** No persistent evidence ledger; no temporal model; less reliable on huge repos; no drift detection.
- **Effort:** Low. **Irreversibility:** Very Low.

### Option D — "Skill-as-Code SDK Headless" (document §23)
`archctl orchestrate` drives the OpenCode server/SDK headlessly, SSE events, JSON-Schema-validated subagent outputs.
- **Pros:** Fully automated, CI-embeddable, reproducible.
- **Cons:** Most complex; depends on unverified SDK/SSE stability; overkill until Option A proves out.
- **Effort:** Very High. **Irreversibility:** High.

---

## 6. Recommended MVP & Explicit Cuts

**Build (Option A, ~4–6 wk to first signal):**
1. XDG project-resolver — **discriminated `SourceIdentity`** (git mode: `BLAKE3(normalized_remote + root_commit)`; directory mode: `BLAKE3(canonical_realpath)`) + **portable `projectId`** re-bound on import *(✅ RESOLVED downstream ADR-0003; original exploratory note was Git-only)* — small TS/CLI shim. Git is an *optional capability adapter*, not a universal prerequisite.
2. Evidence ledger v1 (JSONL: path/lines/hash/extractor/confidence/classification).
3. Thin Architecture IR v1 (elements, relationships, confidence, evidence-refs) — **forward-compatible with temporal fields but no history store yet**.
4. **One** wrapped skill end-to-end (lmammino c4 → Structurizr projection) to prove the loop.
5. Fast profile only: Git + ast-grep outline + ctags + one build-tool (cargo OR dep-cruiser).
6. Structurizr **`local` viewer** for workspace inspection plus a pinned headless validation/export command (~~Structurizr Lite container~~ — **Lite is EOL**; ✅ RESOLVED downstream ADR-0005); PlantUML local for UML.
7. Basic auditor subagent (refute, no separate falsifier yet).

**Cut from MVP (defer):**
- Rust core (move to Phase 3 *only if* Option A validates).
- Temporal twin / history (Phase 4).
- Declared-vs-observed diff (needs telemetry; Phase 5).
- Falsifier agent (Phase 4).
- Semantic (SCIP/LSP) + deep (Joern/CodeQL) profiles (Phase 2/5).
- Control-plane plugin richness — keep only the project-resolve + write-guard hooks.
- CI/CD drift gates (Phase 4).

**Validation gate before any further investment:** measure on **2 real repos** (1 small Rust, 1 medium TS) against invariant assertions (elements present, zero unsupported claims, render succeeds). If unsupported-claims > 0 on medium repo, the hypothesis is in doubt → reconsider.

---

## 7. Lateral-Thinking Opportunities (the document's best ideas)

1. **Architecture intelligence as a temporal evidence system.** (§18) Storing `validFrom/validTo/firstObservedAt/lastVerifiedAt` per relationship turns the model into a *digital twin of architecture over time*. Strong, original. **Caveat:** this is a Phase 4+ capability; build the IR *schema-compatible* now, defer the store.
2. **Declared / static / observed graph separation.** (§19) Three graphs with set-difference queries (declared−observed = unused; observed−declared = drift) is genuinely insightful and the strongest analytical feature. **Caveat:** observed graph requires telemetry most repos lack; static+declared is the realistic MVP.
3. **Active-learning questioning.** (§21) Ask humans only when `impact × uncertainty × cost-of-error` exceeds a threshold. Excellent UX principle; avoids the "endless questionnaire" failure of cheriftj-style skills.
4. **Renderer independence.** Neutral IR → multiple projections. The correct architectural instinct and the document's most defensible design decision.
5. **Adoption path.** XDG "mirror of the repo" that never touches Git is a strong differentiator for enterprise adoption (clean repos, shareable sidecar repos). Validates the user's explicit "don't pollute Git" constraint.
6. **Skeptical/falsifier agent.** (§20) An agent that *tries to disprove* the model rather than improve it. Sound epistemic hygiene; aligns with the "evidence vs inference" ontology.

---

## 8. Domain Glossary & Unresolved Ambiguities

| Term | Working Definition | Status |
|---|---|---|
| Architecture IR | Neutral, renderer-independent model: elements + relationships + confidence + evidence refs | ✅ Resolved |
| Evidence | A source-grounded observation (path/lines/hash) with extractor + confidence | ✅ Resolved |
| Fact / Inference / Hypothesis / Unknown / Conflict | Confidence/classification ontology | ✅ Resolved |
| Declared graph | What docs/ADR/IaC say exists | ✅ Resolved |
| Static graph | What code/imports/contracts imply | ✅ Resolved |
| Observed graph | What runtime traces show | ✅ Resolved |
| Capability router | Maps abstract capabilities (extract.symbols) to concrete tool adapters | ✅ Resolved |
| Repository vs Clone vs Worktree | Logical id / physical copy / branch+HEAD view | ✅ Resolved |
| **SourceIdentity** | Discriminated project identity (`git` mode: repo/worktree ids; `directory` mode: directory id). **Portable `projectId`** is re-bound to a local SourceIdentity on import. *(Corrects the original Git-only exploratory note ✅ RESOLVED ADR-0003.)* | ✅ Resolved |
| **"Architecture intelligence platform"** | Ambiguous — single product? harness? OpenCode distribution? | ⚠️ **Unresolved** |
| **Canonical C4 store** | Structurizr DSL vs the neutral IR — which is *the* source? Doc says both at different points | ⚠️ **Unresolved** |
| **MVP boundary** | Doc lists a 5-phase roadmap but never a crisp "what's the first deliverable that proves value" | ⚠️ **Unresolved** |
| **Confidence calibration** | How are confidences assigned/validated? No method given | ⚠️ **Unresolved** |

---

## 9. Entropy Protocol A — Connascence Landscape (heuristic, greenfield docs only)

**Method: heuristic · Confidence: low (design document, no code).**

This assesses the *proposed design's* coupling, since no code exists.

| Component A | Component B | Connascence Type | I(bits) est. | Severity |
|---|---|---|---|---|
| Architecture IR schema | All agents/skills/renderers | Name + Type | ~3.5–4.5 | ❌ High (central hub — expected, but high-risk surface) |
| Evidence ledger schema | Synthesizer + auditors + IR | Type + Meaning | ~2.5–3.0 | ⚠️ Medium |
| Capability router | Tool adapters | Type (via registry) | ~0.8 | ✅ OK (OCP: add adapters freely) |
| Rust core ↔ TS plugin | IPC contract (JSON over stdio/CLI) | Position + Value | ~2.5 | ⚠️ Medium (the cross-language seam) |
| Pipeline stages (evidence→IR→view→render→audit) | Each stage | Execution + Position | ~3.0 | ⚠️ Medium (inherent to pipeline; mitigated by checkpoint) |
| OpenCode plugin API | archctl | Meaning (undocumented assumptions about hook semantics) | ~1.5–2.0 | ⚠️ Medium (version drift) |

- **Critical pairs (I > 3.0):** the **IR schema hub** is the single highest-coupling surface. Any IR change propagates everywhere. *Recommendation:* version the IR (`schemaVersion`), keep it minimal, treat schema evolution as a first-class concern.
- **Hidden connascence:** Meaning-coupling between the plugin's assumed OpenCode hook semantics and the actual API — invisible until an OpenCode version bump breaks it.
- **OCP assessment:** The capability-router/adapter-registry is genuinely OCP-compliant (add tools without modifying the router) — the design's strongest entropy property.
- **Design Quality Score (estimated):** **~0.45 (🟡 ACCEPTABLE)** — per-component SOLID discipline is good; system-level complexity (9 agents + 14 skills + 10 tools + 2 languages) drags cohesion down. *Confidence: low.*
- **Recommendation:** The IR and the Rust↔TS seam are the two entropy hotspots to design carefully; everything else is acceptably decoupled *if scope is held*.

---

## 10. Auto-Grill de las Afirmaciones de la Exploración (Español)

> Grilling de *mis propias* afirmaciones de exploración. No escalo asuntos cosméticos. Solo desafío lo que afecta a decisiones.

**Pregunta 1 — "El documento sobredimensiona el alcance 10×."**
- *Resolución:* **Auto-resuelta (confianza 0.9).** Evidencia: la petición original (línea 7) pide "skills para diagramación C4/UML"; el documento propone 9 agentes + núcleo Rust + gemelo temporal. La discrepancia es objetiva y medible. No es escalable a discusión: es un hecho del texto.

**Pregunta 2 — "La investigación citada no valida la ingeniería inversa fiable desde código."**
- *Resolución:* **Auto-resuelta (confianza 0.85).** Verifiqué ambos papers: 2510.22787 toma como entrada un *system brief* (texto), no un codebase; Code2UML es "visualization". Ninguno demuestra RE fiable en repos reales grandes y multilenguaje. Afirmación sólida.

**Pregunta 3 — "Las skills son de Claude Code, no de OpenCode nativo."**
- *Resolución:* **Auto-resuelta (confianza 0.9).** Los repos tienen `.claude-plugin/` y `commands/`. Es estructural. *Matiz honesto:* OpenCode sí descubre skills en `.claude/skills/` (constatado en el skill-registry local), así que hay compatibilidad parcial — mi afirmación debe suavizarse: la adaptación es **real pero acotada**, no un muro.

**Pregunta 4 — "Rust se compromete demasiado pronto."**
- *Resolución:* **Escalada con recomendación (confianza 0.6).** Es una decisión de negocio/técnica con trade-offs reales (tipado fuerte + performance vs. velocidad de iteración). No la puedo resolver solo: depende de si el usuario quiere un *producto* o *validar primero*. Recomiendo Option A (sin Rust) primero; Rust sólo si Option A valida. El usuario debe decidir.

**Pregunta 5 — "DQS estimado ~0.45."**
- *Resolución:* **Marcada como baja confianza.** Es una estimación heurística sobre un documento de diseño, no sobre código. No la uses como base de decisión dura; úsala solo como señal direccional de "el sistema es complejo y conviene recortar alcance."

**Pregunta 6 — Afirmaciones no verificadas (subagent_depth, OPENCODE_CONFIG_DIR, servidor/SSE, SCIP governance).**
- *Resolución:* **Parcialmente resuelta.** Las claims de **OpenCode** (top-level `mcp`, `subagent_depth`, skills/references/agent/plugin/permission) fueron **verificadas contra el schema/docs oficial en vivo** — ✅ RESOLVED downstream ADR-0007. La implementación sigue **pinneando una release y haciendo schema-tests** antes de depender de cualquier hook. Queda **pendiente de verificación externa**: la afirmación de **governance de SCIP (2026)** y la estabilidad del **servidor/SDK con SSE** — marcar solo esas como no bloqueantes para spec.

**Resumen del grill:** 4 afirmaciones auto-resueltas con evidencia sólida; 1 escalada (decisión Rust — requiere al humano); 1 marcada baja-confianza (DQS); ~~1 grupo escalado (claims de OpenCode/SCIP)~~ → **claims de OpenCode ahora RESUELTAS por verificación en vivo; solo SCIP-governance queda pendiente**. Ningún asunto cosmético escalado. La afirmación más vulnerable de mi exploración es el DQS numérico — trátalo con escepticismo.

---

## 11. Inputs Required Downstream

### For `sddk-propose`
- **Decision needed from user:** Option A (no-Rust-first) vs Option B (Rust core) vs Option C (skill-only). Recommend A.
- Scope statement: which repos/languages the MVP targets (recommend 1 Rust + 1 TS).
- Confirm non-goals (temporal model, observed graph, falsifier, deep analysis deferred).
- Confirm storage: XDG-only (recommended) vs in-repo `.architecture/`.

### For `sddk-spec`
- IR v1 schema (minimal: elements, relationships, confidence, evidence-refs, schemaVersion).
- Evidence ledger v1 schema.
- Invariant-based acceptance criteria (not pixel-diff): elements-present, zero-unsupported-claims, render-succeeds.
- Capability descriptor format (yaml) for adapters.

### For ADRs
- **ADR-1:** Plugin-first/no-Rust-first vs Rust core (defer Rust until validation).
- **ADR-2:** Canonical C4 source = Structurizr DSL *as a projection* of neutral IR (resolve the doc's ambiguity: IR is truth, Structurizr is canonical view).
- **ADR-3:** XDG external storage + **discriminated `SourceIdentity` (`git` \| `directory`)**: git repo id = `BLAKE3(normalized_remote + root_commit)` (sharable); worktree id = `BLAKE3(repository_id + realpath(show_toplevel))`; directory id = `BLAKE3(canonical_realpath)` (local-only); **portable `projectId`** carried in export bundles, re-bound on import. *(Corrects the original Git-only `BLAKE3(remote+root_commit)` input; ✅ RESOLVED downstream.)*
- **ADR-4:** Renderer independence policy (IR → multiple projections; Mermaid not canonical).
- **ADR-5:** Tool reuse-over-rebuild (no custom parsers/indexers).
- **ADR-6:** Evidence ontology (fact/inference/hypothesis/unknown/conflict) + confidence provenance.

### For ROADMAP
- Phase 0 (spike): Option A on 2 real repos → validation gate.
- Phase 1: XDG resolver + evidence ledger + thin IR + 1 wrapped skill + Structurizr projection.
- Phase 2: capability router + fast profile (ast-grep/ctags/build-tools) + basic auditor.
- Phase 3: Rust core (conditional on validation) + OpenCode control-plane plugin.
- Phase 4: temporal model schema + drift diff (static/declared) + CI gates + falsifier.
- Phase 5: observed graph (telemetry) + deep analysis (Joern/CodeQL) + headless SDK.

---

## Affected Areas
- `Skills-para-agentes-IA.md` — the source design document under review (entire document).
- `.atl/skill-registry.md` — confirms OpenCode skill-discovery paths (supports partial Claude-Code skill compat).
- `sddk/architecture-intelligence-platform/` — this report (new planning artifact).

## Risks
- 🔴 Core reverse-engineering loop may not be reliable enough on real repos (unvalidated).
- 🔴 Build-before-validate if Option B is chosen prematurely.
- 🟡 OpenCode version drift breaks plugin hooks/config.
- 🟡 Claude-Code skill adaptation surface.
- 🟡 Operational burden of the full toolchain.

## Ready for Proposal
**Yes — conditionally.** Proceed to `sddk-propose` with **Option A (plugin-first/no-Rust-first)** as the recommended path. The orchestrator should tell the user: the document is a strong design compass but a 10× oversized build plan; the research is real but doesn't validate the hardest part; recommend validating the core loop cheaply before any Rust investment. **One user decision required:** Rust-now vs Rust-later (recommend later).

---

## Standard Envelope
- **status:** success
- **executive_summary:** The document is architecturally literate, source-grounded (citations verified real), and innovative (temporal twin, graph separation, falsifier), but 10× oversized, unvalidated on its hardest claim (reliable codebase reverse-engineering), and front-loads irreversible choices (Rust). Recommend a plugin-first/no-Rust-first MVP (Option A) to validate the core loop on 2 real repos before platform investment.
- **context_quality:** C0
- **taxonomy:** dominant axes — `build-vs-buy` (reuse CLIs/skills vs build), `boundary_seam` (Rust↔TS), `coupling_connascence` (IR hub), `mvp_scope` (10× expansion), `temporal_evidence` (deferred capability)
- **artifacts:** `sddk/architecture-intelligence-platform/explore-report.md` (this file); Engram observation under `sddk/architecture-intelligence-platform/explore`
- **next_recommended:** `sddk-propose` (Option A) → `sddk-spec` (IR/ledger schemas, invariant acceptance) → ADRs 1–6
- **risks:** see above
- **skill_resolution:** sddk-explore (executed), entropy-sdd Protocol A (heuristic, executed), auto-grill (text-mode, Spanish, executed), cognitive-doc-design (applied to report structure)
