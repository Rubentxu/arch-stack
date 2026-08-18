# ADR-046 — Seguridad de plugins y supply chain

> **Estado:** Aceptado (parcial) — 2026-08-13
> **Acceptance scope (shipped):** plugin tap model via M76 (v1.36.0, PR #152); `archctl plugin install <author>/<plugin>@<version>` con SHA256 verify.
> **Acceptance scope (deferred):** capability gating per plugin origin (B2), signed-plugin attestation path (B3), per-plugin permission scopes beyond `permissions.yaml` baseline.
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

Plugin tap instala material de red localmente. La auditoría detectó namespace XDG
inconsistente, staging antes de asegurar root, checksum remoto opcional e identidad
author/name/version usada como path sin value objects estrictos.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Frontera no confiable: value objects; root bajo Arch Stack; checksum obligatorio
para remoto; extracción segura; manifest de capabilities; trust state
`local|verified|trusted|untrusted`; staging→verify→activate atómico.



## Rationale y beneficios

Reduce traversal/tampering, habilita least privilege y hace auditable el origen.

## Costes y consecuencias negativas

Más fricción de publicación. Hash aporta integridad/reproducibilidad, no autenticidad
plena si tap y hash se comprometen juntos.

## Riesgos y mitigaciones

Limitar además expanded size/file count y rechazar device/FIFO/symlink escapes.



## Estrategia de migración

P0 path/staging/hash/unpack; P1 manifest; P3 firma/trust enforcement. Legacy queda
`legacy-unverified` hasta reinstalar.

## Verificación y criterios de aceptación

- malicious names rechazados;
- tar no escapa staging;
- remote sin hash falla;
- first install funciona;
- current cambia atómicamente;
- inspect muestra source/hash/capabilities.

## Alternativas consideradas

A) HTTPS basta: no.
B) WASM sandbox ya: demasiado cambio.
C) firma GPG obligatoria en P0: puede bloquear adopción.

## Referencias internas

`archctl/src/plugin/mod.rs`, `plugin/install.rs`, ADR-004 y ADR-040 distribution.

## Changelog

- 2026-08-13 | proposed | ADR-046 creado a partir de la auditoría de consolidación.
