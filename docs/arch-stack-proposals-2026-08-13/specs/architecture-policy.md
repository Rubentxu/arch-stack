# Spec — Architecture Policy & Fitness Functions

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Expresar restricciones mínimas y evaluarlas sobre el grafo.

## Scope

Selectors, dependency/cycle/evidence/confidence, severity, waivers, JSON/SARIF/JUnit.



## Public surface

`archctl architecture check --policy <file> --format sarif`.

## Modelo y semántica

Rule IDs estables; violation referencia graph IDs/evidence/source; waiver con reason/expiry.





## Escenarios Given / When / Then

Given forbidden edge, error con path.
Given waiver expired, violation vuelve.
Given selector no match, warning.

## Plan de implementación

Pure evaluator → 6 rule types → outputs → self-dogfood → CI.

## Estrategia de pruebas

Rule fixtures; selector properties; SARIF schema; self-policy.

## Métricas y SLOs de producto/ingeniería

<1s 10k nodes base rules; deterministic.



## Dependencias y cross-references

ADR-054; cognitive/policy; ADR-043.

## Ejemplos

Ver `../examples/architecture-policy.toml`.
