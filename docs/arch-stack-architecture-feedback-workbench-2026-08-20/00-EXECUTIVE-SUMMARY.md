# Executive Summary

## Problema

Las herramientas de arquitectura suelen fallar en uno de estos extremos: especificaciones desconectadas de la realidad, grafos ricos pero cognitivamente inabarcables, diagramas bonitos sin trazabilidad, informes de agentes en Markdown que el humano no consume o una falsa *single source of truth* que borra contradicciones valiosas.

## Propuesta

Arch Stack evoluciona de *code knowledge graph workbench* a **Architecture Feedback Workbench**.

Mantiene separados y reconciliables:

- **Intent** — lo que queremos que sea el sistema.
- **Static reality** — lo que el código demuestra.
- **Runtime reality** — lo que realmente ocurre.
- **Validation** — tests, UAT y policies.
- **Human feedback** — aceptación, rechazo y corrección.
- **Agent reasoning** — hipótesis y propuestas, nunca hechos silenciosos.

```text
Intent ─┐
Static ─┼─> Reconciliation ─> Visual understanding ─> Human feedback
Runtime ┤
Tests ──┘
```

## Capacidades estrella

Smart System Overview, Moldable Inspector, coordinated views, DSM, System Map, Intent Map, Reconciliation Matrix, Why/Evidence Path, Semantic Review, Architecture Stories, Architecture Causality Graph, graph-driven UAT impact, Agent↔Visual protocol, Time Machine y What-if posterior.

## Principio operativo

> El agente puede interpretar y proponer. El núcleo determinista observa, calcula y reconcilia. El humano decide donde existe ambigüedad o intención.
