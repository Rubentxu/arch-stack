# Roadmap — OpenCode Architecture Diagrammer

**Estado:** propuesta revisada  
**Versión:** 2.1  
**Fecha:** 29 de julio de 2026

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

---

# M0 — Validación de OpenCode

## Objetivo

Validar agentes, skills, permisos y salidas externas.

## Trabajo

- Perfil mediante `OPENCODE_CONFIG_DIR`.
- Agente `diagram-architect`.
- Subagentes:
  - `architecture-evidence`;
  - `c4-modeler`;
  - `uml-modeler`;
  - `diagram-reviewer`.
- Command `/diagram`.
- Permisos de lectura del repositorio.
- Escritura denegada en el repositorio.
- Acceso autorizado al directorio XDG.
- Skill C4 y skill PlantUML.
- Render local mínimo.

## Salida

- `/diagram c4 context` delega correctamente.
- `/diagram sequence` delega a UML.
- No cambia `git status`.
- Las sesiones hijas son navegables.

---

# M1 — Skillset reproducible

## Objetivo

Reutilizar skills upstream sin forks permanentes.

## Trabajo

- `skills.lock.yaml`.
- Integrar:
  - `c4-codebase-architecture-skill`;
  - `c4-skill`;
  - `c4-model-skill`;
  - `plantuml-skill`;
  - Mermaid;
  - draw.io opcional.
- Wrappers:
  - `architecture-discovery`;
  - `c4-from-graph`;
  - `use-cases-from-graph`;
  - `class-view-from-graph`;
  - `sequence-from-scenario`;
  - `diagram-review`.
- Pruebas de carga, permisos y actualización.

## Salida

- Upstream intacto.
- Versiones fijadas.
- Cada agente ve solo sus skills.
- Actualizaciones evaluables antes de activarse.

---

# M2 — `archctl`, XDG y LadybugDB

## Objetivo

Crear el sidecar persistente mínimo.

## Trabajo

### Proyecto

- Workspace Rust.
- Resolución XDG.
- Identidad de repositorio y worktree.
- Bloqueo por proyecto.
- JSON estable de entrada/salida.

### LadybugDB

- crate `lbug`.
- `architecture.lbdb`.
- migración inicial.
- catálogo de metatipos y predicados.
- exportación e importación.
- comprobación de versión y esquema.
- estrategia de backup.

### Núcleo de datos

- `MetaType`.
- `Predicate`.
- `Element` y `ElementVersion`.
- `SemanticRelation` y `RelationVersion`.
- `Snapshot`.
- `Evidence`.
- `Artifact`.
- `AnalysisRun`.
- `SEMANTIC_EDGE`.

### OpenCode

Custom tools:

```text
arch_project
arch_run
arch_graph
arch_snapshot
arch_artifact
```

## Salida

- La base se crea fuera del repo.
- Dos worktrees tienen overlays distintos.
- Se persiste un elemento y una relación con evidencia.
- El índice materializado se reconstruye.
- La base puede exportarse e importarse.

---

# M3 — Evidencias y adaptadores básicos

## Objetivo

Poblar el grafo con información útil.

## Herramientas

- Git.
- ripgrep.
- ast-grep.
- metadatos del build.

## Trabajo

- inventario de repositorio;
- adaptador `ast-grep`;
- reglas iniciales por framework;
- manifests y entrypoints;
- paquetes y módulos;
- evidencias por fichero, líneas, hash y commit;
- `ToolRun`;
- importación incremental y masiva;
- invalidación por fichero.

## Salida

- Nodos y relaciones observados.
- Evidencias consultables.
- Cambiar un fichero invalida solo su conocimiento derivado.
- No se almacena el AST completo.

---

# M4 — Vertical C4

## Objetivo

Crear Context, Container y Component desde el grafo.

## Trabajo

- tipos y predicados C4;
- reglas de jerarquía;
- wrapper `c4-from-graph`;
- especificaciones de vista;
- Structurizr DSL;
- render local;
- revisión de niveles;
- artefactos y manifiestos.

## Aceptación

```text
/diagram c4 container
```

Produce:

- contexto suficiente;
- Container;
- `workspace.dsl`;
- render;
- evidencias;
- inferencias explícitas;
- diagrama persistido.

## Salida

- Mismos IDs en todas las vistas.
- No aparecen clases en Container.
- Regenerar sin cambios reutiliza el grafo.

---

# M5 — Casos de uso y escenarios

