# Roadmap — OpenCode Architecture Diagrammer

**Estado:** propuesta revisada
**Versión:** 2.2
**Fecha:** 29 de julio de 2026
**Cambios vs 2.1:** ciclo M5–M8 introducido tras [ADR-012](adr/ADR-012-adopcion-incremental-crates-analisis.md); M9–M12 desplazados desde los antiguos M5–M8; M0–M4 marcados como completados contra la implementación real.

---

## Principios

1. OpenCode, agentes y skills son el producto.
2. `archctl` es una CLI auxiliar.
3. LadybugDB entra pronto porque C4 y UML deben compartir identidades.
4. Se entregan verticales completas.
5. Cero escritura dentro del repositorio.
6. Se reutilizan herramientas existentes.
7. No se añade un daemon hasta que la concurrencia lo justifique.
8. Cada diagrama tiene propósito, alcance y evidencia.
9. **Adoptamos crates de análisis como librerías, no como CLIs.** ADR-012.

---

# M0 — Validación de OpenCode ✅

## Objetivo

Validar agentes, skills, permisos y salidas externas.

## Trabajo

- Perfil mediante `OPENCODE_CONFIG_DIR`.
- Agente `diagram-architect`.
- Subagentes: `architecture-evidence`, `c4-modeler`, `uml-modeler`, `diagram-reviewer`.
- Command `/diagram`.
- Permisos de lectura del repositorio, escritura denegada, acceso autorizado al directorio XDG.
- Skill C4 y skill PlantUML.
- Render local mínimo.

## Salida (lograda)

- `/diagram c4 context` delega correctamente.
- `/diagram sequence` delega a UML.
- No cambia `git status`.
- Las sesiones hijas son navegables.

## Estado de implementación

`profile/opencode.jsonc`, 5 agentes en `profile/agents/`, command en
`profile/commands/diagram.md`, plugin `archctl-env.ts`. Commit `0ea2065`.

---

# M1 — Skillset reproducible ✅

## Objetivo

Reutilizar skills upstream sin forks permanentes.

## Trabajo

- `skills.lock.yaml`.
- Integrar: `c4-codebase-architecture-skill`, `c4-skill`,
  `c4-model-skill`, `plantuml-skill`, Mermaid y draw.io (estos dos
  últimos como opcionales con `<TBD>` pendiente).
- Wrappers: `architecture-discovery`, `c4-from-graph`,
  `use-cases-from-graph`, `class-view-from-graph`,
  `sequence-from-scenario`, `diagram-review`.
- Pruebas de carga, permisos y actualización.

## Salida (lograda)

- Upstream intacto.
- Versiones fijadas con `<pin at first sync>`.
- Cada agente ve solo sus skills via `permission.skill` per-agent.
- Actualizaciones evaluables antes de activarse.

## Estado de implementación

`archctl/config/skills.lock.yaml` + 6 wrappers en
`profile/skills/<name>/SKILL.md`. Commits `c701fdc`, `22c57e5`.

---

# M2 — `archctl`, XDG y LadybugDB ✅

## Objetivo

Crear el sidecar persistente mínimo.

## Trabajo

### Proyecto

- Workspace Rust (edition 2024).
- Resolución XDG cross-OS (HOME / USERPROFILE / HOMEDRIVE+HOMEPATH).
- Identidad de repositorio y worktree (SHA-256 + UUIDv4).
- JSON estable de entrada/salida.

### LadybugDB

- crate `lbug 0.18.3`.
- `architecture.lbdb` bajo `${XDG_DATA_HOME}/archctl/projects/<uuid>/`.
- migración inicial desde `docs/schema/001_initial_schema.cypher`.
- Strip de `CREATE GRAPH` / `USE` (lbug single-graph mode).
- Validación allowlist de ids contra Cypher injection.

### Núcleo de datos

- `MetaType`, `Predicate`, `Element`, `ElementVersion`,
  `SemanticRelation`, `RelationVersion`, `Snapshot`, `Evidence`,
  `SourceArtifact`, `ToolRun`, `Artifact`, `AnalysisRun`.
