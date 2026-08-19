# ADR-055 — Sanitized Architecture Bundle compartible

> **Estado:** Cerrado — 2026-08-18 (implementado: fase 1 strict bundle v1.61.0, fase 2 scanner anti-secretos v1.63.0, fase 3 entropía Shannon v1.67.0)
> **Superseded by**: ADR-061 (reconsideración de triggers)
> **Reopen trigger (original)**: ADR-019 perf budget breach (bundle >10MB) AND ≥1 external-distribution consumer requesting redacted form. **Este trigger ha sido reemplazado.**
> **Reopen trigger (nuevo)**: ≥1 stakeholder (interno o externo) que necesite compartir arquitectura sin código fuente. Ver ADR-061 para justificación.
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Baseline actual:** `main@dfdb3bf` (v1.60.0-pre)
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Decisión original (2026-08-13)

`.archbundle` versionado deny-by-default: manifest, graph slice, claims/evidence
metadata sanitizada, policies, capabilities, optional snapshots/diffs, checksums.
Source bytes excluidos por defecto.

## Decisión de reconsideración (2026-08-18)

Los triggers originales de reopen eran demasiado restrictivos:

| Trigger original | Problema | Nuevo trigger |
|-----------------|----------|---------------|
| ADR-019 perf breach (>10MB) | Causar breach = **regresión**, no trigger de feature | **Eliminado** |
| ≥1 external-distribution consumer | Excluye auditoría interna legítima | **≥1 stakeholder (interno o externo) que necesite compartir arquitectura sin código fuente** |

**Justificación** (detallada en ADR-061):
- El riesgo de NO implementar es Crítico (filtración de secretos en bundles)
- El caso de uso está documentado en el propio ADR: "Para onboarding/consultoría/agentes"
- Un consultor externo bajo NDA califica como stakeholder legítimo
- El MVP es mínimo y no afecta el comportamiento por defecto

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favor determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.
- **Nuevo**: Sanitización deny-by-default para compartir bundles con terceros (auditores, consultores, stakeholders).

## Estrategia de migración

MVP export strict manual; archview read-only; profiles custom/firma después.

## Verificación y criterios de aceptación

- [ ] `--profile strict` exporta sin source paths, secrets, absolute paths
- [ ] Allowlist de metadata definido y documentado
- [ ] Scanner detecta secretos conocidos y los excluye
- [ ] Checksum SHA-256 del bundle para verificar integridad
- [ ] archview abre bundle strict en modo read-only
- [ ] 0 regression en bundle size para perfil default (no-strict)

## Changelog

- 2026-08-13 | proposed | ADR-055 creado a partir de la auditoría de consolidación.
- 2026-08-18 | deferred | Diferido con triggers demasiado restrictivos (external consumer + perf breach)
- 2026-08-18 | reopened | Triggers reemplazados por ADR-061: stakeholder interno/externo ahora cuenta
