# ADR-055 — Sanitized Architecture Bundle compartible

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Para onboarding/consultoría/agentes interesa compartir arquitectura sin entregar
source, paths sensibles, secretos ni DB completa.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

`.archbundle` versionado deny-by-default: manifest, graph slice, claims/evidence
metadata sanitizada, policies, capabilities, optional snapshots/diffs, checksums.
Source bytes excluidos por defecto.



## Rationale y beneficios

Portable, offline, reproducible y local-first.

## Costes y consecuencias negativas

Sanitización puede filtrar nombres sensibles; requiere allowlist/scanner.





## Estrategia de migración

MVP export strict manual; archview read-only; profiles custom/firma después.

## Verificación y criterios de aceptación

- scanner malicioso;
- no source;
- no absolute paths;
- redaction manifest;
- hash;
- import read-only.

## Alternativas consideradas

A) zip viewer bundle: sin privacy contract.
B) DB export: filtra.
C) cloud share: contradice local-first.

## Referencias internas

projection bundle, evidence model, ADR-004/049.

## Changelog

- 2026-08-13 | proposed | ADR-055 creado a partir de la auditoría de consolidación.