- `OF_TYPE`, `RELATION_TYPE`, `VERSION_OF`, `CURRENT_VERSION`,
  `REL_SOURCE`, `REL_TARGET`, `SEMANTIC_EDGE`, `SUPPORTED_BY`, etc.

### CLI

`archctl doctor | project resolve | graph {init,stat,query,neighbours}
| render | skills {list,sync,verify,activate}`.

## Salida (lograda)

- La base se crea fuera del repo (`${XDG_DATA_HOME}/archctl/projects/`).
- Dos worktrees tienen project_ids distintos.
- Schema bootstrap idempotente vía `.archctl-schema` marker.

## Estado de implementación

`archctl/src/{cli,telemetry,xdg,identity,project,doctor,graph}/`.
Commits `4c0471c`, `f63f616`.

---

# M3 — Evidencias y adaptadores básicos ✅ (unido con M4)

## Objetivo

Poblar el grafo con información útil.

## Herramientas

- `ast-grep-core 0.45` + 6 gramáticas tree-sitter (rust, typescript,
  javascript, python, go, java) — no CLI, librería in-process.
- `ignore 0.4` para tree walking respetando `.gitignore` (con
  `require_git(false)` para proyectos no-git).

## Trabajo

- Inventario de repositorio (tree + language histogram).
- `archctl inventory {tree, languages}`.
- `archctl inventory supported_files` filtra a lenguajes parseables.
- `archctl evidence extract --lang L --pattern P --claim C --put`
  produce `Evidence` rows MERGE en el grafo.
- Ids de evidencia deterministas (blake3 de `path + start_byte +
  end_byte + text`).

## Salida (lograda)

- 96 matches `fn $NAME` extraídos del propio `archctl/` y persistidos.
- Idempotencia verificada (re-running --put no duplica rows).
- 33 tests verdes (12 astgrep + 5 inventory + 7 evidence + 9 graph/identity).

## Estado de implementación

`archctl/src/{astgrep,inventory,evidence}.rs`. Commit `ea47114`.

---

# M4 — Vertical C4 (extractores, no render)

## Objetivo

Crear Context, Container y Component desde el grafo. Por el momento
sólo los extractores; el render Structurizr sigue siendo
`archctl render` directo.

## Trabajo

- Tipos y predicados C4 (`c4:system:*`, `c4:container:*`,
  `c4:component:*`) sembrados en el `metamodel-core.json`.
- Reglas de jerarquía (`container belongs_to system`, `component
  belongs_to container`).
- Rule pack tree-sitter-graph (M8) para mapear clases anotadas
  (`@Component`, `@RestController`, etc.) a `c4.component`.
- Especificaciones de vista persistidas.
- Render local Structurizr (CLI o Lite) vía `archctl render`.

## Aceptación

`archctl diagram c4 container --from-graph` produce:

- contexto suficiente;
- Container;
- `workspace.dsl`;
- render;
- evidencias;
- inferencias explícitas;
- diagrama persistido.

## Salida (objetivo)

- Mismos IDs en todas las vistas.
- No aparecen clases en Container.
- Regenerar sin cambios reutiliza el grafo.

---

# M5 — `gix` para identidad de repositorio

## Objetivo

Eliminar fork+exec del CLI Git en el path crítico de
`archctl project resolve` (operación más frecuente: cada llamada de
un agente).

## Trabajo

- Reemplazar `Command::new("git")` en `identity.rs` por
  `gix::discover(cwd)` + `repo.workdir()` + `repo.head_commit()` +
  `repo.find_remote("origin")`.
- Mantener fallback a CLI si `gix` no puede manejar un caso
  específico (repos con submódulos complejos, worktrees linkeados).
- API pública sin cambios (`SourceIdentity`, `resolve_source_identity`).

## Salida

- `archctl project resolve` no invoca procesos externos.
- Latencia < 5 ms en repos típicos (vs 15–30 ms con fork+exec).

