# Spec — Moldable Workbench Navigation

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Coordinar vistas según entidad/tarea mediante identidad estable y semantic zoom.

## Scope

Cross-view identity, zoom, breadcrumbs, action palette, lens recommendation, history.



## Public surface

`NavigationTarget { canonicalId, preferredLens?, focus? }`; renderer IDs nunca canónicos.

## Modelo y semántica

Levels combinan jerarquía C4 y code hierarchy mediante relaciones explícitas.





## Escenarios Given / When / Then

Given container double-click, component lens + breadcrumb.
Given function Up, owning module/component si evidence.
Given auto lens, rationale visible y reversible.

## Plan de implementación

Identity bridge → action palette → semantic zoom → recommendation.

## Estrategia de pruebas

UI integration; back/forward; keyboard; 10k perf.

## Métricas y SLOs de producto/ingeniería

<100ms navigation after data loaded; no full reload lens switch.



## Dependencias y cross-references

ADR-056; archview existing views; capability registry.
