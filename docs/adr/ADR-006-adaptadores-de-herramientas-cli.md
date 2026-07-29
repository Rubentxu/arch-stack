# ADR-006 — Adaptadores de herramientas CLI existentes

**Estado:** Aceptado con excepción parcial (ver Status update)
**Fecha:** 29 de julio de 2026
**Status update:** 29 de julio de 2026 — ast-grep movido a librería

## Contexto

Crear parsers, indexadores, resolvedores de símbolos y call graphs propios desviaría el proyecto.

## Decisión

`archctl` implementará adaptadores y normalizadores, no analizadores.

### Núcleo

- Git.
- ripgrep.
- `ast-grep`.
- herramientas nativas del build.
- Structurizr CLI.
- PlantUML.
- Mermaid CLI cuando corresponda.

### Opcionales

- LSP.
- SCIP.
- Universal Ctags.
- dependency-cruiser.
- `jdeps`.
- Semgrep.
- Joern.
- Terraform.
- Helm.
- kubectl.
- Syft.

## Capabilities

Los agentes solicitan:

```text
inventory.repository
syntax.patterns
symbols.list
references.find
dependencies.module
calls.path
infrastructure.topology
diagram.render
diagram.validate
```

El router selecciona el adaptador disponible.

## Salida normalizada

Todo adaptador produce:

- elementos candidatos;
- relaciones candidatas;
- evidencias;
- herramienta y versión;
- confianza;
- snapshot;
- diagnósticos.

`archctl` transforma el resultado y lo importa en LadybugDB.

## Perfiles

### `fast`

Git, ripgrep, ast-grep y build metadata.

### `semantic`

LSP, SCIP y herramientas del lenguaje.

### `deep`

Joern, Semgrep avanzado u otras herramientas profundas.

## Consecuencias

- Incorporación progresiva de lenguajes.
- Sustitución sencilla de tools.
- Importación masiva mediante ficheros temporales.
- Confianza distinta según la fuente.

---

## Status update — 29 de julio de 2026

**Excepción parcial**: para `ast-grep` ya no aplica "adaptador de CLI". A partir de M4 (commit `ea47114`) y consolidado en M7 (ver [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md)), archctl integra `ast-grep-core` como librería in-process. Las razones:

- M4 ya ejecuta `ast_grep_core::Pattern::try_new(...)` directamente; no se invoca `ast-grep scan` como subproceso.
- Elimina fork+exec del path de `archctl evidence extract`.
- Permite streaming de matches sobre repos grandes sin overhead de serialización JSONL.
- `ast-grep-language 0.45` (M7) reemplaza el boilerplate `impl Language for Lang` por gramática con un catálogo pre-cableado.

**Lo que sigue aplicando** (todas las demás herramientas del Núcleo y Opcionales):

- Git → `gix` (M5), fork+exec eliminado, sigue siendo librería.
- ripgrep → se mantiene como CLI hasta que aparezca demanda real de reemplazarlo.
- Herramientas nativas del build (Cargo, npm, go, javac, scalac) → CLIs ejecutados, nunca reimplementados.
- Structurizr CLI / Lite → CLIs externos.
- PlantUML → `plantuml.jar` local.
- Mermaid CLI → opcional.
- SCIP, LSP, Universal Ctags, dependency-cruiser, `jdeps`, Semgrep, Joern, Terraform, Helm, kubectl, Syft → CLIs cuando se introduzcan en M14.

**Política resultante**: por defecto adaptamos CLIs; cuando una librería Rust exista, sea mantenida activamente y aporte valor claro (latencia, parsing incremental, semántica), la integramos como librería con ADR dedicado que documente el cambio. Esta política queda codificada en ADR-012.