## Estado

Pendiente.

---

# M6 — `cargo_metadata` para `inventory depends`

## Objetivo

Resucitar `archctl inventory depends` (eliminado en M3) con un JSON
estable producido por Cargo, en lugar de parsear `Cargo.toml` a mano.

## Trabajo

- Añadir `cargo_metadata 0.23` como dep.
- `archctl inventory depends` ejecuta `cargo_metadata::MetadataCommand`
  y devuelve `packages` + `resolve.nodes` + `workspace_members`.
- Idempotente y libre de errores de parseo propios.

## Salida

- Inventario de dependencias nativo de Cargo, no reinventado.
- `Cargo.lock` resuelto incluido.

## Estado

Pendiente.

---

# M7 — `ast-grep-language` y Kotlin

## Objetivo

Reducir el boilerplate `impl Language for Lang` por gramática y
habilitar **kotlin** sin coste adicional (su `builtin-parser` ya lo
trae).

## Trabajo

- Evaluar si `ast-grep-language::SupportLang` cubre nuestros 6
  lenguajes actuales; si sí, sustituir el dispatch manual de
  `astgrep.rs` por un dispatch sobre `SupportLang`.
- Decidir entre `default-features` (25+ gramáticas, +binario) vs
  features custom (solo las nuestras).
- Habilitar Kotlin: añadir entrada en `Lang::ALL`, test de smoke.

## Salida

- Menos código de boilerplate en `astgrep.rs`.
- Kotlin parseable por ast-grep.
- Binario ≤ 80 MB (verificación de la métrica de ADR-012).

## Estado

Pendiente.

---

# M8 — `tree-sitter-graph` para extractores declarativos

## Objetivo

Sustituir la extracción ad-hoc de `evidence.rs` (matches sueltos
producidos por ast-grep) por un DSL declarativo que produzca
`ElementCandidate` + `RelationCandidate` directos.

## Trabajo

- Añadir `tree-sitter-graph 0.12` como dep.
- Módulo nuevo `archctl/src/extractors/` con:
  - `RulePack` (carga un fichero `.tsg`).
  - `Rule` (pattern + emit).
  - `Extractor` (corre el rule pack sobre un AST).
- Rule pack mínimo por lenguaje: Rust (structs, traits, impls),
  Java (`@Component`, `@RestController`), Python (funciones,
  clases).
- Integración con `evidence::extract` para que las aristas producidas
  por el rule pack se conviertan en `SemanticRelation` con
  evidencia.

## Salida

- Un extractor declarativo reemplaza 50+ líneas de código imperativo
  en `evidence.rs`.
- Los rule packs son archivos `.tsg` versionados independientemente
  del binario.

## Estado

Pendiente.

---

# M9 — Casos de uso y escenarios

## Objetivo

Representar objetivos y escenarios funcionales.

## Trabajo

- Actores y casos de uso.
- `include`, `extend` y participación.
- Escenarios principal y alternativos.
- Evidencias desde tests, contratos y documentación.
- Candidatos inferidos con confirmación humana.
- PlantUML Use Case.

## Salida

- `/diagram use-cases checkout`.
- Caso de uso enlazado con escenarios.
- Un endpoint aislado no se confirma como caso de uso.
- Actores y sistema objetivo identificables.

---

# M10 — Secuencias y C4 Dynamic

## Objetivo

Generar secuencias multinivel desde escenarios.

## Trabajo

- Participantes e interacciones ordenadas.
- Llamadas síncronas y asíncronas, eventos, returns.
- `alt`, `opt`, `loop` y `par`.
- Rutas de llamada.
- Proyección: operación, clase, componente, contenedor, sistema.
- PlantUML Sequence y Structurizr Dynamic.

## Salida

- La secuencia muestra mensajes significativos.
- Cada interacción enlaza con evidencia.
- El mismo escenario produce vista UML y C4 Dynamic.
- El usuario puede expandir o colapsar nivel.

---

# M11 — Diagramas de clases

## Objetivo

