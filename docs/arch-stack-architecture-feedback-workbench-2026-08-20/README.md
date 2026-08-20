# Arch Stack — Architecture Feedback Workbench

**Blueprint consolidado de implementación — 20 de agosto de 2026**

Este paquete consolida y refina las decisiones surgidas durante la evolución de `arch-stack`: Architecture Intelligence, Moldable Development/GToolkit, Graphify, ActiveGraph, CodeSpeak/Twip, visual thinking, UAT, agentes, intent-vs-reality, knowledge graphs y visualización de arquitectura.

No es un PRD aspiracional. Está organizado para convertirse en trabajo ejecutable: visión y valor humano, arquitectura técnica y stack explícito, frontera determinista/probabilística/humana, pipeline incremental de conocimiento, visual grammar, ADRs, specs, roadmap, backlog, UAT y matriz de trazabilidad.

## North Star

> Reducir drásticamente el esfuerzo necesario para que un humano forme, verifique, mantenga y corrija su modelo mental de un sistema software complejo.

## Fórmula de producto

```text
CODE / DOCS / RUNTIME / INTENT
             │
             ▼
      VERIFIABLE GRAPH
             │
             ▼
        ANALYSIS
             │
             ▼
     VISUAL THINKING
             │
             ▼
          HUMAN
             │
          FEEDBACK
             │
             ▼
          AGENTS
             │
             └──────────────↺
```

**El grafo es el sustrato. La visualización es el lenguaje. El feedback es el mecanismo de corrección. La comprensión humana es el producto.**

## Baseline de integración

El blueprint presupone el estado observado de `arch-stack` alrededor de `v1.80.0` (2026-08-20): Rust + LadybugDB + Tree-sitter/ast-grep + petgraph, Archview en SolidJS + G6 5 + ELK, workspace state, source drawer, explain, semantic zoom, culling/LOD, navigation, sidebar tabs y ADR-019 perf-ci-gate.

No se propone un rewrite. Se propone una evolución incremental.
