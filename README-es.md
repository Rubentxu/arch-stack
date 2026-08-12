# arch-stack

**Diagramas de arquitectura desde código — C4, UML, secuencia y más.**

`arch-stack` es un CLI + workbench local que hace reverse-engineering de tu repositorio en un grafo de conocimiento arquitectónico y lo proyecta como diagramas interactivos C4 y UML. Se ejecuta completamente en tu máquina; nada sale de tu entorno por defecto.

[![Último Release](https://img.shields.io/github/v/release/Rubentxu/arch-stack?logo=github&label=latest)](https://github.com/Rubentxu/arch-stack/releases/latest)
[![Licencia: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/Rubentxu/arch-stack/release.yml?logo=github)](https://github.com/Rubentxu/arch-stack/actions)
[![rust-version](https://img.shields.io/badge/rust-1.91%2B-blue.svg?logo=rust)](archctl/Cargo.toml)

---

## De un Vistazo

```
$ archctl doctor                              # verificar entorno
$ archctl ide install opencode                 # conectar con OpenCode
$ /diagram c4 context                          # C4 Contexto de Sistema
$ /diagram c4 container                        # C4 Contenedor
$ /diagram class order-domain                  # Diagrama de Clases UML
$ /diagram sequence "crear pedido"             # Secuencia UML
$ archctl view                                # abrir el workbench
```

---

## Características

| Capacidad | Descripción |
|---|---|
| **Diagramas C4** | Context, Container, Component desde extracción de código |
| **Diagramas UML** | Class, Sequence, State, Use Case |
| **Multi-lenguaje** | Rust, Go, Python, Java, Kotlin (call-graph) |
| **Local-first** | Todos los datos en `~/.local/share/archctl/` (XDG) |
| **Basado en evidencia** | Cada nodo y arista enlaza a `file:line` de procedencia |
| **Workbench embebido** | `archctl view` sirve archview desde el binario (sin instalación separada) |
| **Integración IDE** | OpenCode, ZCode, Claude Code, Codex vía `archctl ide` |
| **Reproducible** | Proyecciones deterministas desde la misma base de código |

---

## Arquitectura

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tu IDE (OpenCode / Claude Code)               │
│                                                                  │
│   /diagram c4 container                                          │
│        │                                                         │
│        ▼                                                         │
│   diagram-architect  (agente orquestador)                         │
│   ├── c4-modeler        → skill c4-from-graph                  │
│   ├── uml-modeler        → skills class/sequence/usecase         │
│   ├── architecture-evidence → skill architecture-discovery        │
│   └── diagram-reviewer    → skill diagram-review                 │
└────────────────────────────┬────────────────────────────────────┘
                             │ archctl code / archctl diagram
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                       archctl (CLI Rust)                        │
│                                                                  │
│   code call-graph   → extraer hechos del código fuente          │
│   code class-diagram                                             │
│   code sequence                                                   │
│   code state-machine                                             │
│   diagram export   → proyectar vistas (C4 / UML)               │
│   view             → servir workbench archview embebido          │
│                                                                  │
│   LadybugDB (grafo) ← persiste en ~/.local/share/archctl/      │
└─────────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    archview (Workbench Embebido)                 │
│   Renderizador G6 canvas · pan/zoom · inspector de evidencia     │
│   (servido por archctl view — sin instalación separada)           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Instalación

### Opción 1 — Homebrew (recomendada)

```bash
brew install Rubentxu/arch-stack/archctl
```

### Opción 2 — Binario precompilado

```bash
# Descargar el último release para Linux x86_64
curl -L https://github.com/Rubentxu/arch-stack/releases/latest/download/archctl-x86_64-unknown-linux-gnu.tar.gz \
  -o /tmp/archctl.tar.gz

# Extraer a un directorio en tu PATH
tar -xzf /tmp/archctl.tar.gz -C ~/.local/bin/

# Verificar
archctl --version
```

Para otras plataformas, consulta [Releases](https://github.com/Rubentxu/arch-stack/releases/latest).

### Opción 3 — Desde código fuente

```bash
git clone https://github.com/Rubentxu/arch-stack.git
cd arch-stack/archctl
cargo build --release
./target/release/archctl --version
```

### Opción 4 — Con self-update (una vez tengas archctl)

```bash
archctl self install          # instalar última estable
archctl self update           # actualizar a la última
archctl self update --check  # verificar sin instalar
```

---

## Integración con IDE

Después de instalar `archctl`, conéctalo a tu IDE:

```bash
# OpenCode
archctl ide install opencode

# Claude Code
archctl ide install claude-code

# ZCode
archctl ide install zcode

# Codex
archctl ide install codex

# Ver qué está instalado
archctl ide list --installed

# Diagnosticar un IDE
archctl ide doctor opencode
```

---

## Referencia de Comandos CLI

### Diagnóstico y configuración

```bash
archctl doctor              # verificar entorno y dependencias
archctl project resolve     # detectar identidad del repositorio actual
```

### Extracción de código

```bash
archctl code call-graph        # extraer call graph (todos los lenguajes soportados)
archctl code class-diagram     # extraer definiciones de clase / struct
archctl code sequence          # extraer secuencias / call chains
archctl code state-machine     # extraer candidatos a state machine
```

### Proyección de diagramas

```bash
archctl diagram export c4-context              # Vista C4 System Context
archctl diagram export c4-container            # Vista C4 Container
archctl diagram export c4-component           # Vista C4 Component
archctl diagram project --view sequence:*      # Vista UML Sequence
archctl diagram project --view class:*         # Vista UML Class
archctl diagram project --view usecase:*      # Vista UML Use Case
archctl diagram project --view state:*        # Vista UML State Machine
```

### Workbench

```bash
archctl view                # iniciar workbench embebido (puerto aleatorio)
archctl view --port 9000   # iniciar en un puerto fijo
```

### Introspección del grafo

```bash
archctl graph list-elements                  # listar todos los nodos
archctl graph list-relations                 # listar todas las aristas
archctl evidence list --element <id>         # evidencia de un nodo
archctl evidence list --relation <rel>       # evidencia de una arista
```

### Gestión del ciclo de vida

```bash
archctl self install [version]   # instalar una versión
archctl self list                 # listar versiones instaladas
archctl self use <version>       # cambiar versión activa
archctl self update              # actualizar a la última
archctl self uninstall           # eliminar versión actual
```

---

## Comandos de OpenCode

Cuando está integrado con OpenCode, el punto de entrada es `/diagram`:

| Comando | Descripción |
|---|---|
| `/diagram c4 context` | Diagrama C4 de Contexto de Sistema |
| `/diagram c4 container` | Diagrama C4 de Contenedor |
| `/diagram c4 container <scope>` | C4 Container con ámbito en un módulo |
| `/diagram c4 component <module>` | Diagrama C4 de Componente |
| `/diagram class <module>` | Diagrama de Clases UML |
| `/diagram sequence <function>` | Secuencia UML para un call chain |
| `/diagram usecase <name>` | Diagrama de Casos de Uso UML |
| `/diagram state <entity>` | Máquina de Estados UML |
| `/diagram explain <element-id>` | Mostrar evidencia de un nodo |
| `/diagram evidence <relation-id>` | Mostrar evidencia de una relación |
| `/diagram update` | Refrescar diagramas tras cambios en código |
| `/diagram review` | Validar calidad del diagrama contra el grafo |

---

## Dependencias Externas (opcionales)

Algunos comandos requieren herramientas externas. `archctl` informa claramente si faltan.

| Herramienta | Necesaria para | Instalación |
|---|---|---|
| Java | Renderizado PlantUML | `sudo apt install default-jre` |
| PlantUML | Exportación diagramas PlantUML | [plantuml.com/download](https://plantuml.com/download) |
| ast-grep | Extracción Rust / TypeScript | `cargo install ast-grep` |
| tree-sitter-graph | Extracción avanzada | `cargo install tree-sitter-graph` |

---

## Documentación

| Documento | Descripción |
|---|---|
| [`docs/README.md`](docs/README.md) | Índice completo de documentación |
| [`docs/STATE.md`](docs/STATE.md) | Estado actual, capacidades entregadas |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Horizontes H0–H3 + historial de milestones |
| [`docs/adr/`](docs/adr/) | Registros de Decisiones de Arquitectura (041 ADRs) |
| [`docs/specs/`](docs/specs/) | Especificaciones de vistas y contratos |
| [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md) | Modelo de datos del grafo canónico |
| [`MANUAL.md`](MANUAL.md) | Guía completa de uso |

---

## Persistencia de Datos

Todos los datos viven fuera de tu repositorio, siguiendo [ADR-004](docs/adr/ADR-004-persistencia-externa-xdg.md):

```
~/.local/share/archctl/
└── projects/<hash>/
    └── architecture.lbdb      # el grafo canónico

~/.config/archctl/
├── config.toml                 # configuración global
└── plugins/                   # plugins instalados
```

Tu código fuente **nunca es modificado**. `archctl` solo lee.

---

## Licencia

`arch-stack` se distribuye bajo **MIT OR Apache-2.0**. Ver [`LICENSE`](LICENSE) para más detalles.
