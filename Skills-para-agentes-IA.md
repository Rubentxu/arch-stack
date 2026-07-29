> From: https://chatgpt.com/c/6a69ab29-0814-83ed-a876-f279e71f9e11

# you asked

message time: 2026-07-29 09:28:17

ok quiero que busques skills para agentes de IA expertos en diagramacion en modelo C4 y UML que use tools como mermaid, plantuml o cualquier otro util para su visualizacion de datos, que puedan capturar con ingenieria inversa y agentes LLM todo lo necesario para crear estos diagramas con detalle y efectivos

---

# chatgpt response

Voy a buscar skills y repositorios actuales orientados a C4/UML, generación con Mermaid/PlantUML y extracción automática desde código. Después los separaré entre “skill lista para usar”, MCP/tool integrable y piezas para construir un agente realmente fiable.

Ya aparecen varias opciones útiles, pero hay una diferencia importante: unas solo dibujan y otras sí inspeccionan el repositorio. Las más prometedoras para ingeniería inversa son las que separan evidencia de inferencias y generan un modelo intermedio antes de renderizar.

He encontrado un ecosistema bastante más maduro de lo que esperaba. **No existe todavía una única skill que haga con rigor ingeniería inversa, modelado C4/UML, renderizado, validación visual y actualización continua**, pero sí podemos componer un conjunto muy potente.

## Mi selección principal

| Proyecto | Especialidad | Ingeniería inversa | Salidas | Valoración |
|---|---|---:|---|---|
| **lmammino/c4-codebase-architecture-skill** | Recuperar arquitectura C4 desde repositorios | **Alta** | Mermaid, PlantUML, Structurizr DSL | **Mejor base general** |
| **jonverrier/AgentDoc** | Repositorios legacy y muy grandes | **Alta** | C4 + Mermaid + documentación jerárquica | **Mejor para monolitos grandes** |
| **bitsmuggler/c4-skill** | C4 formal como modelo, no solo dibujos | **Alta** | Structurizr DSL, PlantUML, Mermaid, HTML | **Mejor modelo canónico** |
| **cheriftj/c4-model-skill** | Diseñar, documentar, revisar y actualizar C4 | Media-alta | Mermaid, Structurizr, PlantUML | **Mejor workflow interactivo** |
| **Daves-Claude-Code-Skills** | Calidad visual y revisión arquitectónica | Baja en extracción | Mermaid, PlantUML, Python Diagrams | **Mejor revisor visual** |
| **Agents365 drawio-skill** | Diagramas editables y presentación profesional | Media-alta | draw.io, PNG, SVG, PDF | **Mejor acabado editable** |
| **Agents365 plantuml-skill** | UML formal y C4-PlantUML | Media | PlantUML, PNG, SVG | **Mejor complemento UML** |
| **docToolchain/diag-agent** | Generación y reparación autónoma | Baja por sí solo | PlantUML, C4, BPMN, Mermaid vía Kroki | **Buen motor de validación** |

---

## 1. La mejor skill de partida: `c4-codebase-architecture-skill`

