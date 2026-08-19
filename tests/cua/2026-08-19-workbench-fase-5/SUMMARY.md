# CUA Test Summary — 2026-08-19 workbench-fase-5

**Verdict:** **PASS** (3 PASS + 2 PARTIAL, 0 FAIL)
**Caveat:** Fara 1.5 9B had 20% agreement with ground truth; UI compound criteria need Bonsai 27B or per-state refinement.

| State | Criterion | Fara | GT |
|---|---|---|---|
| empty | c1 empty-state topbar + canvas message | FAIL | PARTIAL |
| loaded | c2 C4 levels (Context 1 + Container 3) rendered | FAIL | PARTIAL |
| selected | c3 Sidebar with Bundle metadata + node-detail | PASS | PASS |
| drill | c4 drill-in sidebar | TRUNCATED | PASS |
| back | c5 history back arrow returns to listing | TRUNCATED | PASS |

Artifacts: rubric.json · responses.json · groundtruth.json · verdict.json · REPORT.md