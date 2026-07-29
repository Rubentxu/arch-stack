# ADR-007 — Diagramas como proyecciones del grafo

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

C4 y UML tienen metamodelos diferentes. Mermaid, PlantUML, Structurizr y draw.io son formatos de representación, no fuentes de verdad completas.

## Decisión

### Grafo canónico

LadybugDB conserva identidades, relaciones, escenarios y evidencias.

### C4

Structurizr DSL es la salida preferente para:

- Landscape.
- Context.
- Container.
- Component.
- Dynamic.
- Deployment.

### UML

PlantUML es la salida preferente para:

- casos de uso;
- clases;
- secuencia;
- actividad;
- estado;
- componentes;
- despliegue.

### Mermaid

Salida ligera para Markdown y previews.

### draw.io

Salida editable y de presentación.

## Vista persistida

Cada diagrama se representa en el grafo mediante:

```text
view.diagram
view.member
view.edge
view.group
```

Los miembros referencian elementos canónicos. Las aristas de vista referencian relaciones canónicas.

## Regla de calidad

Un diagrama debe ser:

1. renderizable;
2. apropiado para su notación;
3. sustentado;
4. legible;
5. acotado;
6. reproducible desde su especificación de vista.

## Consecuencias

- Un cambio visual no altera el modelo.
- C4 mantiene consistencia entre vistas.
- UML conserva expresividad.
- El mismo escenario puede producir UML Sequence y C4 Dynamic.
