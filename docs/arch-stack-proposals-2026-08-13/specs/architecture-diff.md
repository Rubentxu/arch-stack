# Spec — Architecture Diff

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Comparar arquitectura semántica entre dos Git refs/snapshots con provenance.

## Scope

Elements, relations, semantic props, confidence, policy, cycles, boundaries, unresolved.



## Public surface

`archctl architecture diff A..B`; schema `architecture-diff-report/1`.

## Modelo y semántica

Tipos: added/removed/changed/confidence_changed/policy_regression/improvement/cycle changes.





## Escenarios Given / When / Then

Given relation nueva, report + evidence.
Given visual move, semantic diff empty.
Given extractor version differs, compatibility metadata lo marca.

## Plan de implementación

Refactor cognitive/delta → snapshot provider → schema → CLI → DriftView.

## Estrategia de pruebas

Golden graph; deterministic ordering; schema compatibility; benchmark.

## Métricas y SLOs de producto/ingeniería

p95 <2s cached <10k nodes; hash estable.



## Dependencias y cross-references

ADR-050/053; cognitive/delta; DriftView.

## Ejemplos

Ver `../examples/architecture-diff-output.json`.
