# Coherence Report: spec + design → tasks — FINAL (gate 1.6)

**Change:** `architecture-intelligence-platform`
**Gate:** 1.6 — spec + design → tasks (adversarial re-run)
**Coherence Trigger:** `spec+design->tasks`
**Date:** 2026-07-29
**Model:** MiniMax M2.7-highspeed

---

## Coherence Score: 97

**Status:** PASS ✅

---

## Adversarial Check Results (12 items)

| # | Check | Verdict | Evidence |
|---|-------|---------|----------|
| 1 | **Exactly 8 real ADRs, no ghost IDs, canonical four-digit refs** | ✅ PASS | proposal §9: ADR-0001..ADR-0008 (8 total); ADR-0009..0012 removed; deferred decisions in separate table; README: 8 entries; ADR files: 8 (0001-0008) |
| 2 | **Unsupported high-confidence claim threshold globally zero/hard-fail** | ✅ PASS | proposal §6 quality_gates: `unsupported_claims_high_confidence: 0 # HARD FAIL si > 0`; spec R3/R11: `confidence ≥ 0.9` with 0 evidenceRefs = HARD FAIL, no carve-outs; design §2 error model: `QualityGateViolation`; spec R11 table: exact kill thresholds; all ≥0.9 without evidence = immediate pipeline abort |
| 3 | **TypeScript M0-M2 and archctl naming consistent** | ✅ PASS | design §1 table: `archctl-orchestrator`, `archctl-extractor`, `archctl-synthesizer`, `archctl-auditor`; tasks.md: same agent filenames + command `archctl.md`; proposal §5: "archctl" consistently; proposal C3: archctl named as command+binary; ADR-0001: TypeScript-only locked for M0-M2 |
| 4 | **Plural OpenCode filesystem paths, singular JSON config keys** | ✅ PASS | design §4: `.opencode/{agents/, skills/, commands/, plugins/}` (plural dirs); config: `"agent"`, `"command"`, `"plugin"` (singular keys); proposal §7: `mcp` top-level (singular), `skills` (config key, singular form) |
| 5 | **Phase 0 has 4 tasks and Gate Zero genuinely tests adapted skill + produced IR against tiny gold fixture** | ✅ PASS | tasks.md Phase 0: 0.1, 0.2, 0.3, 0.4 (4 checkboxes); proposal §3 Phase 0: 4 items; Gate Zero Part A: skill compatibility; Part B: "run adapted skill against tiny 5-file non-Git fixture with manually labelled gold set, normalize produced output (NOT hand-authored) to minimal IR → workspace.dsl → render locally"; "El gold es hand-authored; el IR producido NO es hand-authored" |
| 6 | **Task total arithmetic equals checkbox count** | ✅ PASS | Phase 0: 4 (0.1-0.4); Phase 1: 12 (1.1-1.12); Phase 2: 16 (2.1-2.16); Deferred: 6 (D1-D6); Total: 38; Standard envelope: `total: 38, phase_0: 4, phase_1: 12, phase_2: 16, deferred: 6` |
| 7 | **Spec/design/tasks align on C4-compatible IR kinds** | ✅ PASS | spec R5: `person | softwareSystem | container | component | codeElement`; design §3 IR schema: same closed set; design §3: "non-canonical distinctions (module, service…) live in tags NOT top-level kinds"; tasks.md: no kind vocabulary changes; projection §3: renderer-neutral C4-compatible taxonomy |
| 8 | **SourceIdentity remains coherent** | ✅ PASS | spec R1: discriminated SourceIdentity (git \| directory); design §3: same discriminated union; ADR-0003: "Git is optional capability adapter"; CONTEXT.md: SourceIdentity + Repository id + Worktree id + Directory id + Portable project id all defined; proposal §8: discriminated SourceIdentity in identity table |
| 9 | **Untrusted repo data, symlink containment, pins, hook runtime probes, ledger single-writer semantics covered without scope explosion** | ✅ PASS | spec R9/R10: repo data = datos (no instructions), read/write contained, symlink escapes rejected; design §7: threat model treats repo as untrusted read; design §4: belt-and-suspenders write-guard; tasks 0.4/1.11/2.2: adversarial fixtures including symlink-escape probe (realpath must fail); tasks 2.2: "realpath containment + reject symlink escapes + atomic temp+rename"; tasks 1.4: "single-writer per-run segments OR safe append semantics (no concurrency ambiguity: each run writes runs/<runId>/evidence.jsonl; cross-run appends sequential under per-project advisory lock)"; tasks 0.4: license check + sandbox; tasks 2.11: MCP executables + extractor binary inventory (version+hash+license); tasks 1.2: smoke probe tests actual hook firing + permission ordering (not just schema presence) |
| 10 | **Structurizr local/view vs headless export wording accurate** | ✅ PASS | proposal C3: "Structurizr local = herramienta self-hosted para visualización (viewing)"; "headless requiere comando pineado soportado, track vNext"; design §6: "Structurizr local: self-hosted workspace viewer (not a generic distribution)"; "for headless DSL validation/export the pipeline pins the currently supported Structurizr CLI command and tracks migration to vNext" |
| 11 | **All ADRs remain Proposed** | ✅ PASS | ADR files: all status "Proposed"; README: all "Proposed"; proposal §9: all "Proposed"; design §12: all "Proposed" |
| 12 | **Spec/design/tasks align on everything else: hooks, confidence method, RunContext, supply-chain, language drift** | ✅ PASS | RunContext.commit: design §5 = identity anchor (git commit or snapshotId); design §3 revision discriminated union matches spec R2; spec R4: `heuristic-v1` declared valid method for Phase 1; ADR-0008: skills.lock.json + license + sandbox + write-guard + allowlist all covered; ADR-0001: TS-only for M0-M2, task 2.16 creates language drift guard script, task 1.2 references it by name (TBD until created); ADR-0005: Lite EOL confirmed, local is viewer, headless is pinned CLI; spec R12: all kill thresholds precise (coverage<.70, render<.80, Jaccard<.80, precision<.70, recall<.60, >200k tokens, >30min) |

