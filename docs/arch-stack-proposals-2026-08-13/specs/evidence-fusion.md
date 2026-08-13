# Spec — Evidence Fusion

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Agregar múltiples observaciones sin perder procedencia ni confidence opaco.

## Scope

Observation identity, producer, supports/contradicts, aggregation, staleness.



## Public surface

`ObservationRepository` + `ClaimEvaluator`; visible mediante explain/coverage.

## Modelo y semántica

Aggregator v1 simple, determinista, order-independent; independencia y contradicción explícitas.





## Escenarios Given / When / Then

Given AST + manifest support, claim 2 supports.
Given contradiction, ambas quedan.
Given producer version cambia, antigua puede marcarse stale.

## Plan de implementación

Schema dual-write → backfill → aggregator → readers → retire legacy.

## Estrategia de pruebas

Migration; idempotency; commutativity; calibration corpus.

## Métricas y SLOs de producto/ingeniería

Aggregation determinista/commutative; 0 provenance loss.



## Dependencias y cross-references

ADR-049; evidence.rs; migrations.
