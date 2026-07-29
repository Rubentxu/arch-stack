# ADR-006 — Adaptadores de herramientas CLI existentes

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

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
