# ADR-048 — LadybugDB como adapter nativo con matriz de compatibilidad

> **Estado:** Aceptado — 2026-08-13 (shipped as `archctl doctor --scope storage [--json]`, v1.42.0, PR #174; 5-axis JSON envelope `archctlVersion` + `lbugCrateVersion` + `native` + `targetCompilerStdlib` + `findings[]`)
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

El build de release auditado falló en bindings C++ de `lbug` por `<format>` ausente;
además una dependencia nativa mutable/versionada aparte puede desacoplar crate,
headers, compiler y ABI. El store es infraestructura crítica.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Encapsular LadybugDB detrás de adapter nativo con versions pinned, source digest,
compatibility probe, `doctor --scope storage`, builds nativos por OS y smoke
CRUD/migrations por target.

## Superficie propuesta

```text
archctl doctor --scope storage
  archctl, lbug crate, native library, c++ stdlib, db schema, status
```

## Rationale y beneficios

Aísla el riesgo, da errores accionables y hace release reproducible.

## Costes y consecuencias negativas

Mantener la matriz cuesta y puede exigir toolchains nuevos.





## Estrategia de migración

Recuperar build → registrar tuple exacta → probe → mover imports detrás del adapter
→ release gate.

## Verificación y criterios de aceptación

- doctor muestra crate/native/schema/toolchain;
- mismatch falla antes de DB;
- Tier-1 smoke;
- domain/application sin lbug;
- no artifact `latest` mutable.

## Alternativas consideradas

A) build from source siempre: caro.
B) cambiar DB ahora: no justificado.
C) latest mutable: no reproducible.

## Referencias internas

Cargo.toml, store.rs, DATA-MODEL-LADYBUGDB, release, ADR-005/010.

## Changelog

- 2026-08-13 | proposed | ADR-048 creado a partir de la auditoría de consolidación.
