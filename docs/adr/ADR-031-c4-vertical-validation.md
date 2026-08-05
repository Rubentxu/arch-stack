# ADR-031 — C4 Vertical End-to-End Validation: Bugs discovered by real-project smoke test

> **Ciclo:** `m26-c4-vertical-validation`
> **Estado:** Aceptado
> **Fecha:** 2026-08-05
> **Complementa:** [ADR-024](ADR-024-element-category-semantics.md) (category semantics)
> **Predecesor del:** [M27 Sandbox + Benchmarks](ROADMAP.md) (datos para v1.0)

## Contexto

[ADR-024](ADR-024-element-category-semantics.md) (v0.14.9) cerró la ambigüedad
de `Element.category` y aplicó el fix de query en `export.rs` + `queries.rs`. Los
402 tests pasaban, pero **ningún test E2E con un proyecto real de GitHub se
había ejecutado**. La suite existente usa `TempDir` + `MockGraphStore` donde
ambos paths de DB coinciden y los errores de schema se silencian con `.ok()`.

Al ejecutar la pipeline C4 completa contra `tokio-rs/axum` (workspace real con
4 crates y ~677 archivos Rust), descubrimos **5 bugs reales** que rompían el
vertical end-to-end. Este ADR documenta cada uno, su causa raíz, y el fix.

## Bugs descubiertos

### B1 — `apply()` usaba `cwd` directo, no `info.project_dir`

