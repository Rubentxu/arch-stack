# ADR-001 — OpenCode primero; `archctl` como sidecar

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

OpenCode ya ofrece agentes, subagentes, permisos, commands, custom tools y Agent Skills. La solución necesita razonamiento y delegación, pero también persistencia independiente de las sesiones.

## Decisión

OpenCode será el plano cognitivo y de interacción.

`archctl` será una CLI auxiliar responsable de:

- identidad de proyecto y worktree;
- directorios XDG;
- adaptadores CLI;
- normalización de evidencias;
- acceso a LadybugDB;
- consultas y caminos;
- snapshots;
- especificaciones de vistas;
- modelos y artefactos;
- recuperación.

`archctl` no:

- invoca LLM;
- selecciona el tipo de diagrama;
- decide el significado arquitectónico;
- contiene prompts;
- reemplaza las skills;
- es un portal;
- modifica el código de la aplicación.

## Integración

Custom tools globales y delgadas:

```text
arch_project
arch_run
arch_scan
arch_graph
arch_snapshot
arch_scenario
arch_diagram
arch_artifact
```

Cada tool invoca `archctl` y devuelve JSON validado.

## Consecuencias

- Razonamiento visible en agentes y skills.
- Persistencia testeable sin LLM.
- Recuperación independiente de la conversación.
- Menor acoplamiento con OpenCode.
