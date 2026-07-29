# ADR-006 — Política de integración de herramientas: preferir librerías, descartar CLIs

**Estado:** Sustituido por ADR-012 (ver Status update)
**Fecha:** 29 de julio de 2026
**Status update:** 29 de julio de 2026 — Política revisada

## Contexto

Crear parsers, indexadores, resolvedores de símbolos y call graphs propios desviaría el proyecto. La propuesta original era "adaptar CLIs externos", pero archctl no necesita reinventar capacidades que ya existen como librerías Rust mantenidas. Adaptar un CLI incurre en fork+exec, parsing de salida textual, y una capa de normalización que la librería ya hace internamente.

## Decisión original (29 de julio de 2026)

`archctl` implementará adaptadores y normalizadores, no analizadores.

### Núcleo (original)

- Git.
- ripgrep.
- `ast-grep`.
- herramientas nativas del build.
- Structurizr CLI.
- PlantUML.
- Mermaid CLI cuando corresponda.

### Opcionales (original)

- LSP, SCIP, Universal Ctags, dependency-cruiser, `jdeps`, Semgrep, Joern, Terraform, Helm, kubectl, Syft.

### Capabilities

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

### Salida normalizada

Todo adaptador produce:

- elementos candidatos;
- relaciones candidatas;
- evidencias;
- herramienta y versión;
- confianza;
- snapshot;
- diagnósticos.

`archctl` transforma el resultado y lo importa en LadybugDB.

### Perfiles (originales)

- `fast`: Git, ripgrep, ast-grep, build metadata.
- `semantic`: LSP, SCIP, herramientas del lenguaje.
- `deep`: Joern, Semgrep avanzado u otras herramientas profundas.

## Sustitución parcial (29 de julio de 2026)

ADR-012 introduce una **política revisada y más estricta**: archctl descarta CLIs externos cuando existe una librería Rust mantenida activamente que cumple la misma función. Los CLIs se invocan solo cuando no hay alternativa razonable en Rust.

### Tabla de sustituciones

| CLI original | Librería sustituta | Estado |
|---|---|---|
| `git <subcmd>` | `gix 0.86` | M5, ya planeado |
| `ast-grep scan` | `ast-grep-core 0.45` + `ast-grep-language 0.45` | M4 + M7, ya integrado |
| `cargo metadata` (parseo manual de TOML) | `cargo_metadata 0.23` | M6, ya planeado |
| `ripgrep` (futuro) | `ignore 0.4` + `grep-regex` | ya parcialmente en uso; sin commit dedicado |
| `tree-sitter` invocación ad-hoc | `tree-sitter-graph 0.12` (M8) | M8, ya planeado |
| `ctags` (futuro) | `ctrs` u otro port Rust | sin plazo |
| `scip` indexador externo (futuro) | `scip` crate | M14, ya planeado |
| `semgrep` / `joern` (futuro) | sin alternativa Rust mantenida | mantener CLI si se introduce |
| `dependency-cruiser` / `jdeps` (futuro) | sin alternativa Rust razonable | mantener CLI si se introduce |

### CLIs que se mantienen explícitamente

No son "adaptadores" en el sentido de este ADR. Son **fronteras del sistema** donde no hay alternativa en Rust mantenida o el coste de embedding supera el beneficio:

- **Renderers** (PlantUML vía `plantuml.jar`, Structurizr CLI / Lite, Mermaid CLI). Ninguno tiene renderer equivalente en Rust razonablemente mantenido; no podemos "descartarlos" sin reescribir renderers. ADR-011 refuerza además el bloqueo de servicios públicos (plantuml.com, kroki.io).
- **Build metadata no-Cargo** (`npm ls`, `go list`, `javac -XprintRounds`). No hay alternativa unificada en Rust; cada herramienta tiene su propio protocolo.
- **Infraestructura como código** (Terraform, Helm, kubectl, Syft). Ninguno tiene parser Rust mantenido equivalente a la herramienta oficial.
- **Análisis profundo opcional** (Semgrep, Joern) si se introducen en M14: mantener CLI hasta que aparezca port maduro.

### Política operativa

1. Antes de añadir una herramienta al Núcleo o a Opcionales, **evaluar primero si existe una librería Rust mantenida**. Si existe, preferir la librería. Documentar la sustitución en un ADR dedicado.
2. Si no existe librería, adaptar el CLI con un envoltorio mínimo que produzca la salida normalizada del contrato de este ADR.
3. Si en el futuro aparece una librería para una herramienta que era CLI, abrir un ADR de sustitución siguiendo el patrón de ADR-012.
4. Renderers y herramientas sin alternativa Rust siguen siendo CLI. El ADR-011 (renderers locales y bloqueo de públicos) refuerza el aislamiento de los renderers.

### Perfiles revisados

- `fast`: librerías in-process (gix + ast-grep-core + tree-sitter-graph + cargo_metadata). Cero fork+exec.
- `semantic`: SCIP + (futuro) tree-sitter-stack-graphs.
- `deep`: cualquier análisis profundo que en el futuro tenga port Rust; hasta entonces, CLI con opt-in explícito.

## Consecuencias

### Positivas (de la decisión original)

- Incorporación progresiva de lenguajes.
- Sustitución sencilla de tools.
- Importación masiva mediante ficheros temporales.
- Confianza distinta según la fuente.

### Positivas (de la política revisada)

- Cero fork+exec en el path crítico de los agentes.
- Latencia < 5 ms en operaciones de identidad y pattern matching.
- Streaming de matches sin round-trip por JSONL.
- Contratos tipados Rust entre módulos (no strings parseados).

### Negativas y riesgos (de la política revisada)

- Mayor superficie de dependencias y de versiones a trackear.
- Las librerías Rust a veces quedan atrás de las herramientas oficiales en cobertura (p. ej. tree-sitter-stack-graphs necesita reglas por framework, mientras SCIP las trae el indexador externo).
- El policy "1 crate por commit" introduce churn en el lockfile que requiere disciplina de revisión.

## Cómo revertir

Cada librería adoptada documenta su reversión en [ADR-012 § Cómo revertir](ADR-012-adopcion-incremental-crates-analisis.md). La política revisada misma se revierte cambiando este ADR de vuelta a "adaptadores CLI", pero no debería hacerse salvo que aparezca un caso donde una librería sustituta tenga bugs graves y el CLI sea claramente superior.