---

## Remaining Material Issues

**NONE.** All adversarial checks pass. No blocking issues remain.

### Minor Observations (non-blocking, no remediation required)

| Item | Observation |
|------|-----------|
| ADR-0001 still "Proposed" in ADR files | Correct — user acceptance is the next step per spec |
| Confidence calibration method unresolved | Correct — explicitly an open Phase-1 experiment; `heuristic-v1` is a declared valid method during experiment |
| Exact OpenCode hook signatures unresolved | Correct — version-pin + CI schema-test mitigates; spike validates at runtime |

---

## Bidirectional Traceability Summary

| Axis | Status |
|------|--------|
| Proposal capabilities (5) → spec requirements (14) | ✅ All mapped |
| Spec requirements (14) → design components | ✅ All covered |
| Design components → tasks (38) | ✅ All tasks traceable |
| ADR decisions (8) → design components | ✅ All consistent |
| ADR status: all Proposed | ✅ Confirmed |
| SourceIdentity: spec/design/ADR-0003/CONTEXT | ✅ Coherent |
| Discriminated evidence revision: spec/design/ADR-0003 | ✅ Coherent |
| Kill gates: proposal/spec/design/tasks | ✅ Consistent |
| Structurizr wording: proposal/design/ADR-0005 | ✅ Consistent |

---

## Score Breakdown

| Category | Score | Notes |
|----------|-------|-------|
| 12 adversarial checks | 60/60 | All items verified pass |
| Bidirectional traceability (proposal↔spec↔design↔tasks) | 19/20 | Minor: spec traceability table has R3 appearing twice (not an error, just duplicate row) |
| ADR consistency (8 Proposed, no ghost IDs) | 20/20 | Exactly 8, four-digit canonical refs |
| Kill gates precise | 20/20 | All exact numeric thresholds |
| SourceIdentity coherence | 20/20 | Git optional, discriminated, portable projectId |
| Scope containment (no explosion) | 20/20 | Adversarial cases + single-writer + realpath containment |
| **TOTAL** | **159/160** | **99.4% → normalized 97** |

---

## Artifact References

- `sddk/architecture-intelligence-platform/proposal.md` (propose, id: e324d8fb)
- `sddk/architecture-intelligence-platform/spec.md` (spec, id: 44526a5f)
- `sddk/architecture-intelligence-platform/design.md` (design, id: fe0e8fb8)
- `sddk/architecture-intelligence-platform/tasks.md` (tasks, id: pending)
- `docs/adr/0001-plugin-first-no-rust-first.md` (Proposed)
- `docs/adr/0002-neutral-ir-truth-structurizr-projection.md` (Proposed)
- `docs/adr/0003-xdg-runtime-state-export-bundle.md` (Proposed)
- `docs/adr/0004-evidence-ontology-confidence-provenance.md` (Proposed)
- `docs/adr/0005-renderer-routing-local-first.md` (Proposed)
- `docs/adr/0006-reuse-over-rebuild-capability-adapters.md` (Proposed)
- `docs/adr/0007-opencode-version-pin-schema-contract-minimal-topology.md` (Proposed)
- `docs/adr/0008-supply-chain-pinning-sandbox.md` (Proposed)
- `CONTEXT.md` (all terms resolved)

---

## Verdict

**Score: 97 — PASS ✅**

All 12 adversarial checks pass. The design is coherent, task-ready, and meets all gate criteria:
- Exactly 8 ADRs (no ghost IDs)
- Globally zero hard-fail for unsupported high-confidence claims
- Consistent TypeScript naming (`archctl-*`) and plural OpenCode paths
- Phase 0 has exactly 4 tasks
- Gate Zero genuinely tests adapted skill + produced IR against gold fixture
- Task arithmetic is correct (38 total)
- C4-compatible IR kinds aligned across all artifacts
- SourceIdentity remains fully coherent
- All security/supply-chain requirements covered without scope explosion
- Structurizr local/view vs headless wording is accurate
- All ADRs remain Proposed

**No material issues remain. Pipeline may proceed to task generation.**

---

*Coherence gate 1.6 final | Score: 97 | PASS*
*Persistence: sddk/architecture-intelligence-platform/coherence/spec-design-to-tasks*
