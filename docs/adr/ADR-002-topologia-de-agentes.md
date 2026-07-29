# ADR-002 — Topología mínima de agentes

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

La diagramación necesita especialización, pero demasiados agentes aumentan delegaciones, latencia y dificultad de depuración.

## Decisión

### Agente primario

- `diagram-architect`

### Subagentes personalizados

- `architecture-evidence`
- `c4-modeler`
- `uml-modeler`
- `diagram-reviewer`

### Reutilización de OpenCode

- `explore` para búsqueda de solo lectura.
- `scout` para documentación y dependencias externas.
- `general` solo para excepciones.

## Flujo

```text
pregunta
  → evidencia
  → actualización del grafo
  → especificación de vista
  → modelo textual
  → render
  → revisión
```

Los especialistas consumen IDs y subgrafos, no copias textuales extensas del repositorio.

## Evolución

`uml-modeler` solo se dividirá en estructural y comportamiento si las evaluaciones muestran fallos repetibles.

## Consecuencias

- Responsabilidades claras.
- Pocos contratos.
- Menor consumo de contexto.
- El grafo funciona como memoria compartida entre subagentes.