**Síntoma:** `archctl code c4-discover --apply` reportaba "Applied: 4 elements
written" pero la base de datos quedaba vacía. `archctl graph query
"MATCH (n:Element) RETURN count(n)"` retornaba 0. `archctl diagram export`
generaba un bundle con `elementCount: 0`.

**Causa raíz:** El CLI pasaba `&cwd` (path del proyecto fuente) a la función
`apply()` del bounded context `code::c4_discover`. Pero `graph query`,
`diagram export` y otros comandos resolvían el path vía
`crate::project::resolve_project(&cwd)` que devuelve `info.project_dir` —
un UUID-based path en XDG (`~/.local/share/archctl/projects/<uuid>/`).

Resultado: discover escribía en `<cwd>/architecture.lbdb`, los lectores leían de
`<XDG>/.../architecture.lbdb`. **Dos DBs distintas**.

**Afectaba a:** `code::c4_discover::apply`, `code::call_graph::apply`,
`code::class_diagram::apply`, `code::state_machine::apply` — todos los 4
extractores.

**Por qué no se detectó antes:** Los tests usan `TempDir` donde ambos paths
coinciden. Los E2E con `assert_cmd` también usan TempDir.

**Fix:** Pasar `&info.project_dir` a todas las funciones `apply()`. Para
`CallGraph`, `Sequence`, `ClassDiagram`, `StateMachine` (que tienen
`cwd: PathBuf` con default `"."`), añadir `let cwd = ctx.resolve_cwd(...)`
explícito en el caller (estaban usando el default sin resolver).

```rust
// cli.rs:code_c4_discover_cmd
let info = crate::project::resolve_project(&cwd.to_string_lossy());
let apply_report = apply_report(&info.project_dir, &report, &*ctx.fs) ...
```

**Severidad:** CRÍTICO — bloquea cualquier `--apply` real.

### B2 — `query_evidence_for_versions` y `query_version_props`: Cypher inválido por IDs sin comillas

**Síntoma:** `archctl diagram export` fallaba con:

```
Parser exception: Invalid input <MATCH (ev:ElementVersion)-[r:SUPPORTED_BY]->
(e:Evidence) WHERE ev.id IN [blake3:>: expected rule oC_SingleQuery (line: 1, offset: 79)
```

**Causa raíz:** Las funciones construían `id_list = safe_ids.join(", ")` y
lo interpolaban en `WHERE ev.id IN [{id_list}]`. Sin comillas, lbug Cypher
interpretaba `[blake3:...` como `[<expr> <op> <expr>]`.

**Fix:** Envolver cada ID en comillas simples:

```rust
let id_list = safe_ids
    .iter()
    .map(|id| format!("'{}'", id))
    .collect::<Vec<_>>()
    .join(", ");
```

**Severidad:** CRÍTICO — bloquea export con evidencia.

### B3 — `write_evidence` silenciaba errores con `.ok()`

**Síntoma:** Después de aplicar, `graph query "MATCH (e:Evidence) RETURN
count(e)"` retornaba 0, aunque `apply_report.evidences_written = 4`.

**Causa raíz:** `c4_discover::write_evidence` ejecutaba un MERGE Cypher que
intentaba `SET ev.status = 'Drafted'` y `SET ev.language = ''`, columnas que
no existen en el schema `Evidence` (definido en
`docs/schema/001_initial_schema.cypher`). El query fallaba y el código
silenciaba con `store.query(&cypher).ok();` — el `evidences_written += 1`
se ejecutaba igualmente, dando reportes mentirosos.

**Fix:**
1. Quitar columnas inválidas (`status`, `language`) del SET.
2. Cambiar `.ok()` por `.context("write_evidence: MERGE Evidence")?` para
   propagar el error (debug, no release).
3. `status` se mantiene en `ev.props` como dato (no columna top-level).

**Severidad:** CRÍTICO — todas las evidencias de c4_discover eran fantasmas.

### B4 — `version_id` collision: todos los containers compartían el mismo ElementVersion

**Síntoma:** Después de apply, `MATCH (e:Element)-[:CURRENT_VERSION]->(v)
RETURN e.canonical_key, v.id` mostraba 4 elementos apuntando al MISMO
ElementVersion `blake3:8c33e22...`.

**Causa raíz:** `write_element_version` calculaba
`version_id = blake3(version_props_str)`. Pero `version_props` solo
incluye `strategy`, `confidence`, `merged_from`, `discovery_schema_version`
— constantes para todos los containers de una misma estrategia. El hash
era idéntico.

**Fix:** Incluir `element_id` (que incluye `canonical_key`) en el input
del hash:

```rust
let version_id = format!(
    "blake3:{}",
    blake3::hash(format!("{version_props_str}:{element_id}").as_bytes()).to_hex()
);
```

**Severidad:** ALTO — rompía la trazabilidad Element → ElementVersion → Evidence
porque el `:SUPPORTED_BY` link apuntaba a un nodo compartido.

### B5 — Inconsistencia `status: "Drafted"` vs `"drafted"`

**Síntoma:** Después de `evidence accept`, el `e.props.status` seguía
siendo `"Drafted"` (con mayúscula). El export filtraba
`status == "accepted"` y retornaba 0 evidencias.

**Causa raíz:** `c4_discover::write_evidence` escribía
`"status": "Drafted"` (capitalizada). Pero `EvidenceStatus::as_str()`
retorna `"drafted"` (lowercase) y `EvidenceStatus::from_props()`
únicamente reconoce lowercase — cualquier otra cosa (incluyendo `"Drafted"`
o missing) cae en el default `_ => Self::Accepted`. Esto significaba que
`accept_evidence` (línea 511) detectaba `current == Accepted` y retornaba
`Ok(())` inmediatamente como "idempotent".

**Fix:** Cambiar `"Drafted"` → `"drafted"` en `c4_discover::write_evidence`
y `call_graph::write_evidence` (también usaba `"Drafted"`).

**Severidad:** ALTO — `evidence accept` era no-op para todas las evidences
generadas por extractores.

### B6 — Bundle schema mismatch: `node.type = "c4"` y `node.status = "active"`

**Síntoma:** `archctl diagram validate <bundle>` retornaba:
```
[nodes/0/status] "active" is not one of "accepted", "drafted" or "superseded";
[nodes/0/type] "c4" is not one of "context", "container" or 3 other candidates;
```

**Causa raíz:** El export serializaba el campo `type` con el valor de
`Element.category` (que tras ADR-024 es `"c4"`) y el campo `status` con
`Element.current_status` (que es `"active"` o `"deprecated"` en el modelo
interno). Pero el schema `diagram-projection.schema.json` define:

```json
"type":   { "enum": ["context", "container", "component", "dynamic", "deployment"] },
"status": { "enum": ["accepted", "drafted", "superseded"] }
```

**Fix:** Añadir dos funciones de mapeo en `export.rs`:

```rust
fn kind_id_to_type(kind_id: &str) -> String {
    // Strip "mt." prefix and namespace; keep last segment.
    kind_id.rsplit('.').next().unwrap_or(kind_id).to_string()
}

fn schema_valid_status(current: &str) -> String {
    match current {
        "active"     => "drafted".to_string(),
        "deprecated" => "superseded".to_string(),
        other        => other.to_string(),
    }
}
```

**Severidad:** ALTO — el bundle generado nunca era válido según el schema,
aunque `diagram export` reportaba éxito. archview no habría podido
consumirlo.

## Validación end-to-end

Después de los 6 fixes, el vertical C4 funciona con `tokio-rs/axum`:

```
$ archctl code c4-discover --strategy cargo-workspace --apply
Applied: 4 elements written, 0 skipped, 4 evidences, 4 artifacts

$ for id in $(archctl graph query "MATCH (e:Evidence) RETURN e.id"); do
    archctl evidence accept --id "$id"
  done
accepted: ev:963b64b0...
accepted: ev:235450e1...
accepted: ev:37d02481...
accepted: ev:cac13e5b...

$ archctl diagram export --output /tmp/axum-bundle "container:*"
Exported 4 elements, 0 edges, 4 evidence

$ archctl diagram validate /tmp/axum-bundle
Bundle /tmp/axum-bundle is valid
```

## Lecciones

1. **Los tests `TempDir` ocultan bugs de path.** El patrón "open DB A, write DB
   B, read DB A" no se detecta cuando A == B. Necesitamos smoke tests con
   proyectos reales fuera de TempDir.

2. **`.ok()` en query fallido oculta bugs.** El patrón
   `store.query(cypher).ok()` convierte errores en éxitos silenciosos y
   rompe los contadores. Reservar `.ok()` solo para queries best-effort
   (edges, meta-type seeds) y propagar errores en writes primarios.

3. **Los unit tests del fix M26 pasaron porque el MockGraphStore no
   valida schema.** Las queries que el código real rechazaba (e.g.,
   `SET ev.status` en columna inexistente) eran aceptadas por el mock sin
   error. Necesitamos integration tests con un store real (LbugStore) en
   el pipeline de CI.

4. **El casing de strings en DB es un contrato frágil.** `parse_label` y
   `as_str` deben ser la única fuente de verdad para el formato. Cualquier
   string literal `"Drafted"` en otro archivo es una bomba de tiempo.

5. **El vertical necesita un test de regression explícito.** El test
   `tests/c4_vertical.rs` que añadí en M26 solo verificaba que el CLI
   arrancara y produjera JSON válido — no verificaba que discover+apply
   persistiera datos. Necesita reescribirse como test de DB real
   (con `LbugStore`).

## Decisiones

### D1 — Tests E2E con proyectos reales (no TempDir)

**Elección:** Crear un harness de smoke testing en `tests/smoke_real_projects.rs`
que clona proyectos reales (en cache `~/.cache/archctl-smoke/`), ejecuta el
vertical completo, y valida contra `diagram validate`. Tests marcados
`#[ignore]` para CI normal, corren en `cargo test -- --ignored` en jobs de
nightly.

**Por qué:** El bug B1 no se habría detectado con TempDir. Necesitamos
probar el path completo en condiciones reales.

### D2 — Debt M27: Sandbox + Benchmarks

**Elección:** Crear M27 — `archctl-bench` (sandbox podman + quadlets +
harness multi-proyecto + métricas FP/FN/tiempo/memoria). Aceptar que v1.0
no puede shippearse hasta tener **datos** sobre comportamiento en
proyectos reales, no solo "los tests pasan".

**Por qué:** Sin métricas reales, "v1.0" es un salto de fe. M27 es la
**primera release candidate** solo cuando los benchmarks demuestren
comportamiento aceptable.

## Trabajo derivado

- [ ] Reescribir `tests/c4_vertical.rs` para usar `LbugStore` real (no
      `assert_cmd` con TempDir).
- [ ] Crear `tests/smoke_real_projects.rs` con 5+ proyectos reales
      multi-lenguaje.
- [ ] M27 — Sandbox podman + quadlets + benchmarks.
- [ ] Investigar si `call_graph`, `class_diagram`, `state_machine` tienen
      los mismos bugs B2/B3/B4/B5 que c4_discover.

## Referencias

- ADR-024 — Element.category semantics (predecesor conceptual)
- ADR-007 — Diagramas como proyecciones
- ADR-013 — Viewer ortogonal
- ADR-026–029 — Diagram authoring toolchain
- PR #44 — fix(m26): C4 contract integrity
- ROADMAP M17 — archview (bloqueado por M26, ahora desbloqueado)