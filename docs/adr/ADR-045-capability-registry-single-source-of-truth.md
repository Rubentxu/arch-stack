# ADR-045 — Capability Registry como fuente única de verdad

> **Cycle:** `p-38e02210a9f14317/p1-08-capability-registry`
> **Status:** Aceptado — 2026-08-16
> **Supersedes:** ADR-044 §Puertos (persistence ports section)
> **Aplica a:** `archctl/src/capability/*.rs`, `archctl/src/cli.rs`, `docs/CAPABILITIES.md`, `scripts/verify-local.sh`

## Contexto

Feature/language support evoluciona a ritmos distintos. Call Graph, Class Diagram y
State Machine no tienen la misma matriz; comentarios/errores históricos pueden quedar
desincronizados de la implementación. Sin un registro centralizado, la única forma de
conocer qué capabilities existen era inspeccionar el código fuente.

## Decisión

Crear `CapabilityRegistry` tipado/serializable con capability, language, maturity,
requirements, determinism, schema y availability. CLI, doctor, MCP, docs y archview
consumen el mismo contrato.

### 1. Modelo de dominio (`archctl/src/capability/mod.rs`)

```rust
pub struct Capability {
    pub capability: String,         // e.g. "code.call_graph"
    pub language: Option<String>,   // None = universal
    pub maturity: Maturity,         // alpha | beta | stable
    pub requirements: Vec<String>,  // e.g. ["ast-grep", "ctags"]
    pub deterministic: bool,
    pub schema: Option<String>,    // e.g. "call-graph-report/1"
    pub availability: Availability, // cli | mcp | plugin | render | ide | doctor
}
```

`CapabilityRegistry::registry()` devuelve `&'static [Capability]` — single source of truth
para toda la herramienta.

### 2. CLI `capabilities` (`archctl/src/capability/cli.rs`)

| Flag | Output |
|---|---|
| (none) | Resumen tabular: capability · language · maturity |
| `--json` | Array de `Capability` serializado |
| `--check` | Exits 0 si `docs/CAPABILITIES.md` está sincronizado con registry |

### 3. Documentación generable (`archctl/src/capability/docs.rs`)

`render_markdown()` produce una tabla Markdown con columnas:
`Capability | Language | Maturity | Requirements | Deterministic | Schema | Availability`

`docs/CAPABILITIES.md` se genera via `archctl capabilities --format markdown > docs/CAPABILITIES.md`.

### 4. Gate de staleness (`scripts/verify-local.sh`)

```bash
archctl capabilities --format markdown | diff -q docs/CAPABILITIES.md - >/dev/null 2>&1
run_gate "P1-08 capabilities docs up-to-date" $?
```

Si `docs/CAPABILITIES.md` está desincronizado, el gate falla con exit non-zero.

### 5. Staleness check en CI (`scripts/test-ci-gates.sh` §9c)

Tres tests:
- `verify-local.sh contains capabilities staleness gate` — grep
- `capabilities --check exits 0 when fresh` — require
- `capabilities --check exits non-zero when stale` — require_not

## Superficie implementada

```json
{"capability":"code.call_graph","language":"kotlin","maturity":"beta",
 "deterministic":true,"schema":"call-graph-report/1"}
```

Ejemplo completo en `docs/CAPABILITIES.md` (84 líneas, 8 categorías).

## Rationale y beneficios

- Elimina matrices duplicadas en README.md y MANUAL.md
- Habilita feature negotiation (MCP puede consultar registry)
- Maturity visible y auditable por tooling
- Extensión por plugins vía `Capability` struct

## Costes y consecuencias negativas

Riesgo de mega-config; debe describir capacidades, no wiring. Mitigado:
capabilities son datos, no comportamiento; el registry es `&'static [Capability]`
sin lógica de negocio.

## Estrategia de migración

1. Registrar estado actual sin cambiar comportamiento (completado)
2. Alignment tests via golden fixture `capability_markdown_golden.txt`
3. Exponer `archctl capabilities --json` (completado)
4. Generar `docs/CAPABILITIES.md` (completado)
5. Gate de staleness en verify-local y test-ci-gates (completado)

## Verificación y criterios de aceptación

- [x] `cargo test --features test-fixtures --quiet` — todos los tests pasan
- [x] `archctl capabilities --check` exits 0 con docs/CAPABILITIES.md sincronizado
- [x] `archctl capabilities --check` exits non-zero tras editar CAPABILITIES.md manualmente
- [x] Golden test en `archctl/src/capability/docs.rs` reproduce exactamente el output de `--format markdown`
- [x] Symlink `archctl/docs → ../docs` permite `--check` desde cualquier cwd

## Alternativas consideradas

A) Enums locales: drift inevitable sin fuente centralizada.
B) Reflexión automática: no expresa maturity ni requirements.
C) YAML-only: posible, pero el core debe conservar typing.

## Referencias internas

`archctl/src/capability/*`, cognitive MCP, manifests, docs/specs/index.md.

## Changelog

- 2026-08-13 | Propuesto | ADR-045 creado a partir de la auditoría de consolidación.
- 2026-08-16 | Aceptado | Implementación completa en cycle P1-08. Registry tipado, CLI --json/--check/--format markdown, golden test, staleness gate en verify-local y test-ci-gates.