Es la opción que más coincide con lo que buscas. Inspecciona evidencias como entrypoints, manifests, infraestructura y descriptores de despliegue; genera Context, Container y Component; y, algo fundamental, **separa los hechos observados de las inferencias del agente**. También puede producir Mermaid, PlantUML o Structurizr DSL. ([GitHub](https://github.com/lmammino/c4-codebase-architecture-skill/blob/main/README.md))

Instalación:

```bash
npx skills add lmammino/c4-codebase-architecture-skill \
  --skill c4-codebase-architecture
```

La usaría como **agente descubridor y modelador inicial**, pero no como solución completa: todavía necesita mejores extractores deterministas, validación visual y control de deriva.

---

## 2. Para repositorios legacy enormes: `AgentDoc`

AgentDoc, anteriormente llamado C4-Agent, está especialmente bien planteado para repositorios que no caben en la ventana de contexto. Recorre el árbol de directorios de abajo arriba, genera resúmenes locales y después los consolida hasta construir la visión global. Además, comprueba timestamps para evitar regenerar documentación que no ha cambiado. ([GitHub](https://github.com/jonverrier/McpDoc))

Su enfoque es adecuado para:

- Monolitos Java, Groovy o .NET.
- Monorepos con muchos módulos.
- Jenkins Shared Libraries extensas.
- Proyectos antiguos con poca documentación.
- Análisis incremental sin reenviar todo el código al LLM.

Yo reutilizaría de AgentDoc principalmente su estrategia de **resúmenes arquitectónicos jerárquicos e incrementales**.

---

## 3. Para un modelo C4 serio: `bitsmuggler/c4-skill`

Este plugin analiza el código y genera un `workspace.dsl` de Structurizr. Después puede exportar las vistas a PlantUML, Mermaid, HTML, PNG o SVG. Soporta System Landscape, Context, Container, Component, Deployment y Dynamic. ([GitHub](https://github.com/bitsmuggler/c4-skill))

Es importante porque mantiene un **modelo semántico único** del que se derivan múltiples diagramas. Esto es mucho más fiable que pedirle al agente que genere separadamente cinco ficheros Mermaid que después pueden contradecirse.

```bash
git clone https://github.com/bitsmuggler/c4-skill.git ~/.claude/c4-skill

mkdir -p ~/.claude/commands
ln -s ~/.claude/c4-skill/commands/c4.md ~/.claude/commands/c4.md
```

Para tu caso, mi modelo canónico sería:

```text
Repositorio + configuración + infraestructura + runtime
                         ↓
             Modelo arquitectónico intermedio
                         ↓
                Structurizr workspace.dsl
             ↙             ↓              ↘
       C4 web         C4-PlantUML      Mermaid/Markdown
```

Structurizr está diseñado expresamente como “models as code” para C4, admite control de versiones y permite combinar layout automático con ajustes manuales. ([Structurizr](https://docs.structurizr.com/as-code))

---

## 4. Mejor workflow interactivo: `cheriftj/c4-model-skill`

Esta skill distingue cinco situaciones diferentes:

- Diseñar un sistema nuevo.
- Recuperar arquitectura desde código.
- Crear C4 desde documentación en prosa.
- Revisar diagramas existentes.
- Actualizar un modelo anterior.

Tiene comandos especializados como `/c4m:code`, `/c4m:review` y `/c4m:update`, e incorpora puntos explícitos de validación por cada nivel C4. Puede producir Mermaid, Structurizr o PlantUML. ([GitHub](https://github.com/cheriftj/c4-model-skill))

Instalación en Claude Code:

```text
/plugin marketplace add cheriftj/c4-model-skill
/plugin install c4m@c4-model-skill
```

Me gusta especialmente como **orquestador de conversación**, aunque sustituiría parte de sus preguntas al usuario por análisis automático del repositorio.

---

## 5. La mejor skill para revisar calidad: `Daves-Claude-Code-Skills`

Este repositorio contiene tres skills complementarias:

- `/diagram`
- `/c4-diagram`
- `/diagram-review`

No se limita a comprobar que el código Mermaid o PlantUML compile. Inspecciona la imagen renderizada y evalúa cruces de líneas, jerarquía visual, agrupación, dirección del flujo, trazabilidad, mezcla de niveles de abstracción, legibilidad de etiquetas y nodos aislados. Si detecta problemas, vuelve a generar el diagrama. ([GitHub](https://github.com/DavidROliverBA/Daves-Claude-Code-Skills))

Este componente es imprescindible. Una representación puede ser sintácticamente válida y arquitectónicamente correcta, pero visualmente desastrosa.

Lo convertiría en una skill independiente:

```text
diagram-quality-reviewer
```

Que recibiese el modelo, el código fuente del diagrama y el PNG/SVG renderizado, y devolviese:

```yaml
quality:
  syntax_valid: true
  semantic_consistency: 0.94
  edge_crossings: 3
  abstraction_mixing: false
  unreadable_labels: []
  orphan_nodes: []
  visual_score: 87
  decision: accepted
```

---

## 6. Para diagramas editables: `Agents365-ai/drawio-skill`

Esta es una de las opciones más completas visualmente. Puede convertir código Python, JavaScript/TypeScript, Go y Rust, además de Terraform, Kubernetes, Docker Compose y SQL, en diagramas autoorganizados de draw.io. Incluye presets para UML, C4, arquitectura, ERD, BPMN, SysML y topologías de red. También realiza una comprobación visual de la imagen generada. ([GitHub](https://github.com/Agents365-ai/drawio-skill))

Instalación:

```bash
npx skills add Agents365-ai/365-skills -g
```

O manualmente:

```bash
git clone https://github.com/Agents365-ai/drawio-skill.git \
  ~/.claude/skills/drawio-skill
```

No lo emplearía como fuente de verdad. Lo usaría como **adaptador de presentación**:

```text
Structurizr/C4 IR → draw.io editable → revisión humana
```

Esto permitiría que el agente genere el modelo y después un arquitecto pueda recolocar, anotar o personalizar el resultado sin destruir el modelo original.

---

## 7. Para UML: `plantuml-skill`

La skill de Agents365 soporta diagramas de secuencia, componentes, clases, ER, actividad, casos de uso, estados, C4, Gantt y otros. Valida la sintaxis, reintenta errores de renderizado y vuelve a inspeccionar visualmente el PNG. También dispone de Kroki local y `plantuml.jar` para entornos aislados. ([GitHub](https://github.com/Agents365-ai/plantuml-skill))

La complementaría con la skill de SpillwaveSolutions, que añade ejemplos concretos de conversión desde Spring Boot, FastAPI, Python ETL, Node.js y React, además de extracción de bloques PlantUML desde Markdown y validación para CI/CD. ([GitHub](https://github.com/SpillwaveSolutions/plantuml))

Para repositorios corporativos usaría siempre:

```text
PlantUML local
o
Kroki desplegado internamente
```

No enviaría código, nombres de sistemas ni diagramas a un Kroki público. La propia skill distingue entre el backend público y las alternativas locales, y avisa de si el contenido sale de la máquina. ([GitHub](https://github.com/Agents365-ai/plantuml-skill))

---

## 8. MCPs útiles, pero no sustituyen a las skills

### UML-MCP

`antoinebou12/uml-mcp` expone generación mediante PlantUML, Mermaid, D2 y Kroki. Es un buen adaptador para que el agente renderice y valide diagramas, pero no es por sí mismo un recuperador de arquitectura. ([GitHub](https://github.com/antoinebou12/uml-mcp))

### PlantUML MCP Server

`infobip/plantuml-mcp-server` permite generar SVG/PNG, usar includes externos, guardar localmente y restringir las rutas de escritura. Es un renderer bastante limpio para integrar en agentes. ([GitHub](https://github.com/infobip/plantuml-mcp-server))

Ejemplo:

```json
{
  "mcpServers": {
    "plantuml": {
      "command": "npx",
      "args": ["plantuml-mcp-server"],
      "env": {
        "PLANTUML_SERVER_URL": "http://localhost:8080"
      }
    }
  }
}
```

### `diag-agent`

Puede ejecutarse como CLI, biblioteca o servidor MCP. Valida la descripción, genera el diagrama, comprueba la sintaxis mediante Kroki y entra en un ciclo autónomo de corrección. Soporta PlantUML, C4, BPMN y Mermaid. ([GitHub](https://github.com/docToolchain/diag-agent))

Lo usaría como **servicio de renderizado y reparación**, no como agente principal de ingeniería inversa.

---

# Mermaid, PlantUML o Structurizr

## Mi decisión

### Structurizr DSL: fuente canónica C4

Para:

- System Landscape.
- System Context.
- Containers.
- Components.
- Deployment.
- Dynamic views.
- Relaciones reutilizadas entre diferentes vistas.
- Navegación y zoom arquitectónico.

Structurizr conserva el modelo y genera las vistas a partir de él; no se limita a dibujar cajas. ([Structurizr](https://docs.structurizr.com/as-code))

### PlantUML: UML formal y C4 complejo

Para:

- Secuencias.
- Clases.
- Estados.
- Actividades.
- Componentes UML.
- Despliegue UML.
- Diagramas C4 grandes con control direccional.

C4-PlantUML soporta Context, Container, Component, Dynamic, Deployment y secuencias estilizadas, además de relaciones direccionales, tags, temas, sprites y control de layout. ([GitHub](https://github.com/plantuml-stdlib/C4-PlantUML))

### Mermaid: documentación ligera

Para:

- README.
- Pull requests.
- GitHub/GitLab.
- Flujos pequeños.
- Secuencias sencillas.
- Diagramas rápidos.

No elegiría Mermaid C4 como representación principal: su propia documentación sigue calificando la sintaxis C4 como **experimental**, con funcionalidades y layout todavía incompletos. ([Mermaid](https://mermaid.ai/open-source/syntax/c4.html))

### draw.io: edición y comunicación

Para:

- Presentaciones.
- Talleres.
- Revisión por personas no técnicas.
- Diagramas que necesiten retoques manuales.
- Exportación a PDF o SVG.

---

# Arquitectura de skills que recomiendo construir

No crearía un único agente omnipotente. Crearía un **equipo de skills especializadas** que compartiesen un modelo intermedio.

## 1. `architecture-evidence-collector`

Responsabilidades:

- Detectar lenguajes y frameworks.
- Localizar entrypoints.
- Analizar imports y dependencias.
- Leer manifests y ficheros de build.
- Inspeccionar Dockerfiles y Compose.
- Inspeccionar Terraform, Helm y Kubernetes.
- Extraer OpenAPI, AsyncAPI, GraphQL y protobuf.
- Detectar bases de datos, colas, cachés y servicios externos.
- Inspeccionar pipelines Jenkins, GitHub Actions o GitLab CI.
- Recoger trazas OpenTelemetry cuando estén disponibles.

Debe generar evidencias, no diagramas:

```yaml
entities:
  - id: order-service
    kind: container
    technology: Rust
    evidence:
      - path: crates/order-service/Cargo.toml
      - path: deploy/order-service.yaml
    confidence: 0.99

relationships:
  - source: api-gateway
    target: order-service
    protocol: HTTP
    evidence:
      - path: gateway/routes.yaml
      - path: crates/gateway/src/routes.rs
    confidence: 0.96
```

## 2. `architecture-model-builder`

Fusiona las evidencias en un grafo arquitectónico neutral:

```text
Person
SoftwareSystem
Container
Component
CodeElement
DeploymentNode
Database
Queue
Interface
Relationship
Scenario
Evidence
Decision
```

Aquí se decide qué pertenece a cada nivel C4.

## 3. `c4-architecture-modeler`

Produce:

- Landscape.
- Context.
- Container.
- Component cuando aporte valor.
- Deployment.
- Dynamic views.

Debe impedir que aparezcan, por ejemplo, tablas SQL dentro de un diagrama Container o pods individuales dentro del Context.

## 4. `uml-behaviour-modeler`

Especializado en:

- Sequence diagrams desde llamadas y trazas.
- State diagrams desde enums, transiciones y handlers.
- Activity diagrams desde procesos.
- Class diagrams desde AST e información de tipos.
- Component UML desde módulos e interfaces.
- Deployment UML desde IaC.

## 5. `diagram-renderer-router`

Decide automáticamente:

```yaml
routing:
  c4_canonical: structurizr
  c4_complex_static: c4-plantuml
  uml: plantuml
  markdown_preview: mermaid
  editable_presentation: drawio
  cloud_infrastructure: diagrams_or_drawio
```

## 6. `diagram-quality-reviewer`

Combina:

- Validación sintáctica.
- Validación del metamodelo.
- Revisión visual del SVG/PNG.
- Revisión C4.
- Revisión UML.
- Detección de saturación y cruces.
- Comparación contra evidencias.
- Corrección iterativa.

## 7. `architecture-drift-detector`

En cada pull request:

```text
código modificado
       ↓
evidencias afectadas
       ↓
subgrafo arquitectónico afectado
       ↓
diagramas que deben regenerarse
       ↓
diferencia semántica
```

En lugar de mostrar únicamente un diff textual de Mermaid:

```yaml
architecture_changes:
  added:
    - PaymentService -> FraudService
  removed:
    - PaymentService -> LegacyRiskAPI
  technology_changed:
    - payment-db: PostgreSQL 15 -> PostgreSQL 17
  deployment_changed:
    - replicas: 2 -> 5
```

---

# Por qué conviene utilizar varios agentes

La investigación reciente sobre Code2UML utiliza cinco agentes especializados: planificación, análisis, diagramación, corrección y análisis de dependencias, junto con una capa determinista que compacta la representación intermedia para no superar el contexto del LLM. En su evaluación obtuvo alta validez sintáctica y resultados estables al crecer el repositorio. ([arXiv](https://arxiv.org/abs/2605.24453?utm_source=chatgpt.com))

Otro trabajo sobre automatización C4 utiliza agentes con roles separados y combina verificaciones deterministas de estructura y reglas C4 con evaluación semántica mediante LLM. ([arXiv](https://arxiv.org/abs/2510.22787?utm_source=chatgpt.com))

Por tanto, el patrón más sólido es:

```text
Herramientas deterministas
          +
Agentes especializados
          +
Modelo intermedio verificable
          +
Renderizadores intercambiables
          +
Revisión visual
```

No:

```text
“Lee el repositorio y crea un Mermaid”
```

---

# Combinación que instalaría inicialmente

## Núcleo

```bash
npx skills add lmammino/c4-codebase-architecture-skill \
  --skill c4-codebase-architecture

npx skills add Agents365-ai/365-skills -g
```

## C4 interactivo

```text
/plugin marketplace add cheriftj/c4-model-skill
/plugin install c4m@c4-model-skill
```

## Modelo Structurizr

```bash
git clone https://github.com/bitsmuggler/c4-skill.git ~/.claude/c4-skill

mkdir -p ~/.claude/commands
ln -s ~/.claude/c4-skill/commands/c4.md ~/.claude/commands/c4.md
```

## Renderizado local

```bash
podman run -d \
  --name plantuml-server \
  -p 8080:8080 \
  plantuml/plantuml-server:jetty
```

```bash
podman run -d \
  --name structurizr-lite \
  -p 8081:8080 \
  -v "$PWD/architecture:/usr/local/structurizr" \
  structurizr/lite
```

## Mi veredicto

La base óptima sería:

```text
lmammino/c4-codebase-architecture-skill
        + AgentDoc para análisis jerárquico
        + Structurizr como modelo canónico
        + C4-PlantUML para render complejo
        + PlantUML para UML
        + Mermaid para documentación ligera
        + draw.io para edición visual
        + diagram-review para control de calidad
```

Esto sí puede evolucionar hacia un agente de **inteligencia arquitectónica**, capaz no solo de dibujar, sino de justificar cada elemento con evidencias, detectar inferencias dudosas, navegar por niveles C4, reconstruir comportamientos UML y mantener la documentación sincronizada con el código.

---

# you asked

message time: 2026-07-29 09:43:18

ok y si quisiera crear esa arquitectura de skills y un agente orquestador en opencode dedicado con subagentes especializados sobre los puntos que has comentado? ¿como lo haria?,¿como integraria todas las skills y subagentes?, ¿como se persistiria, como se recuperaria, como evolucionaria a lo largo del tiempo?, Investigalo y dame una propuesta util con pensamiento lateral

---

# chatgpt response

Voy a contrastar primero qué soporta OpenCode hoy de forma nativa —agentes, subagentes, skills, MCP, comandos, persistencia y sesiones— y después te propongo una arquitectura operativa, no solo conceptual.

La base nativa ya permite una solución limpia: OpenCode distingue agentes primarios y subagentes, descubre `SKILL.md` bajo demanda y permite limitar por agente tanto skills como herramientas/MCP. La persistencia duradera, sin embargo, no debe descansar solo en el historial de conversación.

## Propuesta principal

No construiría esto como «un prompt grande con muchos subagentes». Lo plantearía como una **plataforma de inteligencia arquitectónica dentro de OpenCode**, con cuatro planos separados:

```text
┌──────────────────────────────────────────────────────────────────┐
│  Plano de control                                                │
│  OpenCode + agente orquestador + máquina de estados + checkpoints│
├──────────────────────────────────────────────────────────────────┤
│  Plano de conocimiento                                           │
│  Evidencias + grafo arquitectónico + decisiones + temporalidad   │
├──────────────────────────────────────────────────────────────────┤
│  Plano de ejecución                                              │
│  Extractores AST/LSP/IaC + herramientas + MCP + renderizadores   │
├──────────────────────────────────────────────────────────────────┤
│  Plano de presentación                                           │
│  C4, UML, Mermaid, PlantUML, Structurizr, draw.io, HTML           │
└──────────────────────────────────────────────────────────────────┘
```

La idea fundamental es:

> **Los diagramas no son la fuente de verdad. Son proyecciones de un modelo arquitectónico persistente, verificable y versionado.**

OpenCode ya tiene las piezas necesarias: agentes primarios, subagentes, sesiones hijas, skills cargadas bajo demanda, permisos por agente, herramientas personalizadas, plugins con eventos y servidor/SDK programable. ([OpenCode](https://opencode.ai/docs/agents/))

---

# 1. Qué papel cumple cada elemento de OpenCode

Conviene no mezclar responsabilidades.

| Elemento | Responsabilidad |
|---|---|
| **Agente** | Rol, razonamiento, modelo y permisos |
| **Subagente** | Ejecutar una tarea especializada y acotada |
| **Skill** | Procedimiento reutilizable, reglas y conocimiento operativo |
| **Custom tool** | Operación determinista local |
| **MCP** | Integración con sistemas externos |
| **Plugin** | Ciclo de vida, persistencia, observabilidad y políticas |
| **Command** | Punto de entrada reproducible para el usuario |
| **AGENTS.md** | Constitución global del sistema |
| **Architecture IR** | Fuente de verdad arquitectónica |

OpenCode descubre skills en `.opencode/skills/`, `.agents/skills/`, `.claude/skills/` y sus equivalentes globales. El agente solo ve inicialmente el nombre y la descripción, y carga el contenido completo mediante la herramienta `skill` cuando lo necesita. También se puede restringir qué skills ve cada agente mediante patrones de permisos. ([OpenCode](https://opencode.ai/docs/skills/))

Eso permite que cada subagente tenga acceso únicamente a su pequeño conjunto de procedimientos.

---

# 2. Arquitectura de agentes

Crearía un único agente primario dedicado:

```text
architecture-orchestrator
```

Y ocho subagentes.

## Agente primario: `architecture-orchestrator`

No debería analizar miles de ficheros ni dibujar directamente. Sus responsabilidades serían:

1. Entender el objetivo.
2. Inspeccionar el estado persistido.
3. Construir un plan de ejecución.
4. Delegar trabajos independientes.
5. Validar contratos de salida.
6. Resolver contradicciones.
7. Aprobar la actualización del modelo.
8. Generar un informe final.

OpenCode permite configurar un agente primario personalizado como agente predeterminado y controlar la profundidad de delegación con `subagent_depth`. Con profundidad `2`, el orquestador puede llamar a especialistas y algunos especialistas pueden delegar una tarea adicional acotada. ([OpenCode](https://opencode.ai/docs/config/))

## Subagentes

### `repo-cartographer`

Hace inventario del repositorio:

- lenguajes;
- módulos;
- manifests;
- entrypoints;
- estructura de directorios;
- artefactos de construcción;
- configuración;
- documentación existente.

Solo lectura.

### `code-structure-extractor`

Especializado en estructura estática:

- símbolos;
- clases;
- traits e interfaces;
- imports;
- dependencias;
- llamadas;
- módulos;
- tipos;
- endpoints.

Utiliza herramientas deterministas; el LLM interpreta excepciones.

### `infrastructure-extractor`

Analiza:

- Terraform;
- Kubernetes;
- Helm;
- Docker/Compose;
- Jenkins;
- GitHub Actions;
- GitLab CI;
- redes;
- almacenamiento;
- despliegues;
- servicios gestionados.

### `runtime-behaviour-extractor`

Obtiene arquitectura dinámica desde:

- trazas OpenTelemetry;
- logs estructurados;
- pruebas de integración;
- contratos OpenAPI/AsyncAPI;
- llamadas HTTP;
- colas;
- eventos;
- consultas;
- perfiles de ejecución.

### `architecture-synthesizer`

Fusiona las evidencias y construye el modelo intermedio. No renderiza.

Debe diferenciar:

```text
HECHO       → observado directamente
INFERENCIA  → deducido con una confianza
HIPÓTESIS   → explicación provisional
DESCONOCIDO → información ausente
CONFLICTO   → evidencias incompatibles
```

### `c4-modeler`

Proyecta el modelo en:

- Landscape;
- Context;
- Container;
- Component;
- Deployment;
- Dynamic.

Controla que no se mezclen niveles de abstracción.

### `uml-modeler`

Produce:

- secuencia;
- actividad;
- estado;
- componentes;
- clases;
- despliegue;
- casos de uso, cuando realmente aporten valor.

### `architecture-auditor`

Es el abogado del diablo:

- intenta refutar el modelo;
- busca componentes inventados;
- busca relaciones sin evidencia;
- detecta omisiones;
- revisa C4;
- revisa UML;
- compara arquitectura declarada y observada.

Este último agente es especialmente importante. No debe mejorar el dibujo: debe **intentar demostrar que el dibujo es incorrecto**.

---

# 3. Skills necesarias

No haría una skill por agente. Separaría las skills por procedimiento reusable.

```text
.opencode/skills/
├── architecture-evidence/
├── architecture-ir/
├── c4-modeling/
├── c4-review/
├── uml-sequence/
├── uml-structural/
├── runtime-architecture/
├── infrastructure-discovery/
├── diagram-rendering/
├── diagram-visual-review/
├── architecture-drift/
├── evidence-confidence/
├── architecture-resume/
└── architecture-learning/
```

Ejemplo:

```markdown
---
name: architecture-evidence
description: Extract and classify source-grounded architectural evidence from code, configuration, infrastructure and runtime artifacts. Use before creating or changing an architecture model.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: architecture-evidence-v1
---

# Objective

Create evidence records. Do not create diagrams.

# Required process

1. Identify the source commit.
2. Use deterministic extractors first.
3. Record exact files and line ranges.
4. Separate observations from interpretations.
5. Assign confidence and freshness.
6. Return output matching architecture-evidence-v1.

# Forbidden

- Do not invent technologies.
- Do not infer protocols from names alone.
- Do not create a C4 element without evidence.
- Do not hide contradictions.
```

OpenCode solo reconoce ciertos campos estándar en el frontmatter, pero admite un mapa `metadata` de cadenas, por lo que puedes almacenar versión, madurez, esquema de salida y propietario. ([OpenCode](https://opencode.ai/docs/skills/))

---

# 4. Herramientas deterministas: `archctl`

Para tu caso crearía un núcleo en Rust llamado, por ejemplo:

```text
archctl
```

TypeScript quedaría limitado a adaptadores muy finos para OpenCode.

```text
OpenCode custom tool
         ↓
  adaptador TypeScript
         ↓
      archctl Rust
         ↓
 AST / Git / IaC / SQLite / renderizadores
```

OpenCode permite definir custom tools en `.opencode/tools/` usando TypeScript o JavaScript, pero esas definiciones pueden invocar scripts o binarios escritos en cualquier lenguaje. Las herramientas reciben además el directorio, worktree, sesión, agente y mensaje actuales. ([OpenCode](https://opencode.ai/docs/custom-tools/?utm_source=chatgpt.com))

## Comandos de `archctl`

```bash
archctl scan repository
archctl extract symbols
archctl extract dependencies
archctl extract infrastructure
archctl extract contracts
archctl ingest traces

archctl evidence add
archctl evidence query
archctl evidence conflicts

archctl model build
archctl model validate
archctl model diff
archctl model migrate

archctl view generate --type c4-container
archctl view generate --type uml-sequence

archctl render --format structurizr
archctl render --format plantuml
archctl render --format mermaid
archctl render --format drawio

archctl run checkpoint
archctl run resume
archctl run status
```

## Herramientas visibles para los agentes

Los agentes no necesitan conocer todos los comandos internos. Expondría herramientas de alto nivel:

```text
arch_scan_repository
arch_extract_static_structure
arch_extract_infrastructure
arch_ingest_runtime
arch_query_evidence
arch_submit_evidence
arch_build_model
arch_query_model
arch_generate_view
arch_render_view
arch_validate_view
arch_checkpoint
arch_resume
```

Todas devolverían JSON validado, no prosa libre.

---

# 5. Custom tools frente a MCP

Usaría esta regla:

## Custom tools locales

Para operaciones que formen parte del propio sistema:

- analizar código;
- ejecutar Tree-sitter;
- consultar Git;
- leer el grafo;
- persistir evidencias;
- renderizar PlantUML;
- validar Structurizr;
- comparar modelos;
- crear checkpoints.

## MCP

Para sistemas externos:

- GitHub/GitLab/Gitea;
- Jenkins;
- Kubernetes remoto;
- Grafana/Loki/Tempo;
- OpenTelemetry;
- catálogos de APIs;
- CMDB;
- Backstage;
- documentación corporativa;
- almacenamiento de artefactos.

Esto evita llenar el contexto con cientos de definiciones MCP que casi nunca se utilizan. La propia documentación de OpenCode advierte que las herramientas MCP consumen contexto y recomienda habilitar únicamente los servidores necesarios. ([OpenCode](https://opencode.ai/v2/docs/mcp-servers?utm_source=chatgpt.com))

Además crearía perfiles de MCP:

```text
mcp-profile-static
mcp-profile-runtime
mcp-profile-cloud
mcp-profile-corporate
```

El orquestador activaría solo el perfil necesario para cada ejecución.

### Atención con la configuración MCP

La documentación estable actual muestra servidores directamente bajo `mcp`, mientras que la documentación V2 muestra `mcp.servers`. El proyecto parece estar en transición de esquema. Conviene fijar una versión de OpenCode, validar la configuración contra su esquema y no copiar configuraciones de ramas distintas. ([OpenCode](https://opencode.ai/v2/docs/mcp-servers?utm_source=chatgpt.com))

---

# 6. Estructura del proyecto

Separaría el harness reusable del repositorio analizado.

## Repositorio del harness

```text
architecture-agent-harness/
├── opencode.jsonc
├── AGENTS.md
├── agents/
│   ├── architecture-orchestrator.md
│   ├── repo-cartographer.md
│   ├── code-structure-extractor.md
│   ├── infrastructure-extractor.md
│   ├── runtime-behaviour-extractor.md
│   ├── architecture-synthesizer.md
│   ├── c4-modeler.md
│   ├── uml-modeler.md
│   └── architecture-auditor.md
├── skills/
├── commands/
├── tools/
├── plugins/
├── schemas/
├── prompts/
├── evaluations/
├── fixtures/
├── migrations/
├── mcp/
├── crates/
│   └── archctl/
├── package.json
├── Cargo.toml
├── justfile
└── skills.lock.json
```

## Repositorio de la aplicación

```text
my-application/
├── .opencode/
│   └── project-overrides/
├── .architecture/
│   ├── manifest.yaml
│   ├── model/
│   │   ├── architecture.json
│   │   └── architecture.schema-version
│   ├── evidence/
│   │   ├── ledger.jsonl
│   │   ├── contradictions.jsonl
│   │   └── unknowns.jsonl
│   ├── views/
│   │   ├── landscape.yaml
│   │   ├── context.yaml
│   │   ├── containers.yaml
│   │   ├── components.yaml
│   │   └── scenarios.yaml
│   ├── diagrams/
│   │   ├── structurizr/
│   │   ├── plantuml/
│   │   ├── mermaid/
│   │   └── generated/
│   ├── decisions/
│   ├── policies/
│   ├── runs/
│   ├── state/
│   │   └── workflow.json
│   ├── snapshots/
│   └── reports/
└── .architecture-cache/
```

`.architecture/` se versiona con Git.

`.architecture-cache/` se ignora y contiene:

- índices AST;
- embeddings;
- SQLite;
- ficheros renderizados intermedios;
- clones externos;
- caché de hashes;
- artefactos temporales.

---

# 7. Modelo intermedio arquitectónico

No usaría directamente el metamodelo de Structurizr como almacenamiento principal. Crearía un IR neutral.

```json
{
  "schemaVersion": "1.0.0",
  "source": {
    "repository": "my-application",
    "commit": "8db31d1",
    "generatedAt": "2026-07-29T09:00:00+02:00"
  },
  "elements": [
    {
      "id": "container:orders-api",
      "kind": "container",
      "name": "Orders API",
      "technology": ["Rust", "Axum"],
      "confidence": 0.98,
      "freshness": "current",
      "evidence": [
        "ev:Cargo.toml:orders-api",
        "ev:k8s:orders-deployment"
      ]
    }
  ],
  "relationships": [
    {
      "id": "rel:gateway-orders",
      "source": "container:api-gateway",
      "target": "container:orders-api",
      "protocol": "HTTP",
      "confidence": 0.94,
      "evidence": [
        "ev:gateway:route-orders"
      ]
    }
  ]
}
```

Cada evidencia debería contener:

```json
{
  "id": "ev:gateway:route-orders",
  "type": "configuration",
  "claim": "api-gateway routes /orders to orders-api",
  "source": {
    "path": "deploy/gateway/routes.yaml",
    "startLine": 31,
    "endLine": 38,
    "commit": "8db31d1",
    "contentHash": "blake3:..."
  },
  "extractor": {
    "name": "gateway-route-parser",
    "version": "1.3.0"
  },
  "classification": "fact",
  "confidence": 1.0
}
```

Así puedes regenerar las vistas sin volver a interpretar todo el código.

---

# 8. Cómo orquestar la ejecución

## Flujo principal

```text
/intelligence-architecture discover
                  │
                  ▼
        Leer manifest y checkpoint
                  │
                  ▼
      Calcular cambios desde snapshot
                  │
        ┌─────────┼─────────┐
        ▼         ▼         ▼
      Código     IaC      Runtime
        │         │         │
        └─────────┼─────────┘
                  ▼
          Evidence ledger
                  ▼
       Detectar contradicciones
                  ▼
       Architecture synthesizer
                  ▼
          Architecture IR
             ┌────┴────┐
             ▼         ▼
            C4         UML
             └────┬────┘
                  ▼
              Render
                  ▼
        Revisión determinista
                  ▼
        Architecture auditor
                  ▼
       Aprobación / checkpoint
```

## Contrato de delegación

Cada tarea enviada a un subagente debe incluir:

```yaml
task:
  id: run-20260729-static-01
  objective: Extract static dependencies for the payment domain
  scope:
    include:
      - crates/payment/**
    exclude:
      - target/**
  input_commit: 8db31d1
  skills:
    - architecture-evidence
    - architecture-ir
  permitted_tools:
    - arch_extract_static_structure
    - arch_query_evidence
    - arch_submit_evidence
  expected_output:
    schema: architecture-evidence-v1
  completion:
    require_evidence: true
    allow_inferences: true
    minimum_confidence: 0.70
```

El orquestador no debería aceptar un «he terminado». Debe validar:

- esquema;
- commit analizado;
- evidencias;
- hashes;
- cobertura;
- errores;
- contradicciones;
- versión de herramientas.

---

# 9. Configuración de OpenCode

Una configuración conceptual sería:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",

  "default_agent": "architecture-orchestrator",
  "subagent_depth": 2,

  "instructions": [
    "AGENTS.md",
    ".architecture/policies/*.md"
  ],

  "permission": {
    "edit": "ask",
    "bash": "ask",
    "task": "allow",

    "skill": {
      "*": "deny",
      "architecture-*": "allow",
      "c4-*": "allow",
      "uml-*": "allow",
      "diagram-*": "allow"
    },

    "arch_*": "allow"
  },

  "references": {
    "architecture-harness": {
      "path": "../architecture-agent-harness",
      "description": "Architecture agents, schemas, skills and evaluation fixtures"
    }
  }
}
```

OpenCode permite añadir directorios o repositorios Git como referencias para que estén disponibles fuera del worktree actual. También puede cargarse una distribución completa desde un directorio personalizado mediante `OPENCODE_CONFIG_DIR`. ([OpenCode](https://opencode.ai/docs/config/))

Para evitar instalaciones manuales inconsistentes, utilizaría:

```bash
export OPENCODE_CONFIG_DIR="$HOME/.local/share/architecture-agent-harness"
opencode
```

Y un `justfile`:

```makefile
install:
    ./scripts/install.sh

doctor:
    cargo run -p archctl -- doctor
    opencode mcp list

test:
    cargo test --workspace
    bun test

eval:
    cargo run -p archctl -- eval evaluations/

update:
    git pull --ff-only
    bun install --frozen-lockfile
    cargo build --release
```

---

# 10. Definición del agente orquestador

```markdown
---
description: Orchestrates evidence-driven software architecture recovery, C4 modeling, UML generation and architecture drift analysis.
mode: primary
temperature: 0.1
steps: 80
permission:
  edit:
    "*": deny
    ".architecture/**": allow
  bash:
    "*": deny
    "git status*": allow
    "git diff*": allow
    "archctl *": allow
  task: allow
  skill:
    "architecture-*": allow
    "c4-*": allow
    "uml-*": allow
    "diagram-*": allow
---

You are the architecture control plane.

Never infer the complete architecture directly from filenames.

Your responsibilities are:

1. Load the current architecture workflow state.
2. Identify changed or missing evidence.
3. Delegate extraction to appropriate specialists.
4. Require structured, source-grounded results.
5. Resolve or expose contradictions.
6. Update the architecture model only after validation.
7. Treat C4 and UML diagrams as projections.
8. Checkpoint after every completed stage.
9. Never store durable knowledge only in conversation history.
10. Never modify application source code.
```

OpenCode admite agentes Markdown dentro de `.opencode/agents/`, incluyendo modelo, temperatura, permisos y prompt especializado. También permite limitar herramientas MCP y custom tools mediante patrones de nombres. ([OpenCode](https://opencode.ai/docs/agents/))

---

# 11. Persistencia: cuatro niveles

## Nivel 1: sesión OpenCode

OpenCode guarda sesiones y mensajes localmente y permite listar, exportar e importar sesiones. Esto es útil para auditoría y depuración, pero no debería ser la memoria arquitectónica principal. ([OpenCode](https://opencode.ai/docs/troubleshooting/?utm_source=chatgpt.com))

```bash
opencode session list --format json
opencode export <session-id> > session.json
opencode import session.json
```

## Nivel 2: estado de ejecución

```json
{
  "runId": "arch-20260729-001",
  "sourceCommit": "8db31d1",
  "status": "running",
  "stages": {
    "inventory": "completed",
    "staticExtraction": "completed",
    "infrastructureExtraction": "completed",
    "runtimeExtraction": "pending",
    "synthesis": "blocked",
    "render": "pending",
    "audit": "pending"
  },
  "blockedBy": [
    "runtime telemetry unavailable"
  ]
}
```

Se guarda después de cada transición.

## Nivel 3: conocimiento arquitectónico

Persistido en Git:

- evidencias;
- IR;
- vistas;
- decisiones;
- contradicciones;
- hipótesis;
- desconocidos;
- snapshots.

## Nivel 4: índice derivado

SQLite/libSQL local para:

- búsqueda semántica;
- relaciones inversas;
- consultas por fichero;
- impactos;
- similitud;
- historial temporal.

Mi recomendación es:

> **Git/texto es la autoridad; SQLite es un índice regenerable.**

No empezaría por Neo4j. Un grafo externo añade operación, migraciones y dependencia antes de saber si realmente necesitas consultas distribuidas.

---

# 12. Recuperación tras interrupciones

El sistema debe poder recuperarse sin confiar en que el LLM recuerde la conversación.

## Registro de eventos

```jsonl
{"seq":1,"type":"run.started","run":"arch-001","commit":"8db31d1"}
{"seq":2,"type":"stage.completed","stage":"inventory","artifact":"sha256:..."}
{"seq":3,"type":"agent.started","agent":"repo-cartographer","session":"ses_..."}
{"seq":4,"type":"evidence.persisted","count":148,"artifact":"sha256:..."}
{"seq":5,"type":"agent.failed","agent":"runtime-extractor","reason":"tempo unavailable"}
```

## Reanudación

```bash
archctl run resume arch-001
```

El comando:

1. Lee el último checkpoint.
2. Comprueba el commit actual.
3. Valida hashes de artefactos.
4. Invalida resultados obsoletos.
5. Identifica etapas incompletas.
6. Genera el siguiente plan.
7. Devuelve un paquete pequeño al orquestador.

## Compacción de contexto

Los plugins de OpenCode pueden intervenir antes de la compactación e inyectar contexto que deba preservarse. Yo solo inyectaría punteros:

```text
Run activo: arch-001
Estado: .architecture/state/workflow.json
Modelo: .architecture/model/architecture.json
Evidencias: .architecture/evidence/ledger.jsonl
Último checkpoint válido: static-extraction
```

No inyectaría nuevamente el grafo completo. OpenCode expone un hook específico para personalizar la compactación y conservar estado relevante. ([OpenCode](https://opencode.ai/docs/plugins/))

---

# 13. Plugin de control

Crearía:

```text
.opencode/plugins/architecture-control-plane.ts
```

Responsabilidades:

- registrar inicio y final de sesiones;
- escuchar ejecuciones de herramientas;
- persistir resultados antes de devolverlos al agente;
- crear checkpoints;
- impedir escrituras fuera de `.architecture/`;
- bloquear secretos;
- registrar métricas;
- inyectar el puntero al estado en la compactación;
- avisar de resultados huérfanos;
- validar que cada subagente devuelve su esquema.

OpenCode permite escuchar eventos de ficheros, mensajes, sesiones, permisos, tareas y ejecuciones de herramientas. También permite modificar o bloquear una herramienta antes de ejecutarse. ([OpenCode](https://opencode.ai/docs/plugins/))

Esqueleto:

```typescript
import type { Plugin } from "@opencode-ai/plugin";

export const ArchitectureControlPlane: Plugin = async ({
  client,
  worktree,
}) => {
  return {
    "tool.execute.before": async (input, output) => {
      if (
        input.tool === "write" &&
        !String(output.args.filePath).includes("/.architecture/")
      ) {
        throw new Error(
          "Architecture agents may only write below .architecture/",
        );
      }
    },

    "tool.execute.after": async (input, output) => {
      if (input.tool.startsWith("arch_")) {
        // Persist invocation metadata, result hash and session relationship.
      }
    },

    event: async ({ event }) => {
      if (event.type === "session.error") {
        // Mark active agent execution as failed but resumable.
      }

      if (event.type === "session.idle") {
        // Flush event ledger and current checkpoint.
      }
    },

    "experimental.session.compacting": async (_input, output) => {
      output.context.push(`
Architecture state:
- Read .architecture/state/workflow.json before continuing.
- Do not reconstruct completed stages from memory.
- Validate the source commit before resuming.
`);
    },
  };
};
```

---

# 14. Ejecución paralela

OpenCode crea sesiones hijas cuando utiliza subagentes, y permite navegar entre padre e hijos. ([OpenCode](https://opencode.ai/docs/agents/))

Paralelizaría solo tareas independientes:

```text
repo-cartographer ─────────┐
static-extractor ──────────┤
infrastructure-extractor ──┼──► synthesizer
runtime-extractor ─────────┘
```

No paralelizaría:

```text
synthesizer → c4-modeler → auditor
```

porque cada fase depende semánticamente de la anterior.

El plugin comunitario `opencode-background-agents` es interesante para investigaciones de solo lectura: persiste los resultados en disco y sobreviven a reinicios y compactaciones. Sin embargo, sus delegaciones en segundo plano están limitadas deliberadamente a agentes sin permisos de escritura; para trabajo modificador debe utilizarse la herramienta nativa `task`. ([GitHub](https://github.com/kdcokenny/opencode-background-agents))

Por tanto:

- búsquedas, inventarios y revisiones: segundo plano;
- actualización del modelo y artefactos: sesión hija nativa y controlada.

---

# 15. Comandos de usuario

```text
/architecture-init
/architecture-discover
/architecture-update
/architecture-status
/architecture-resume
/architecture-review
/architecture-diff
/architecture-c4 context
/architecture-c4 container payments
/architecture-uml sequence checkout
/architecture-explain container:orders-api
/architecture-evidence container:orders-api
/architecture-unknowns
/architecture-conflicts
```

Ejemplo:

```markdown
---
description: Discover or update the evidence-backed architecture model
agent: architecture-orchestrator
---

Load the architecture discovery skill.

Target scope: $ARGUMENTS

1. Read .architecture/state/workflow.json.
2. Determine whether this is a new run or a resumable run.
3. Calculate changes from the last valid architecture snapshot.
4. Delegate only the required extraction stages.
5. Validate all structured results.
6. Update the architecture model.
7. Generate affected views.
8. Run architecture audit.
9. Persist the final checkpoint and report.
```

OpenCode admite comandos Markdown parametrizados, selección de agente, argumentos y referencias directas a ficheros. ([OpenCode](https://opencode.ai/docs/commands/))

---

# 16. Evolución de las skills

No permitiría que los agentes modifiquen directamente skills estables después de una ejecución.

Crearía un ciclo parecido al desarrollo de software:

```text
Observación
    ↓
Candidato de aprendizaje
    ↓
Propuesta de cambio de skill
    ↓
Test de regresión
    ↓
Evaluación en repositorios fixture
    ↓
Revisión humana
    ↓
experimental
    ↓
canary
    ↓
stable
```

## Estados de una skill

```yaml
metadata:
  version: "1.4.0"
  maturity: experimental
  owner: architecture-platform
  output-schema: architecture-evidence-v2
```

## `skills.lock.json`

```json
{
  "architecture-evidence": {
    "version": "1.4.0",
    "commit": "21ae73c",
    "sha256": "..."
  },
  "c4-modeling": {
    "version": "2.1.2",
    "commit": "bacf987",
    "sha256": "..."
  }
}
```

Así puedes reproducir exactamente qué comportamiento generó cada modelo arquitectónico.

---

# 17. Evaluación de calidad

Crearía un corpus de repositorios fixture:

```text
evaluations/
├── rust-modular-monolith/
├── java-spring-microservices/
├── node-event-driven/
├── terraform-kubernetes/
├── jenkins-shared-library/
├── intentionally-ambiguous/
└── intentionally-inconsistent/
```

No compararía imágenes píxel a píxel. Evaluaría invariantes.

```yaml
expected:
  elements:
    - orders-api
    - orders-db
    - payment-provider

  relationships:
    - source: orders-api
      target: orders-db
      protocol: sql

  forbidden:
    - invented-message-broker

  quality:
    minimum_evidence_coverage: 0.90
    maximum_unsupported_claims: 0
    maximum_orphan_nodes: 1
    render_must_succeed: true
```

## Métricas importantes

- cobertura de evidencias;
- precisión de relaciones;
- elementos sin fuente;
- contradicciones no resueltas;
- confianza media;
- antigüedad de evidencias;
- éxito de renderizado;
- mezcla de niveles C4;
- cruces de conexiones;
- nodos huérfanos;
- estabilidad entre ejecuciones;
- coste por cambio arquitectónico detectado.

---

# 18. Arquitectura como sistema temporal

Aquí está uno de los elementos laterales más valiosos.

En vez de almacenar solo:

```text
Orders API llama a Payment Service
```

almacenaría:

```text
Orders API llamó a Payment Service
desde commit A hasta commit F,
mediante HTTP,
según estas cuatro evidencias,
con confianza 0,97.
```

Eso permite consultar:

```text
¿Cómo era la arquitectura antes de la migración?
¿Cuándo apareció esta dependencia?
¿Qué diagrama correspondía a la release 2.4?
¿Qué elementos llevan seis meses sin evidencia actual?
¿Qué relaciones existen en runtime pero no en código?
```

Cada relación tendría:

```json
{
  "validFrom": "commit:21ae73c",
  "validTo": null,
  "firstObservedAt": "2026-04-12T10:30:00Z",
  "lastVerifiedAt": "2026-07-29T07:30:00Z"
}
```

Esto convierte el sistema en un **gemelo digital temporal de la arquitectura**.

---

# 19. Arquitectura declarada, estática y observada

Mantendría tres grafos:

```text
Grafo declarado
Documentación, ADR, Structurizr, Kubernetes, Terraform

Grafo estático
Imports, dependencias, llamadas, contratos y configuración

Grafo observado
Trazas, logs, métricas, tráfico y eventos
```

Después calcularía:

```text
declarado - observado
    → arquitectura teórica no utilizada

observado - declarado
    → arquitectura oculta o drift

estático - observado
    → caminos posibles pero no vistos

observado - estático
    → reflexión, configuración dinámica, proxies o errores de extracción
```

El diagrama podría incluso mostrar:

```text
línea sólida    = observado
línea discontinua = declarado
línea tenue      = inferido
línea roja       = contradicción
```

---

# 20. El agente escéptico

Añadiría un segundo auditor llamado:

```text
architecture-falsifier
```

Recibe el modelo, pero no las conclusiones del sintetizador. Su misión es:

1. Buscar una explicación alternativa.
2. Seleccionar una muestra de elementos.
3. Volver a inspeccionar sus evidencias.
4. Buscar relaciones opuestas.
5. Detectar nombres engañosos.
6. Identificar dependencias generadas o dinámicas.
7. Proponer experimentos para resolver dudas.

Por ejemplo:

```text
Hipótesis principal:
payment-worker consume payment.created.

Hipótesis alternativa:
payment-worker solo publica payment.created.

Experimento:
buscar consumers, group IDs, bindings y trazas asociadas.
```

Esto es mucho más eficaz que pedir al mismo agente que revise su propio trabajo.

---

# 21. Preguntar solo donde aporta valor

El sistema no debería preguntarte continuamente.

Calcularía:

```text
impacto × incertidumbre × coste de equivocación
```

Solo preguntaría cuando el valor supere un umbral.

Ejemplo:

```yaml
question:
  claim: inventory-db belongs to Inventory bounded context
  confidence: 0.53
  impact: high
  evidence:
    - database name suggests inventory
    - deployment belongs to warehouse namespace
    - ownership file points to logistics team
  alternatives:
    - Inventory
    - Warehouse
    - Shared logistics platform
```

Eso convierte la interacción humana en **active learning**, no en rellenar cuestionarios.

---

# 22. Integración CI/CD

En cada pull request:

```text
git diff
   ↓
ficheros afectados
   ↓
evidencias invalidadas
   ↓
subgrafo afectado
   ↓
vistas afectadas
   ↓
regeneración parcial
   ↓
architecture diff
```

Reporte:

```yaml
architecture_diff:
  added:
    - container:fraud-service
    - relation:payment-api->fraud-service

  removed:
    - relation:payment-api->legacy-risk-api

  changed:
    - element:payment-db
      property: technology
      before: PostgreSQL 15
      after: PostgreSQL 17

  unresolved:
    - claim: fraud-service stores personal data
      confidence: 0.48
```

No fallaría el pipeline porque cambie una coordenada visual. Fallaría por:

- relación sin evidencia;
- nuevo sistema externo no aprobado;
- dependencia de dominio prohibida;
- diagrama no regenerado;
- contradicción crítica;
- violación de una política arquitectónica.

---

# 23. Uso programático del SDK

Para el MVP basta con el agente primario y comandos. Para una versión madura utilizaría el servidor y SDK de OpenCode como motor.

OpenCode puede ejecutarse como servidor headless, crear sesiones y sesiones hijas, consultar estados y recibir eventos. Su SDK permite solicitar respuestas estructuradas validadas mediante JSON Schema. ([OpenCode](https://opencode.ai/docs/server/))

Eso permite que `archctl orchestrate` controle OpenCode:

```text
archctl
   │
   ├─ crea sesión del subagente
   ├─ envía contrato y esquema JSON
   ├─ espera eventos SSE
   ├─ valida respuesta
   ├─ persiste resultado
   └─ avanza máquina de estados
```

Este modelo híbrido es mi recomendación final:

```text
OpenCode primario
    = interfaz conversacional y razonamiento

Plugin OpenCode
    = seguridad, eventos y contexto

archctl
    = máquina de estados y herramientas deterministas

Subagentes
    = especialistas cognitivos

Architecture IR
    = memoria y fuente de verdad
```

---

# 24. Qué reutilizaría y qué construiría

## Reutilizaría

- agentes y subagentes nativos de OpenCode;
- skills nativas;
- permisos;
- sesiones hijas;
- servidor y SDK;
- Structurizr;
- C4-PlantUML;
- Mermaid;
- draw.io;
- PlantUML;
- renderizado local;
- ideas de persistencia de `opencode-background-agents`;
- algunos patrones de `opencode-workspace`.

`opencode-workspace` demuestra una separación útil entre orquestadores y especialistas con permisos distintos, aunque su objetivo es desarrollo general y no arquitectura. ([GitHub](https://github.com/kdcokenny/opencode-workspace))

## Construiría

- `archctl`;
- Architecture IR;
- evidence ledger;
- plugin de control;
- schemas JSON;
- contratos de delegación;
- skills específicas;
- extractores de código e infraestructura;
- arquitectura temporal;
- auditor falsificador;
- drift detector;
- corpus de evaluación.

## No usaría como fuente de verdad

- historial de conversaciones;
- memoria vectorial;
- Mermaid;
- PlantUML;
- imágenes;
- resúmenes del LLM.

Las memorias persistentes comunitarias como Supermemory u `opencode-mem` pueden ser útiles para preferencias, convenciones o recuerdos blandos entre sesiones, pero mantendría el modelo arquitectónico y sus evidencias en el repositorio. Supermemory inyecta recuerdos relevantes y conserva contexto durante compactaciones; `opencode-mem` ofrece almacenamiento vectorial local. ([GitHub](https://github.com/supermemoryai/opencode-supermemory))

---

# 25. Roadmap recomendado

## Fase 1 — Harness nativo

- agente orquestador;
- cuatro extractores;
- skills;
- permisos;
- comandos;
- IR JSON;
- evidence ledger;
- generación Structurizr y PlantUML;
- auditoría básica.

## Fase 2 — Núcleo determinista

- `archctl`;
- SQLite derivado;
- Tree-sitter/LSP;
- extractores IaC;
- hashes;
- checkpoints;
- recuperación.

## Fase 3 — Plugin OpenCode

- captura de eventos;
- control de escrituras;
- persistencia automática;
- compacción;
- observabilidad;
- ejecución paralela.

## Fase 4 — Evolución continua

- architecture diff;
- CI;
- snapshots temporales;
- corpus de evaluación;
- proposals automáticas de mejora de skills;
- promoción experimental/canary/stable.

## Fase 5 — Gemelo arquitectónico

- runtime OpenTelemetry;
- arquitectura declarada/estática/observada;
- navegación HTML;
- consultas naturales;
- impacto de cambios;
- Canvas C4 interactivo;
- integración con tldraw/draw.io;
- simulación y escenarios contrafactuales.

---

## Decisión final

La solución más robusta no es un «swarm» autónomo sin control. Es:

```text
Un agente orquestador fino
+
subagentes con permisos mínimos
+
skills cargadas bajo demanda
+
herramientas deterministas
+
un grafo arquitectónico neutral
+
un ledger de evidencias
+
checkpoints recuperables
+
diagramas como vistas
+
un agente que intente refutar el resultado
```

Esto te daría un sistema capaz de aprender y evolucionar sin convertir sus conclusiones anteriores en dogmas, porque cada elemento seguiría vinculado a su evidencia, confianza, versión, fecha y commit de origen.

---

# you asked

message time: 2026-07-29 09:59:15

ok no se si podemos reutilizar algunas tools existentes como astgrep a lo mejor para cubrir parte  responsabilidades de lo planteado o ayudar en la funcionalidad, investiga otras tools cli como astgrep que puedan ayudarnos a no reinventar la rueda. Lo mismo digo en skill no reinventemos usamos skill ya estudiadas pero adaptadas. Otra cosa es que no me gustaria contaminar el repo de git en la medida de lo posible con estos datos por lo que seria interesante manejarlo desde directorios de usuario organizados por proyectos, de manera que sea un espejo del repo git.

---

# chatgpt response

Voy a centrar la investigación en herramientas reutilizables por responsabilidad: extracción estructural, grafos, seguridad/semántica, IaC, runtime y renderizado. También revisaré cómo encajarlo fuera del repositorio usando XDG y una identidad estable por proyecto/worktree.

La conclusión preliminar es clara: `ast-grep` encaja muy bien como extractor de patrones, pero no debe convertirse en nuestro parser universal. Para relaciones precisas conviene priorizar índices de compilador/LSP o SCIP, y reservar CodeQL/Joern para análisis profundo y bajo demanda.

---

# you asked

message time: 2026-07-29 11:13:40

hola como va

---

# chatgpt response

Va bien. La línea principal ya está clara:

- **`ast-grep`** sirve para detectar patrones estructurales, APIs, anotaciones, convenciones y cambios, pero no debería ser nuestro analizador universal.
- Para relaciones semánticas reales conviene reutilizar **SCIP/LSIF, LSP, Tree-sitter, CodeQL o Joern**, según lenguaje y profundidad.
- Para IaC y despliegue podemos aprovechar herramientas como **Terraform Graph, Helm, kube-lineage, Trivy, Checkov y Syft**.
- Las skills estudiadas se pueden incorporar como dependencias versionadas y envolverlas con adaptadores propios, sin copiarlas ni reescribirlas completamente.
- La persistencia puede quedar **totalmente fuera del repositorio**, bajo rutas XDG como `~/.local/share/<herramienta>/projects/...`, creando un espejo estable del repo basado en remoto, ruta y worktree.

No trabajo en segundo plano: continúo ahora con la investigación y te entregaré una propuesta consolidada de herramientas, arquitectura de directorios externos y estrategia de adaptación de skills.

---

# you asked

message time: 2026-07-29 11:22:33

ok no se si podemos reutilizar algunas tools existentes como astgrep a lo mejor para cubrir parte responsabilidades de lo planteado o ayudar en la funcionalidad, investiga otras tools cli como astgrep que puedan ayudarnos a no reinventar la rueda. Lo mismo digo en skill no reinventemos usamos skill ya estudiadas pero adaptadas. Otra cosa es que no me gustaria contaminar el repo de git en la medida de lo posible con estos datos por lo que seria interesante manejarlo desde directorios de usuario organizados por proyectos, de manera que sea un espejo del repo git.

---

# chatgpt response

Voy a convertirlo en una propuesta de reutilización real: qué herramienta existente cubre cada responsabilidad, dónde encaja `ast-grep`, qué piezas conviene evitar duplicar y cómo montar un almacenamiento XDG externo que refleje cada repositorio y worktree sin ensuciar Git.

Sí: **debemos rediseñar `archctl` para que no implemente analizadores**, sino que actúe como una capa de orquestación, normalización, persistencia y selección de herramientas existentes.

La arquitectura debería seguir esta regla:

> La herramienta existente extrae hechos; `archctl` conserva la procedencia, normaliza los resultados y construye el modelo arquitectónico; los agentes interpretan únicamente lo que las herramientas no pueden determinar.

---

# 1. Qué responsabilidades no debemos implementar

No construiría desde cero:

- Parser multilenguaje.
- Índice de símbolos.
- Resolución de referencias.
- Call graph.
- Grafo de dependencias de paquetes.
- Parser de Terraform, Kubernetes o Helm.
- Generador de SBOM.
- Motor de consultas AST.
- Renderizador PlantUML, Mermaid, Structurizr o draw.io.
- Motor de análisis de flujo de datos.
- Gestor de worktrees o detector de cambios Git.

El núcleo propio se reduciría a:

```text
archctl
├── project-resolver
├── capability-router
├── tool-adapters
├── evidence-normalizer
├── evidence-store
├── architecture-ir
├── workflow-engine
├── checkpoint-manager
├── skill-registry
└── opencode-integration
```

---

# 2. Dónde encaja `ast-grep`

`ast-grep` es especialmente adecuado para tres responsabilidades.

## A. Inventario estructural rápido

Su comando `outline` puede extraer símbolos, imports, exports, miembros, firmas y estructura, con salida JSON. Esto permite utilizarlo como primer cartógrafo del repositorio sin tener que crear nuestros propios extractores Tree-sitter. ([Ast Grep](https://ast-grep.github.io/reference/cli.html?utm_source=chatgpt.com))

Ejemplo conceptual:

```bash
ast-grep outline \
  --items all \
  --view signatures \
  --json=stream \
  /ruta/al/repositorio
```

Esto puede producir una primera capa de evidencias:

```json
{
  "kind": "function",
  "name": "create_order",
  "path": "src/orders/application.rs",
  "language": "Rust",
  "visibility": "public"
}
```

## B. Extractores específicos de frameworks

Aquí es donde realmente brilla:

- Controladores Spring.
- Endpoints Axum, Actix o FastAPI.
- Consumers y producers Kafka.
- Decoradores NestJS.
- Handlers de eventos.
- Repositorios JPA.
- Adaptadores hexagonales.
- Clientes HTTP.
- Acceso a bases de datos.
- Registro de dependencias.
- Anotaciones de seguridad.
- Definición de comandos Jenkins.

Sus reglas pueden combinar patrones AST, relaciones contextuales y metavariables. Además, puede emitir JSON Lines y metadatos personalizados, lo que facilita convertir cada coincidencia en una evidencia arquitectónica. ([Ast Grep](https://ast-grep.github.io/reference/yaml.html?utm_source=chatgpt.com))

Ejemplo de metadatos de una regla:

```yaml
id: spring-rest-controller
language: Java

rule:
  pattern: |
    @RestController
    class $CONTROLLER {
      $$$BODY
    }

message: Spring REST controller

severity: info

metadata:
  architecture.kind: component
  architecture.stereotype: inbound-http-adapter
  architecture.confidence: "0.95"
```

## C. Reglas arquitectónicas y detección de drift

Puede comprobar convenciones como:

- El dominio no importa infraestructura.
- Un controller no accede directamente al repositorio.
- Ciertas APIs deben estar detrás de un adaptador.
- Los handlers deben utilizar un bus de comandos.
- Los módulos no pueden crear dependencias circulares.
- Todo endpoint administrativo debe declarar autorización.

`ast-grep scan` admite configuración externa mediante `--config`, reglas individuales mediante `--rule` y salida JSON o SARIF. Esto permite mantener todas las reglas fuera del repositorio analizado. ([Ast Grep](https://ast-grep.github.io/reference/cli/scan.html?utm_source=chatgpt.com))

```bash
ast-grep scan \
  --config "$ARCHCTL_CONFIG/ast-grep/sgconfig.yml" \
  --json=stream \
  "$REPOSITORY_ROOT"
```

### Limitación importante

`ast-grep` conoce la estructura sintáctica, pero no proporciona por sí solo una resolución semántica completa de:

- Qué definición corresponde realmente a una referencia.
- Implementaciones de interfaces.
- Despacho dinámico.
- Sobrecarga.
- Resolución entre repositorios.
- Call graph preciso.
- Flujo de datos interprocedimental.

Además, su configuración global todavía no sigue XDG de forma nativa, por lo que nuestro adaptador debe pasar siempre `--config` explícitamente. ([Ast Grep](https://ast-grep.github.io/guide/project/project-config.html?utm_source=chatgpt.com))

---

# 3. Herramientas complementarias que reutilizaría

## Nivel 0: descubrimiento e inventario rápido

Estas herramientas se ejecutarían casi siempre porque son rápidas.

### Git

Git ya proporciona todo lo necesario para:

- Detectar raíz del repositorio.
- Resolver el directorio común entre worktrees.
- Enumerar worktrees.
- Obtener el remoto.
- Detectar ficheros cambiados.
- Obtener commits y ramas.
- Diferenciar cambios staged y no staged.

`git rev-parse --show-toplevel` y `--git-common-dir` permiten distinguir la copia de trabajo del repositorio común; `git worktree list --porcelain -z` ofrece un formato estable para automatización. ([Git](https://git-scm.com/docs/git-rev-parse?utm_source=chatgpt.com))

### Universal Ctags

Lo utilizaría como **fallback multilenguaje ultrarrápido** para obtener símbolos, clases, funciones, namespaces, variables, enums y otras declaraciones.

Puede producir JSON Lines:

```bash
ctags \
  --output-format=json \
  --fields=+nKSE \
  --extras=+q \
  -R "$REPOSITORY_ROOT"
```

También dispone de modo interactivo JSON sobre stdin/stdout, interesante para mantener un proceso caliente. ([Guía de Hacking de Ctags](https://docs.ctags.io/en/latest/man/ctags-json-output.5.html?utm_source=chatgpt.com))

Su valor sería:

```text
ast-grep outline
    ↓
inventario estructurado principal

Universal Ctags
    ↓
cobertura de lenguajes no soportados o resultados faltantes
```

### Tree-sitter

No lo usaría directamente mientras `ast-grep` cubra el caso. `ast-grep` ya utiliza Tree-sitter internamente y permite añadir lenguajes personalizados.

Tree-sitter quedaría reservado para:

- Lenguajes sin soporte de `ast-grep`.
- Extractores incrementales propios muy especializados.
- Consultas que necesiten acceso directo al CST.
- Lenguajes internos o DSL corporativas.

Tree-sitter construye árboles sintácticos incrementales y permite ejecutar consultas sobre nodos mediante patrones S-expression. ([Tree-sitter](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html?utm_source=chatgpt.com))

---

# 4. Resolución semántica: SCIP primero

Para relaciones precisas emplearía **SCIP como formato semántico principal** cuando exista indexador para el lenguaje.

SCIP captura:

- Definiciones.
- Referencias.
- Símbolos.
- Implementaciones.
- Navegación entre ficheros.
- Relaciones entre repositorios.
- Información semántica producida por compiladores o indexadores.

En 2026 SCIP pasó hacia una gobernanza comunitaria y continúa utilizándose como formato multilenguaje de inteligencia de código. ([Sourcegraph](https://sourcegraph.com/blog/the-future-of-scip?utm_source=chatgpt.com))

La estrategia sería:

```text
SCIP disponible
    → usarlo como autoridad de símbolos y referencias

SCIP no disponible
    → LSP / herramienta nativa del lenguaje

Sin índice semántico
    → ast-grep + Ctags + inferencia con menor confianza
```

SCIP no tiene por qué requerir Sourcegraph como servidor. Podemos ejecutar los indexadores compatibles, almacenar los índices externamente y normalizar la información que necesitemos.

---

# 5. Stack Graphs como fallback avanzado

Stack Graphs permite describir reglas de resolución de nombres para lenguajes arbitrarios y tiene una implementación en Rust. Es interesante cuando no existe un indexador SCIP adecuado, especialmente para lenguajes o DSL donde sí podemos modelar imports y scopes. ([GitHub](https://github.com/github/stack-graphs?utm_source=chatgpt.com))

No lo pondría en el MVP porque definir correctamente las reglas de cada lenguaje es costoso.

Lo reservaría para:

```text
Lenguaje sin SCIP
+
LSP insuficiente
+
necesidad real de resolución cross-file
```

---

# 6. Aprovechar las herramientas nativas de cada ecosistema

Un error sería inferir dependencias mirando únicamente imports. Los gestores de construcción ya conocen el grafo resuelto.

## Rust

```bash
cargo metadata --format-version 1
```

Devuelve en JSON los miembros del workspace, paquetes, targets y dependencias resueltas. Cargo recomienda indicar explícitamente la versión del formato. ([Documentación de Rust](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html?utm_source=chatgpt.com))

Lo utilizaríamos para:

- Crates.
- Workspaces.
- Targets.
- Features.
- Dependencias internas y externas.
- Binarios, librerías, ejemplos y tests.

## Maven

```bash
mvn dependency:tree \
  -DoutputType=json \
  -DoutputFile=/ruta/externa/dependencies.json
```

El plugin oficial puede emitir texto, JSON, DOT, GraphML y TGF. ([Apache Maven](https://maven.apache.org/plugins/maven-dependency-plugin/tree-mojo.html?utm_source=chatgpt.com))

## Gradle

```bash
./gradlew dependencies
./gradlew dependencyInsight --dependency <nombre>
```

Gradle puede producir grafos completos de dependencias, explicar el origen de una dependencia y generar informes HTML mediante el Project Report Plugin. ([Gradle](https://docs.gradle.org/current/userguide/viewing_debugging_dependencies.html?utm_source=chatgpt.com))

Para evitar parsear texto eternamente, el adaptador maduro podría utilizar la Tooling API de Gradle, pero para el MVP basta con los informes oficiales encapsulados.

## Go

```bash
go list -deps -json ./...
go list -m -json all
```

La salida JSON de `go list` y `go list -m` proporciona paquetes, módulos y dependencias transitivas para automatización. ([Go](https://go.dev/ref/mod?utm_source=chatgpt.com))

## Java compilado

```bash
jdeps --dot-output "$OUTPUT" application.jar
```

`jdeps` analiza dependencias a nivel de paquete o clase a partir de `.class`, directorios o JAR y puede producir DOT. Es especialmente valioso porque analiza lo que realmente terminó compilado. ([Oracle Docs](https://docs.oracle.com/en/java/javase/11/tools/jdeps.html?utm_source=chatgpt.com))

## JavaScript y TypeScript

**dependency-cruiser** es mejor que escribir un extractor propio de imports. Analiza JavaScript/TypeScript, valida reglas arquitectónicas, detecta dependencias y puede emitir su representación interna como JSON. ([GitHub](https://github.com/sverweij/dependency-cruiser?utm_source=chatgpt.com))

```bash
depcruise \
  --output-type json \
  --output-to "$OUTPUT/dependencies.json" \
  src
```

Madge puede quedar como herramienta secundaria para ciclos y visualización rápida, pero dependency-cruiser ofrece mejor encaje como motor de políticas. ([GitHub](https://github.com/pahen/madge?utm_source=chatgpt.com))

## Python

Priorizaría:

1. SCIP cuando esté disponible.
2. LSP/type checker.
3. `ast-grep`.
4. `pydeps` como señal complementaria, nunca como autoridad.

`pydeps` genera grafos de módulos, pero sus propias incidencias muestran casos límite con paquetes editables, imports condicionales y módulos omitidos; por ello le asignaría una confianza inferior. ([GitHub](https://github.com/thebjorn/pydeps?utm_source=chatgpt.com))

---

# 7. Documentación estructural existente

## Doxygen XML

Doxygen puede producir XML estructurado con clases, namespaces, ficheros, funciones y documentación. Cada compound tiene su fichero y existe un `index.xml` que los referencia. ([Doxygen](https://www.doxygen.nl/manual/customize.html?utm_source=chatgpt.com))

Resulta útil para:

- C/C++.
- Java.
- C#.
- Python.
- Proyectos que ya tengan comentarios Doxygen.

No generaríamos UML directamente desde Doxygen; transformaríamos su XML a evidencias y al IR arquitectónico.

## Rustdoc JSON

Rustdoc tiene una salida JSON experimental con tipos, traits, impls, funciones, spans y estructura pública. Sigue requiriendo nightly y opciones inestables, por lo que sería un adaptador opcional y versionado. ([Documentación de Rust](https://doc.rust-lang.org/beta/rustdoc/unstable-features.html?utm_source=chatgpt.com))

---

# 8. Análisis profundo bajo demanda

Estas herramientas no se ejecutarían en cada actualización.

## Semgrep

Lo utilizaría para:

- Políticas arquitectónicas.
- Extracción de patrones de framework.
- Taint analysis localizado.
- Reglas de seguridad.
- Restricciones de capas.
- Invariantes de dominio.

Semgrep permite extraer metavariables y detectar patrones en código, JSON y YAML. Sin embargo, la edición Community está limitada principalmente al análisis por fichero; el análisis cross-file completo pertenece a sus capacidades comerciales. ([Semgrep](https://semgrep.dev/docs/writing-rules/rule-ideas?utm_source=chatgpt.com))

Mi reparto sería:

```text
ast-grep
    → descubrimiento estructural rápido y reglas arquitectónicas sencillas

Semgrep
    → análisis de flujo y políticas donde aporte más precisión
```

## Joern

Joern construye un **Code Property Graph**, combinando AST, control flow, call graph y dataflow. Permite consultar callers, callees y flujos de datos mediante CPGQL y ejecutar scripts sin sesión interactiva. ([Joern Docs](https://docs.joern.io/quickstart/?utm_source=chatgpt.com))

Lo usaría cuando el usuario pregunte:

- ¿Qué caminos llevan hasta esta base de datos?
- ¿Por qué endpoint puede entrar este dato?
- ¿Qué funciones participan en esta transacción?
- ¿Qué componentes dependen indirectamente de esta API?
- ¿Qué secuencia UML representa realmente este caso?

No lo ejecutaría sobre todo el repositorio por defecto.

## CodeQL

CodeQL puede crear una base de datos consultable del código y ejecutar queries, produciendo SARIF u otros resultados. Es potente para análisis profundo y caminos de datos. ([GitHub Docs](https://docs.github.com/en/code-security/reference/code-scanning/codeql/codeql-cli-manual/database-analyze?utm_source=chatgpt.com))

Pero no lo pondría como dependencia obligatoria: el uso del CLI sobre código cerrado está sujeto a condiciones y normalmente requiere licencia comercial adecuada. ([GitHub](https://github.com/github/codeql?utm_source=chatgpt.com))

Por tanto:

```text
Joern
    → opción profunda abierta por defecto

CodeQL
    → adaptador opcional cuando la licencia lo permita
```

---

# 9. Infraestructura sin crear parsers propios

## Terraform

Usaría dos fuentes oficiales:

```bash
terraform graph
terraform show -json plan.out
```

`terraform graph` produce DOT con el grafo de dependencias; `terraform show -json` expone configuración, plan y estado en formato procesable. ([HashiCorp Developer](https://developer.hashicorp.com/terraform/cli/commands/graph?utm_source=chatgpt.com))

El JSON sería la evidencia primaria. DOT sería útil para contrastar relaciones.

## Helm

```bash
helm template release ./chart \
  --values values.yaml
```

`helm template` renderiza localmente los manifiestos finales. Es preferible analizar el resultado renderizado antes que intentar comprender manualmente todas las plantillas Go y valores de Helm. ([helm.sh](https://helm.sh/docs/helm/helm_template/?utm_source=chatgpt.com))

Flujo:

```text
Chart + values
      ↓
helm template
      ↓
manifiestos Kubernetes
      ↓
normalizador Kubernetes
```

## Kubernetes

Para repositorios:

```bash
kubectl create --dry-run=client -f manifests/ -o json
```

Para clústeres:

```bash
kubectl get deployment,statefulset,daemonset,service,ingress,pod \
  --all-namespaces \
  -o json
```

`kubectl get` soporta JSON y selectores, lo que evita parsear la presentación textual. ([Kubernetes](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_get/?utm_source=chatgpt.com))

El normalizador calcularía relaciones como:

```text
Deployment → ReplicaSet → Pod
Service → selector → Pod
Ingress → Service
Pod → ConfigMap
Pod → Secret
Pod → PVC
Workload → ServiceAccount
Workload → Container image
```

## SBOM

Syft puede generar inventarios en CycloneDX, SPDX y Syft JSON a partir de directorios e imágenes. ([GitHub](https://github.com/anchore/syft?utm_source=chatgpt.com))

```bash
syft "$REPOSITORY_ROOT" -o cyclonedx-json
syft "registry/image:tag" -o syft-json
```

Esto ayuda a detectar:

- Tecnologías.
- Bibliotecas.
- Paquetes del sistema.
- Imágenes.
- Versiones.
- Dependencias empaquetadas pero no visibles directamente en el código.

Trivy puede complementar el SBOM y examinar ficheros, imágenes y configuraciones, aunque sus resultados de seguridad no deben confundirse con relaciones arquitectónicas. ([Trivy](https://trivy.dev/docs/latest/supply-chain/sbom/?utm_source=chatgpt.com))

---

# 10. Selección de herramientas por niveles

No ejecutaría todas las herramientas en cada petición.

## Perfil `fast`

Se ejecuta al abrir un repositorio o tras cambios pequeños:

```text
Git
ast-grep outline
ast-grep rule packs
Universal Ctags
gestor de paquetes nativo
detección de manifests
```

Objetivo: segundos y bajo coste.

## Perfil `semantic`

Se activa cuando necesitamos referencias precisas:

```text
SCIP
LSP o compilador
dependency-cruiser
jdeps
Doxygen XML
Terraform/Helm/Kubernetes
```

Objetivo: construir relaciones fiables.

## Perfil `deep`

Solo bajo demanda:

```text
Joern
Semgrep dataflow
CodeQL
trazas runtime
análisis visual profundo
```

Objetivo: resolver preguntas específicas o contradicciones.

El agente no debería decidir comandos concretos. Pediría capacidades:

```text
extract.symbols
extract.framework-endpoints
resolve.references
resolve.callers
resolve.dependencies
extract.infrastructure
analyze.dataflow
render.c4
render.uml
```

Y el router elegiría la herramienta adecuada según lenguaje, disponibilidad, coste y precisión.

---

# 11. Registro declarativo de adaptadores

Cada herramienta tendría un descriptor.

```yaml
id: ast-grep
version-command:
  - ast-grep
  - --version

capabilities:
  - extract.symbols
  - extract.imports
  - extract.framework-patterns
  - validate.architecture-rules

commands:
  outline:
    executable: ast-grep
    args:
      - outline
      - --items
      - all
      - --view
      - signatures
      - --json=stream
      - "${repository.root}"

  scan:
    executable: ast-grep
    args:
      - scan
      - --config
      - "${tool.config}"
      - --json=stream
      - "${repository.root}"

output:
  format: jsonl
  normalizer: ast-grep-v1

quality:
  fact-kind: syntactic
  default-confidence: 0.88
  supports-incremental: true

security:
  network-access: false
  writes-repository: false
```

Otro adaptador podría proporcionar la misma capacidad:

```yaml
id: scip-rust
capabilities:
  - resolve.references
  - resolve.definitions
  - resolve.implementations

quality:
  fact-kind: semantic
  default-confidence: 0.99
```

Esto permite sustituir herramientas sin cambiar skills ni agentes.

---

# 12. Nueva arquitectura completa

```text
                              OpenCode
                                 │
                    architecture-orchestrator
                                 │
                      capacidades solicitadas
                                 │
                                 ▼
                     ┌──────────────────────┐
                     │ Capability Router    │
                     └──────────┬───────────┘
                                │
      ┌──────────────┬──────────┼──────────┬──────────────┐
      ▼              ▼          ▼          ▼              ▼
  ast-grep         SCIP       Joern     Terraform       kubectl
  Ctags            LSP        Semgrep    Helm            Syft
  native builds    jdeps      CodeQL     Compose         Trivy
      │              │          │          │              │
      └──────────────┴──────────┼──────────┴──────────────┘
                                ▼
                     Evidence Normalizer
                                ▼
                       Evidence Ledger
                                ▼
                     Architecture IR temporal
                                ▼
             ┌──────────────────┼──────────────────┐
             ▼                  ▼                  ▼
       C4 modeler          UML modeler        Drift auditor
             │                  │                  │
             └──────────────────┼──────────────────┘
                                ▼
            Structurizr / PlantUML / Mermaid / draw.io
```

---

# 13. No contaminar el repositorio

Estoy de acuerdo: **por defecto no escribiría absolutamente nada dentro del repositorio**.

OpenCode permite cargar agentes, comandos, skills y plugins globalmente, y también cargar una estructura completa desde un directorio personalizado mediante `OPENCODE_CONFIG_DIR`. Las skills globales pueden residir en `~/.config/opencode/skills/`; los plugins globales en `~/.config/opencode/plugins/`. ([OpenCode](https://opencode.ai/docs/skills/))

Usaría una distribución dedicada:

```text
$XDG_CONFIG_HOME/archctl/opencode/
├── opencode.jsonc
├── AGENTS.md
├── agents/
├── commands/
├── skills/
├── plugins/
└── tools/
```

Y lanzaría OpenCode mediante un wrapper:

```bash
#!/usr/bin/env bash
set -euo pipefail

export OPENCODE_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/archctl/opencode"
export ARCHCTL_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}/archctl"
export ARCHCTL_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}/archctl"
export ARCHCTL_STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}/archctl"
export ARCHCTL_CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}/archctl"

exec opencode "$@"
```

Lo llamaría:

```text
archcode
```

El repositorio seguiría completamente limpio:

```text
my-project/
├── src/
├── Cargo.toml
├── deploy/
└── ...
```

Sin:

```text
.opencode/
.architecture/
sgconfig.yml
rules/
generated-diagrams/
```

---

# 14. Separación correcta usando XDG

La especificación XDG separa configuración, datos persistentes, estado mutable y caché. ([Especificaciones de Freedesktop](https://specifications.freedesktop.org/basedir/?utm_source=chatgpt.com))

## Configuración

```text
~/.config/archctl/
```

Contiene:

```text
config.yaml
opencode/
tool-registry/
rule-packs/
skillsets/
project-overrides/
policies/
```

## Datos persistentes

```text
~/.local/share/archctl/
```

Contiene:

```text
projects/
skills/
schemas/
models/
snapshots/
```

## Estado operativo

```text
~/.local/state/archctl/
```

Contiene:

```text
runs/
checkpoints/
locks/
events/
active-sessions/
```

## Caché regenerable

```text
~/.cache/archctl/
```

Contiene:

```text
ast-grep/
ctags/
scip/
joern/
codeql/
sbom/
rendered/
indexes/
```

La regla sería:

```text
Eliminar ~/.cache/archctl
    → no se pierde conocimiento

Eliminar ~/.local/state/archctl
    → se pierden ejecuciones en curso, no el modelo

Eliminar ~/.local/share/archctl
    → se pierde la memoria arquitectónica persistente
```

---

# 15. Identidad estable de repositorios y worktrees

Necesitamos distinguir tres conceptos:

```text
Repository
    identidad lógica compartida

Clone
    una copia física del repositorio

Worktree
    una vista concreta de rama/HEAD y cambios locales
```

## Identidad del repositorio

Primero resolvería:

```bash
git remote get-url origin
git rev-parse --git-common-dir
git rev-parse --show-toplevel
git rev-list --max-parents=0 HEAD
```

El remoto se normalizaría:

```text
git@github.com:empresa/proyecto.git
https://github.com/empresa/proyecto.git
ssh://git@github.com/empresa/proyecto.git
```

A:

```text
github.com/empresa/proyecto
```

Después:

```text
repository_id =
  BLAKE3(normalized_remote_url + root_commit)
```

Cuando no exista remoto:

```text
repository_id =
  BLAKE3(realpath(git_common_dir) + root_commit)
```

Git ofrece directamente la raíz, el directorio común y la configuración del remoto necesarios para hacerlo. ([Git](https://git-scm.com/docs/git-rev-parse?utm_source=chatgpt.com))

## Identidad del worktree

```text
worktree_id =
  BLAKE3(repository_id + realpath(show_toplevel))
```

La rama no forma parte de la identidad porque un worktree puede cambiar de rama.

---

# 16. Estructura del espejo externo

Ejemplo:

```text
~/.local/share/archctl/projects/
└── github.com/
    └── empresa/
        └── payment-platform--4f81a22c/
            ├── project.json
            ├── aliases.json
            ├── repository/
            │   ├── evidence/
            │   │   ├── by-path/
            │   │   │   ├── src/
            │   │   │   │   └── payments/
            │   │   │   │       └── service.rs.jsonl
            │   │   │   └── deploy/
            │   │   │       └── payment.yaml.jsonl
            │   │   ├── relationships.jsonl
            │   │   ├── contradictions.jsonl
            │   │   └── unknowns.jsonl
            │   ├── model/
            │   │   ├── current.json
            │   │   └── history/
            │   ├── views/
            │   │   ├── c4/
            │   │   └── uml/
            │   ├── decisions/
            │   └── snapshots/
            │       └── <commit-sha>/
            └── worktrees/
                ├── main--a81e2c/
                │   ├── source.json
                │   ├── head.json
                │   ├── overlay/
                │   └── runs/
                └── feature-x--44a12d/
                    ├── source.json
                    ├── head.json
                    ├── overlay/
                    └── runs/
```

## Espejo por rutas

No copiaría el código fuente completo. Mantendría un espejo de sus rutas:

```text
Repositorio:
src/payment/service.rs

Espejo:
evidence/by-path/src/payment/service.rs.jsonl
```

Cada evidencia incluiría:

```json
{
  "sourcePath": "src/payment/service.rs",
  "sourceCommit": "3219ac...",
  "startLine": 51,
  "endLine": 73,
  "contentHash": "blake3:...",
  "extractor": "ast-grep",
  "extractorVersion": "0.40.0",
  "rule": "axum-http-handler",
  "classification": "fact",
  "confidence": 0.96
}
```

Por defecto almacenaría:

- Ruta relativa.
- Líneas.
- Hash.
- Símbolo.
- Tipo de evidencia.
- Resultado normalizado.

Los fragmentos completos de código serían opcionales para evitar duplicar código sensible.

---

# 17. Modelo repository + overlay

La memoria arquitectónica se separaría en dos capas.

## Base confirmada

```text
repository/model/current.json
```

Representa un commit confirmado.

## Overlay del worktree

```text
worktrees/<id>/overlay/
```

Representa:

- Cambios sin commit.
- Cambios staged.
- Rama actual.
- Evidencias provisionales.
- Modelo provisional.
- Diagramas provisionales.

Cuando el commit se crea:

```text
overlay validado
      ↓
snapshot del commit
      ↓
actualización del modelo base
      ↓
limpieza del overlay
```

Esto evita que dos worktrees se pisen entre sí.

---

# 18. Caché por contenido, no por ruta

Los resultados costosos se identificarían por:

```text
tool
+ tool_version
+ configuration_hash
+ commit
+ source_file_hash
```

Ejemplo:

```text
~/.cache/archctl/projects/<project-id>/
└── objects/
    └── blake3/
        ├── 21/
        │   └── 21f4...
        └── a7/
            └── a7b3...
```

Así:

- Dos worktrees con el mismo commit reutilizan el análisis.
- Mover el repositorio no invalida el resultado.
- Clonar de nuevo el mismo remoto puede reutilizar conocimiento.
- Solo se recalculan los ficheros modificados.
- Diferentes versiones de una herramienta no mezclan resultados.

---

# 19. Cómo integrar OpenCode sin añadir ficheros al proyecto

El plugin global recibe de OpenCode tanto `directory` como `worktree`, de modo que puede resolver automáticamente el proyecto externo correspondiente. También puede inyectar variables a las ejecuciones shell y registrar custom tools. ([OpenCode](https://opencode.ai/docs/plugins/))

Flujo:

```text
Usuario ejecuta archcode dentro de un repo
                    ↓
Plugin recibe directory/worktree
                    ↓
archctl project resolve
                    ↓
crea o localiza el espejo XDG
                    ↓
inyecta:
ARCHCTL_PROJECT_ID
ARCHCTL_PROJECT_HOME
ARCHCTL_WORKTREE_ID
ARCHCTL_SOURCE_ROOT
                    ↓
agente orquestador disponible
```

Plugin conceptual:

```typescript
import type { Plugin } from "@opencode-ai/plugin";

export const ArchitectureWorkspace: Plugin = async ({
  directory,
  worktree,
  $,
}) => {
  const result =
    await $`archctl project resolve --cwd ${directory} --worktree ${worktree} --json`;

  const project = JSON.parse(result.stdout.toString());

  return {
    "shell.env": async (_input, output) => {
      output.env.ARCHCTL_PROJECT_ID = project.repositoryId;
      output.env.ARCHCTL_PROJECT_HOME = project.projectHome;
      output.env.ARCHCTL_WORKTREE_ID = project.worktreeId;
      output.env.ARCHCTL_SOURCE_ROOT = project.sourceRoot;
    },
  };
};
```

---

# 20. Reutilizar las skills sin copiarlas y degradarlas

Crearía un **registro de skills externas versionadas**.

```text
~/.local/share/archctl/skills/
├── sources/
│   ├── lmammino/
│   │   └── c4-codebase-architecture-skill/
│   │       └── <commit>/
│   ├── bitsmuggler/
│   │   └── c4-skill/
│   │       └── <commit>/
│   ├── cheriftj/
│   │   └── c4-model-skill/
│   │       └── <commit>/
│   └── agents365/
│       ├── drawio-skill/
│       └── plantuml-skill/
├── patches/
├── generated/
└── registry.json
```

Las skills previamente identificadas ya cubren buena parte del proceso:

- `c4-codebase-architecture-skill`: inspección basada en evidencias y generación C4. ([GitHub](https://github.com/lmammino/c4-codebase-architecture-skill/blob/main/README.md?utm_source=chatgpt.com))
- `bitsmuggler/c4-skill`: generación de modelo Structurizr DSL. ([GitHub](https://github.com/bitsmuggler/c4-skill?utm_source=chatgpt.com))
- `cheriftj/c4-model-skill`: diseño, reverse engineering, revisión y actualización con diferentes destinos. ([GitHub](https://github.com/cheriftj/c4-model-skill?utm_source=chatgpt.com))
- `plantuml-skill`: creación y renderizado de UML/PlantUML con opciones locales. ([GitHub](https://github.com/Agents365-ai/plantuml-skill?utm_source=chatgpt.com))
- `drawio-skill`: diagramas editables, transformación desde código e infraestructura y validaciones visuales. ([GitHub](https://github.com/Agents365-ai/drawio-skill?utm_source=chatgpt.com))

---

# 21. Tres formas de reutilizar una skill

## 1. Directa

Cuando ya sea compatible con Agent Skills y no necesite cambios:

```text
upstream skill
      ↓
symlink en opencode/skills
```

Ideal para:

- `plantuml-skill`.
- `drawio-skill`.

## 2. Envuelta

La skill upstream se conserva intacta y añadimos un wrapper que impone nuestros contratos.

```text
architecture-c4-recovery/
├── SKILL.md
├── references/
│   ├── evidence-contract.md
│   ├── architecture-ir-schema.md
│   └── upstream/
│       └── c4-codebase-architecture-skill -> fuente versionada
└── scripts/
```

El wrapper diría:

```markdown
1. Usa el procedimiento de la skill upstream.
2. No inspecciones directamente todo el repositorio.
3. Solicita evidencias mediante archctl.
4. No crees elementos sin evidence IDs.
5. Devuelve Architecture IR, no Mermaid.
```

Ideal para:

- `c4-codebase-architecture-skill`.
- `c4-model-skill`.
- `c4-skill`.

## 3. Parcheada

Solo cuando sea imposible envolverla.

```text
upstream commit
      +
serie de patches
      ↓
skill generada
```

Los parches se guardarían externamente:

```text
skills/patches/
└── bitsmuggler-c4-skill/
    ├── 0001-opencode-compat.patch
    └── 0002-external-output-directory.patch
```

Nunca modificaríamos directamente la copia upstream.

---

# 22. `skillset.lock`

```yaml
schema-version: 1

skills:
  c4-codebase-recovery:
    source: https://github.com/lmammino/c4-codebase-architecture-skill
    commit: 8f3a2d...
    mode: wrapped
    wrapper: architecture-c4-recovery
    license: MIT

  c4-structurizr:
    source: https://github.com/bitsmuggler/c4-skill
    commit: b81291...
    mode: wrapped
    wrapper: architecture-structurizr

  c4-interactive-review:
    source: https://github.com/cheriftj/c4-model-skill
    commit: 73ac1a...
    mode: wrapped

  diagram-plantuml:
    source: https://github.com/Agents365-ai/plantuml-skill
    commit: 92af11...
    mode: direct

  diagram-drawio:
    source: https://github.com/Agents365-ai/drawio-skill
    commit: d01f82...
    mode: direct
```

Comandos:

```bash
archctl skills sync
archctl skills verify
archctl skills build
archctl skills test
archctl skills activate architecture
```

OpenCode carga las skills bajo demanda y permite ubicarlas globalmente, por lo que el skillset generado puede instalarse únicamente en el directorio OpenCode externo. ([OpenCode](https://opencode.ai/docs/skills/))

---

# 23. Redefinición de los subagentes

Con herramientas reales, los subagentes dejan de ser analizadores manuales.

## Antes

```text
code-structure-extractor
    lee miles de ficheros
    intenta entender imports
    construye relaciones
```

## Ahora

```text
code-structure-extractor
    solicita extract.symbols
    solicita resolve.references
    consulta evidencias
    resuelve contradicciones
```

La responsabilidad de cada uno quedaría así:

### `repo-cartographer`

Usa:

- Git.
- `ast-grep outline`.
- Ctags.
- Manifests.
- Gestores de paquetes.

### `static-semantics-specialist`

Usa:

- SCIP.
- LSP.
- `jdeps`.
- dependency-cruiser.
- Doxygen/rustdoc.

### `framework-specialist`

Usa:

- Rule packs de `ast-grep`.
- Semgrep cuando haga falta dataflow.
- Catálogo de patrones por framework.

### `infrastructure-specialist`

Usa:

- Terraform JSON/DOT.
- Helm template.
- Kubernetes JSON.
- Syft.
- Trivy.

### `deep-analysis-specialist`

Usa:

- Joern.
- CodeQL opcional.
- Semgrep avanzado.

### `architecture-synthesizer`

No utiliza código directamente. Solo consume el ledger de evidencias.

### `c4-modeler`

Reutiliza las skills C4 adaptadas y genera una vista desde el Architecture IR.

### `uml-modeler`

Reutiliza PlantUML y genera secuencias, clases o estados desde evidencias y escenarios.

### `visual-reviewer`

Reutiliza las rutinas de draw.io/PlantUML y valida el render.

---

# 24. Proyecto externo configurable

Cada proyecto tendría configuración privada fuera de Git:

```text
~/.config/archctl/projects/<project-id>/project.yaml
```

Ejemplo:

```yaml
repository:
  aliases:
    - payment-platform
    - payments

analysis:
  profiles:
    default: semantic

  exclude:
    - target/**
    - node_modules/**
    - vendor/**
    - generated/**

  languages:
    rust:
      semantic-provider: scip
      fallback:
        - ast-grep
        - ctags

    typescript:
      semantic-provider: scip
      dependency-provider: dependency-cruiser

architecture:
  bounded-contexts:
    payments:
      include:
        - crates/payment/**
        - deploy/payment/**

    fraud:
      include:
        - crates/fraud/**

privacy:
  store-source-snippets: false
  render-using-public-services: false
  data-retention-days: 180
```

Esto permite personalizar el análisis sin añadir:

```text
.archctl.yaml
.opencode/
sgconfig.yml
```

al repositorio.

---

# 25. Compartir resultados sin contaminar el repo

Tendría tres modos.

## Local privado

Todo en XDG. Es el modo predeterminado.

## Exportación puntual

```bash
archctl project export \
  --commit HEAD \
  --output architecture-bundle.tar.zst
```

El bundle incluiría:

- Modelo.
- Evidencias sin código sensible.
- Diagramas.
- Decisiones.
- Versiones de herramientas.
- `skillset.lock`.

## Sidecar compartido

Para equipos, se podría usar un repositorio separado:

```text
payment-platform
payment-platform-architecture
```

O un almacén de objetos/artefactos.

El repositorio de aplicación continuaría limpio, mientras el sidecar compartido contendría únicamente conocimiento arquitectónico curado.

---

# 26. Flujo de ejecución recomendado

```text
archcode
   │
   ▼
resolver proyecto/worktree
   │
   ▼
leer último snapshot
   │
   ▼
git diff desde último commit analizado
   │
   ├── sin cambios relevantes → reutilizar modelo
   │
   └── con cambios
          │
          ▼
     perfil fast
     ├── ast-grep outline
     ├── rules de framework
     ├── Ctags fallback
     └── gestor de paquetes
          │
          ▼
     contradicciones o precisión insuficiente
          │
          ▼
     perfil semantic
     ├── SCIP
     ├── jdeps / dependency-cruiser
     └── herramientas IaC
          │
          ▼
     pregunta compleja
          │
          ▼
     perfil deep
     ├── Joern
     ├── Semgrep
     └── CodeQL opcional
          │
          ▼
     Evidence Ledger
          │
          ▼
     Architecture IR
          │
          ▼
     skills C4/UML adaptadas
          │
          ▼
     render y revisión visual
          │
          ▼
     checkpoint externo
```

---

# 27. MVP que construiría

## Iteración 1: sidecar externo

- Resolver repositorio y worktree.
- Crear estructura XDG.
- Plugin global de OpenCode.
- Wrapper `archcode`.
- Evidence ledger SQLite/JSONL.
- Cero ficheros en el repositorio.

## Iteración 2: extracción rápida

- Git.
- `ast-grep outline`.
- Rule packs externos.
- Universal Ctags.
- Cargo/Maven/Gradle/Go.
- dependency-cruiser.
- Terraform y Helm.

## Iteración 3: modelo arquitectónico

- Architecture IR.
- Evidencias y confianza.
- Modelo por commit.
- Overlay de worktree.
- Skills C4 envueltas.
- Structurizr como proyección principal.

## Iteración 4: precisión semántica

- SCIP.
- `jdeps`.
- Kubernetes runtime.
- Doxygen XML.
- SBOM con Syft.
- Drift incremental.

## Iteración 5: análisis profundo

- Joern.
- Semgrep.
- CodeQL opcional.
- UML dinámico.
- Agente falsificador.

---

# Recomendación final

El diseño debería quedar resumido así:

```text
No creamos parsers.
No creamos indexadores.
No creamos call graphs manualmente.
No copiamos skills para modificarlas.
No escribimos conocimiento en el repositorio.
```

Construimos:

```text
archctl =
    router de capacidades
  + adaptadores de CLIs
  + normalizador de evidencias
  + modelo arquitectónico temporal
  + almacenamiento XDG por proyecto/worktree
  + ensamblador de skills versionadas
  + integración dedicada con OpenCode
```

Y reutilizamos como primera selección:

```text
Sintaxis y patrones      → ast-grep
Inventario universal     → Universal Ctags
Semántica precisa        → SCIP
Dependencias reales      → herramientas nativas del build
Java compilado           → jdeps
JS/TS                    → dependency-cruiser
IaC                      → Terraform + Helm + kubectl
SBOM                     → Syft
Políticas/dataflow       → Semgrep
Análisis profundo        → Joern
C4                       → skills existentes + Structurizr
UML                      → PlantUML skill
Editable/visual          → draw.io skill
```

La mayor aportación propia no sería analizar código, sino **fusionar evidencias heterogéneas, conservar su procedencia y convertirlas en un gemelo arquitectónico temporal recuperable**, completamente desacoplado del repositorio fuente.

---
Powered by [AI Exporter](https://saveai.net)