Generar vistas estructurales acotadas.

## Trabajo

- Clases, interfaces, traits y enums.
- Operaciones y atributos.
- Herencia e implementación.
- Asociaciones, agregación y composición.
- Multiplicidad y roles.
- Enlace clase → componente.
- Filtros por agregado, módulo o colaboración.
- PlantUML Class.

## Salida

- `/diagram class order-domain`.
- No se genera un volcado completo.
- Las relaciones importantes tienen evidencia.
- Las clases enlazan con componentes C4.

---

# M12 — Vista, revisión y formatos

## Objetivo

Persistir diagramas como vistas y mejorar su calidad.

## Trabajo

- `view.diagram`, `view.member`, `view.edge`.
- Especificación de vista.
- Materialización.
- Revisión sintáctica y semántica.
- Densidad, etiquetas y niveles.
- Estados: `draft`, `reviewed`, `accepted`, `stale`.
- Mermaid.
- draw.io.
- SVG, PNG y PDF.

## Salida

- Un diagrama no aceptado conserva sus fallos.
- draw.io es derivado editable.
- El grafo no cambia por un retoque visual.
- Los artefactos conservan hash y renderer.

---

# M13 — Versionado, recuperación y actualización (era M9)

## Objetivo

Mantener el conocimiento a lo largo del tiempo.

## Trabajo

- `ElementVersion` y `RelationVersion`.
- Snapshots.
- Overlays de worktree.
- Diff de snapshots.
- Checkpoints.
- `run resume`.
- Diagramas `stale`.
- Actualización incremental.
- Exportación e importación.
- Migraciones de LadybugDB y del metamodelo.
- Backup antes de migrar.

## Salida

- Una sesión nueva recupera el estado.
- Un cambio localizado no regenera todo.
- Se puede explicar qué cambió entre dos commits.
- Es reproducible qué skills y tools generaron un artefacto.

---

# M14 — Herramientas semánticas opcionales (era M10)

## Objetivo

Mejorar precisión sin hacerlas obligatorias.

## Adaptadores

- LSP (con `lsp-types`).
- SCIP.
- `oxc_parser` + `oxc_semantic` para JS/TS.
- `ra_ap_*` para Rust (encapsulado en `archctl-analysis-rust`).
- Universal Ctags.
- dependency-cruiser.
- `jdeps`.
- Semgrep.
- Joern.
- Terraform, Helm, kubectl, Syft.

## Salida

- Router de capacidades.
- Fallbacks explícitos.
- Confianza según profundidad.
- Ninguna herramienta opcional bloquea el MVP.

---

# M15 — Endurecimiento 1.0 (era M11)

## Trabajo

- Instalador.
- `archctl doctor` extendido (reporta crates integrado).
- Fixtures Rust, Java, TypeScript, Python y mixtos.
- Pruebas de migración.
- Pruebas de bloqueo.
- Limpieza y retención.
- Redacción de secretos.
- Podman.
- Documentación.
- SemVer.

## Métricas

- 0 elementos inventados en fixtures controlados.
- ≥ 90 % de cobertura de evidencias principales.
- 100 % de renders sintácticamente válidos.
- 0 ficheros creados en el repositorio.
- Recuperación tras interrupción.
- Exportación/importación verificadas.
- Reconstrucción correcta de `SEMANTIC_EDGE`.
- Binario ≤ 80 MB.

---

## Primer MVP útil

```text
M0 → M1 → M2 → M3 → M4 → M5 → M6 → M7 → M8
```

Incluye:

- Perfil OpenCode.
- Skills reutilizadas con `skills.lock.yaml`.
- `archctl` como sidecar Rust con `gix` para identidad.
- `cargo_metadata` para dependencias nativas.
- `ast-grep-language` con 7 lenguajes (kotlin incluido).
- LadybugDB con grafo de evidencias.
- `tree-sitter-graph` para extractores declarativos versionables.
- 0 fork+exec en el path crítico de los agentes.
- Salida: diagramas C4 + UML como proyecciones del grafo.
