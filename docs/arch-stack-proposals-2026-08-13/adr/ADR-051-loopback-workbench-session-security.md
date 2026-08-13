# ADR-051 — Token de sesión efímero para acciones del workbench

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

El workbench ya escucha loopback y valida paths, pero endpoints con side effects
pueden ser invocados por otros orígenes/procesos locales si conocen el puerto.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Token aleatorio por proceso `archctl view`; exigir en side effects y aplicar
Origin/Host checks. Bootstrap sin persistir secret.



## Rationale y beneficios

Hardening contra cross-origin localhost sin cuentas/RBAC.

## Costes y consecuencias negativas

Más bootstrap y tests.





## Estrategia de migración

Introducir guard; health/static pueden seguir públicos; side effects pasan a auth.

## Verificación y criterios de aceptación

- ≥128 bits;
- no logs persistentes;
- POST/PUT sin token 403;
- path checks siguen;
- bind loopback.

## Alternativas consideradas

A) confiar loopback: insuficiente.
B) OAuth: exceso.
C) Unix socket: navegador/cross-platform.

## Referencias internas

view.rs, view/source.rs, view/editor.rs, ADR-033.

## Changelog

- 2026-08-13 | proposed | ADR-051 creado a partir de la auditoría de consolidación.
