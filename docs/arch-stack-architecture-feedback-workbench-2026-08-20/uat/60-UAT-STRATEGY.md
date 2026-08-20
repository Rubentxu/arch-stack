# UAT Strategy

## Objetivo
Validar resultados humanos, no sólo widgets.

## Authority order
```text
deterministic data/DOM assertions
        >
human task correctness
        >
human subjective measures
        >
multimodal LLM advisory
```

## Pyramid
Rust unit/invariant → full-vs-incremental differential → contract/schema → Criterion → Vitest → Playwright → Human UAT → LLM CUA advisory.

## A/B principle
Cuando una representación afirma mejorar comprensión, comparar con baseline: text report, node-link o workflow anterior.
