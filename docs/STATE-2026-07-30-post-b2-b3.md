# Estado de `archctl` — Post B2 y B3 (ADR-016)

> Snapshot de cierre tras implementar bloques B2 (manifest + gates)
> y B3 (SourceOrigin en Evidence y TSG) del plan de mejoras
> propuesto en `docs/ADR-016-activegraph-packs-investigacion.md`.
>
> El snapshot anterior (`snapshot/pre-activegraph-investigation`,
> `aa171cd`) sigue siendo válido como punto de retorno previo a la
> investigación de `activegraph-packs`.

**Fecha**: 30 de julio de 2026
**Branch**: `feat/m5-gix-identity`
**HEAD**: `8758fd1`
**Tag de recuperación**: `snapshot/post-b2-b3` apunta a `8758fd1`.

---

## Resumen ejecutivo

| Métrica | Valor |
|---|---|
| Tests | **89 verde** (87 unit + 2 nuevos de SourceOrigin) |
| Warnings | 0 |
| Branch | `feat/m5-gix-identity` |
| Commit HEAD | `7800d07` (state) → `b7276ee` (workflow doc) → `6fa3920` (hook) |
| Último release conceptual | M8 + Ola 1 ports hexagonales + **B2 (scope gates) + B3 (SourceOrigin)** + **trunk-base enforcement** |
| Siguiente paso planeado | B1 (grafo canónico de evidencia) o Refactor 1b (Filesystem port) |

> **Actualizado el workflow** en el mismo ciclo con `docs/git-trunk-base.md` + `commit-msg` hook. Ver § "Workflow codificado" abajo.

## Cambios en este ciclo

### B2 — Manifest + static gates (`debcf82`)

Bloque B2 del plan de mejoras ADR-016. Los manifests declaran
el contrato de un scope en TOML y `archctl doctor --check-scope`
verifica que el contrato se cumple antes de tocar el código.

- Módulo nuevo `archctl/src/scope.rs` (878 LOC, 18 tests).
- 4 gates: `editable_files_exist`, `public_symbols_exist`
  (whole-word regex sobre `pub struct|trait|enum|fn|const|static|type|use`),
  `must_hold_invariants` (literal pattern), `test_count_meets_minimum`
  (suma `passed` de todos los `cargo test` summaries).
- `cargo_dir = "archctl"` opcional para monorepos donde
  `manifests/` vive al lado del workspace, no al lado del crate.
- `archctl doctor --check-scope [--cwd <path>]` subcomando nuevo.
  `archctl doctor` sin flag conserva el health-check original.
- 3 manifests reales: `evidence.toml`, `store.toml`, `tsg.toml`.
- `.atl/` gitignored (artefacto local de `gentle-ai skill-registry`).

### B3 — SourceOrigin en Evidence y TSG (`8758fd1`)

Bloque B3 del plan de mejoras ADR-016. Cada `Evidence` ahora
lleva un tag `source_origin: SourceOrigin` no opcional.

```rust
pub enum SourceOrigin {
    UserWorkspace,    // bytes leídos directamente del workspace
    UserInput,        // texto tipeado por el humano
    ToolOutput,       // salida de otra herramienta (TSG, jdeps, ...)
}
```

Decisiones explícitas:

- **Sin variante `Unknown`.** Provenance faltante es una violación
  de invariante y debe fallar aguas arriba, no estamparse un
  default silencioso.
- **`Evidence::source_origin` es requerido** (no `Option`). El
  compilador enforce que todo call-site decida su procedencia.
- **Stamping por productor:** `evidence::extract` (ast-grep)
  → `UserWorkspace`; `evidence::from_tsg_node` → `ToolOutput`.
- **`SourceOrigin::as_str()` es el contrato** que los probes
  `must_hold` del manifest `evidence.toml` utilizan.

Manifests actualizados:

- `evidence.toml` declara `SourceOrigin::UserWorkspace` y
  `SourceOrigin::ToolOutput` como invariantes.
- `tsg.toml` declara `from_tsg_node` como invariante (acoplamiento).

## Tests añadidos en este ciclo

| Test | Qué prueba |
|---|---|
| `source_origin_as_str_is_stable` | El mapping `as_str` es estable: `user_workspace` / `user_input` / `tool_output`. |
| `evidence_construction_requires_source_origin` | Un closure que mapea cada variante a un `EvidenceKind` compila. Si `source_origin` se volviera opcional, este test seguiría compilando pero la nota sobre la pérdida de cobertura queda en el docstring. |

Cobertura ampliada del doctor:

| Test | Qué prueba |
|---|---|
| 18 tests en `scope.rs` | Cubren los 4 gates positivo y negativo + el parser de `cargo test` |
| `parse_test_pass_count_sums_all_summaries` | Suma todas las líneas `test result: ok.` (cargo emite una por binario) |
| Tests de `must_hold` y `editable_files` | Cubren alias TOML (`editable` ↔ `editable_files`) y el patrón flat (sin `[scope]` anidado) |

## Cómo se usa

```bash
cd /var/home/rubentxu/Proyectos/agentesIA/archctl/archctl
cargo run --quiet -- doctor --check-scope
```

Output esperado (con manifests que pasen todas las gates):

