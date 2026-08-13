# Spec — Task Context Compiler

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Compilar contexto arquitectónico pequeño y verificable para coding agents.

## Scope

Task query, seeds, graph expansion, rank, evidence/policy/ADR, budget/truncation.



## Public surface

`archctl context compile`; MCP; schema `task-context/1`.

## Modelo y semántica

normalize → lexical/symbol seeds → graph expansion → impact → enrichment → scoring → packing → trace.



## Seguridad

Metadata/evidence locations default; source opt-in/capped.

## Escenarios Given / When / Then

Given CheckoutService, exact seed.
Given 12k budget, estimator no excede.
Given disconnected terms, unknowns.
Given same inputs deterministic, hash equal.

## Plan de implementación

Reuse cognitive/context → scorer ports → GraphReadModel → CLI → MCP → preview.

## Estrategia de pruebas

Golden tasks; budget properties; relevance corpus; no-source leakage.

## Métricas y SLOs de producto/ingeniería

Context reduction >80% vs repo text manteniendo known change surface en corpus.



## Dependencias y cross-references

ADR-052; ImpactView; MCP.

## Ejemplos

Ver `../examples/task-context-output.json`.
