# ADR-003 — Reutilización y adaptación de skills

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

Existen skills útiles para C4, Structurizr, PlantUML, Mermaid y draw.io. Reimplementarlas aumentaría el mantenimiento.

## Decisión

### Directas

Se instalan sin cambios cuando encajan:

- PlantUML.
- Mermaid.
- draw.io.

### Envueltas

La skill upstream permanece intacta. Una wrapper:

- define entrada y salida;
- obliga a utilizar IDs y evidencias de `archctl`;
- redirige artefactos a XDG;
- adapta referencias a OpenCode;
- persiste la especificación de vista.

Aplicable a:

- `c4-codebase-architecture-skill`;
- `c4-skill`;
- `c4-model-skill`.

### Parcheadas

Solo cuando no sea posible envolver. Los parches se aplican sobre un commit fijado.

## Registro

`skills.lock.yaml` conserva:

- repositorio;
- commit o versión;
- licencia;
- hash;
- modo de integración;
- wrapper;
- esquema de entrada y salida.

## Prohibiciones

- Copiar una skill sin procedencia.
- Editar upstream.
- Seguir `main` sin fijar versión.
- Permitir autoedición de skills estables.
- Hacer que una skill acceda directamente a LadybugDB.

## Consecuencias

- Actualizaciones controladas.
- Menos código propio.
- Contratos estables con `archctl`.
- Posibilidad de contribuir mejoras a upstream.