## Objetivo

Representar objetivos y escenarios funcionales.

## Trabajo

- actores;
- casos de uso;
- `include`, `extend` y participación;
- escenarios principal y alternativos;
- evidencias desde tests, contratos y documentación;
- candidatos inferidos;
- confirmación humana;
- PlantUML Use Case.

## Salida

- `/diagram use-cases checkout`.
- Caso de uso enlazado con escenarios.
- Un endpoint aislado no se confirma como caso de uso.
- Actores y sistema objetivo identificables.

---

# M6 — Secuencias y C4 Dynamic

## Objetivo

Generar secuencias multinivel desde escenarios.

## Trabajo

- participantes;
- interacciones ordenadas;
- llamadas síncronas y asíncronas;
- eventos;
- returns;
- `alt`, `opt`, `loop` y `par`;
- rutas de llamada;
- proyección:
  - operación;
  - clase;
  - componente;
  - contenedor;
  - sistema.
- PlantUML Sequence.
- Structurizr Dynamic.

## Salida

- La secuencia muestra mensajes significativos.
- Cada interacción enlaza con evidencia.
- El mismo escenario produce una vista UML y una C4 Dynamic.
- El usuario puede expandir o colapsar nivel.

---

# M7 — Diagramas de clases

## Objetivo

Generar vistas estructurales acotadas.

## Trabajo

- clases, interfaces, traits y enums;
- operaciones y atributos;
- herencia e implementación;
- asociaciones, agregación y composición;
- multiplicidad y roles;
- enlace clase → componente;
- filtros por agregado, módulo o colaboración;
- PlantUML Class.

## Salida

- `/diagram class order-domain`.
- No se genera un volcado completo.
- Las relaciones importantes tienen evidencia.
- Las clases enlazan con componentes C4.

---

# M8 — Vista, revisión y formatos

## Objetivo

Persistir diagramas como vistas y mejorar su calidad.

## Trabajo

- `view.diagram`, `view.member`, `view.edge`;
- especificación de vista;
- materialización;
- revisión sintáctica y semántica;
- densidad, etiquetas y niveles;
- estados:
  - draft;
  - reviewed;
  - accepted;
  - stale.
- Mermaid.
- draw.io.
- SVG, PNG y PDF.

## Salida

- Un diagrama no aceptado conserva sus fallos.
- draw.io es derivado editable.
- El grafo no cambia por un retoque visual.
- Los artefactos conservan hash y renderer.

---

# M9 — Versionado, recuperación y actualización

## Objetivo

Mantener el conocimiento a lo largo del tiempo.

## Trabajo

- `ElementVersion` y `RelationVersion`;
- snapshots;
- overlays de worktree;
- diff de snapshots;
- checkpoints;
- `run resume`;
- diagramas `stale`;
- actualización incremental;
- exportación e importación;
- migraciones de LadybugDB y del metamodelo;
- backup antes de migrar.

## Salida

- Una sesión nueva recupera el estado.
- Un cambio localizado no regenera todo.
- Se puede explicar qué cambió entre dos commits.
- Es reproducible qué skills y tools generaron un artefacto.

---

# M10 — Herramientas semánticas opcionales

## Objetivo

Mejorar precisión sin hacerlas obligatorias.

## Adaptadores

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

## Salida

- Router de capacidades.
- Fallbacks explícitos.
- Confianza según profundidad.
- Ninguna herramienta opcional bloquea el MVP.

---

# M11 — Endurecimiento 1.0

## Trabajo

- instalador;
- `archctl doctor`;
- fixtures Rust, Java, TypeScript y mixtos;
- pruebas de migración;
- pruebas de bloqueo;
- limpieza y retención;
- redacción de secretos;
- Podman;
- documentación;
- SemVer.

## Métricas

- 0 elementos inventados en fixtures controlados.
- ≥ 90 % de cobertura de evidencias principales.
- 100 % de renders sintácticamente válidos.
- 0 ficheros creados en el repositorio.
- recuperación tras interrupción.
- exportación/importación verificadas.
- reconstrucción correcta de `SEMANTIC_EDGE`.

---

## Primer MVP útil

```text
M0 → M1 → M2 → M3 → M4 → M5 → M6
```

Incluye:

- perfil OpenCode;
- skills reutilizadas;
- LadybugDB;
- evidencias;
- C4 Context/Container/Component;
- casos de uso;
- secuencias y C4 Dynamic;
- persistencia externa y recuperación básica.