```
[OK  ] scope evidence (0 findings)
[OK  ] scope store (0 findings)
[OK  ] scope tsg (0 findings)
SCOPE: OK
```

Exit 0 si todo pasa, exit 1 si algún gate reporta FAIL. Cada
finding se imprime a stderr con su severidad para que tools
puedan parsear sin truncar la salida estructurada.

## Workflow codificado

Trunk-base workflow está formalizado en este repo:

- **`docs/git-trunk-base.md`** — contrato completo: branch naming,
  atomic commits, conventional commits format, anti-AI-attribution
  policy, recovery tags, trunk sync, state handoff.
- **`.githooks/commit-msg`** — hook ejecutable que enforce
  conventional commits + rechaza cualquier forma de AI-attribution
  fuera de inline code blocks. Bypass con `--no-verify` solo tras
  revisión humana.
- **`scripts/install-hooks.sh`** — wire-up idempotente de
  `core.hooksPath = .githooks` (per-clone). Re-ejecutable. Imprime
  un sanity check contra los últimos 50 commits al instalar.

Para activar tras clonar el repo:

```bash
cd /var/home/rubentxu/Proyectos/agentesIA/archctl
bash scripts/install-hooks.sh
```

11 tests cubren el hook (feat, chore, breaking, merge, fixup,
revert, inline-code-mention pasan; subject too long, AI
attribution real, trailing period, missing space fail).

```bash
cd /var/home/rubentxu/Proyectos/agentesIA/archctl/archctl
cargo run --quiet -- doctor --check-scope
```

Output esperado (con manifests que pasen todas las gates):

```
[OK  ] scope evidence (0 findings)
[OK  ] scope store (0 findings)
[OK  ] scope tsg (0 findings)
SCOPE: OK
```

Exit 0 si todo pasa, exit 1 si algún gate reporta FAIL. Cada
finding se imprime a stderr con su severidad para que tools
puedan parsear sin truncar la salida estructurada.

## Lo que NO está hecho (siguiente sesión)

### Refactor 1b — Filesystem port

Baja prioridad ahora que B2/B3 añadieron controles externos.
Queda igual que en `STATE.md` original.

### B1 — Grafo canónico de evidencia (persistencia + grafo)

El plan original proponía investigar el grafo de evidencia.
Sigue siendo la pieza más grande del ADR-016. Decisión
pendiente: ¿seguir con B1 o pivotar a Refactor 1b?

### Métricas de calidad

Clippy como `cargo clippy -- -D warnings` en CI. Compila sin
warnings hoy, pero no hay enforcement automático.

### Manifests adicionales

Hay 12 módulos `pub` en `lib.rs` sin manifest. Cada uno
candidato natural:

- `astgrep`, `clock`, `environment`, `graph`, `identity`,
  `inventory`, `project`, `render`, `row`, `skills`, `telemetry`,
  `xdg`.

Los más útiles a añadir ahora: `clock` (probar que las fusiones
de `SystemClock`/`FixedClock` no eliminan determinismo),
`environment` (probar que `SystemEnvironment` no se cuela al
test binario), `identity` (probar que `resolve_source_identity`
no se rompe con cwd vacío).

## Decisiones pendientes

- **¿B1 o Refactor 1b primero?** B1 desbloquea query de evidencia
  real (`MATCH (e:Evidence) WHERE e.source_origin = 'tool_output'`).
  Refactor 1b mejora testabilidad sin funcionalidad nueva.
  **Sugerencia**: B1 — la visibilidad del grafo es lo que el
  usuario nota.
- **¿`SourceOrigin::UserInput` se usa?** El campo existe pero
  ningún call site lo emite todavía. El primer consumidor natural
  sería el futuro `archctl evidence add --input` (free-form claim).
- **¿Vale la pena un gate `provenance_complete`?** Hoy los
  manifests proban que existe `source_origin` en el código, no
  que toda fila persistida tenga un valor distinto de vacío en
  el grafo. Para probarlo haría falta un `archctl evidence
  audit` que ejecute `MATCH (e:Evidence) WHERE e.source_origin
  IS NULL RETURN count(e)`. Out of scope para B3.

## Archivos clave para retomar la sesión

- `archctl/src/scope.rs` — gates + parsers. Patrón a extender si
  se añaden más gates.
- `archctl/src/evidence.rs` — `SourceOrigin` enum, los dos call
  sites de stamping (ast-grep + TSG).
- `manifests/evidence.toml`, `manifests/store.toml`,
  `manifests/tsg.toml` — ejemplos a copiar para nuevos scopes.
- `docs/ADR-016-activegraph-packs-investigacion.md` — plan
  completo con los 3 bloques propuestos.

## Comandos de recuperación

```bash
# Volver al estado post-B2-B3 (este snapshot)
cd /var/home/rubentxu/Proyectos/agentesIA/archctl
git checkout snapshot/post-b2-b3

# Volver al estado pre-investigación activoGraph (snapshot original)
git checkout snapshot/pre-activegraph-investigation

# Verificar que tests siguen verde
cd archctl
cargo test

# Ejecutar el doctor con manifests
cd ..
cargo run --quiet --manifest-path archctl/Cargo.toml -- doctor --check-scope
```
