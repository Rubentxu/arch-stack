# ADR-049 — Separar Observation, Evidence y Claim

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Si varias estrategias detectan la misma relación, un único confidence puede ocultar
qué fue observado, inferido o contradicho. Explain y fusion necesitan cada señal.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Metamodelo: `SourceArtifact`, `Evidence`, `Observation`, `Claim`, arista
`supports|contradicts` y `Confidence` recalculable. Observation registra
producer/version/method.

## Superficie propuesta

```text
SourceArtifact <- Evidence <- Observation -(supports/contradicts)-> Claim
```

## Rationale y beneficios

Permite fusion, contradicciones, explain, confidence/coverage y evolución de
extractores sin perder procedencia.

## Costes y consecuencias negativas

Migración de schema y más nodos/aristas. Evitar falsa precisión en scores.





## Estrategia de migración

Añadir tipos sin borrar legacy; dual-write; backfill; cambiar readers; retirar
legacy tras paridad.

## Verificación y criterios de aceptación

- 2 extractores apoyan sin sobrescribir;
- contradicción visible;
- explain lineage completo;
- confidence recalculable;
- borrar observation no elimina otras.

## Alternativas consideradas

A) confidence por edge: insuficiente.
B) provenance en props: difícil de consultar.
C) probabilistic KB completa: exceso.

## Referencias internas

evidence.rs, metamodel-core.json, ADR-005/009/027.

## Changelog

- 2026-08-13 | proposed | ADR-049 creado a partir de la auditoría de consolidación.
