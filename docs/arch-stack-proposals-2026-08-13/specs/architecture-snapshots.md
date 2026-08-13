# Spec — Architecture Snapshots

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Persistir estados comparables ligados a Git sin event sourcing.

## Scope

Identity, creation, ref resolution, retention, schema/extractor digest.



## Public surface

`archctl architecture snapshot create/list/gc`; SnapshotRepository.

## Modelo y semántica

Key = repo identity + SHA + schema + extractor digest; label mutable puede apuntar a immutable snapshot.





## Escenarios Given / When / Then

Given same tuple, idempotent.
Given incompatible schema, diff rebuild/migration.
Given GC, pins remain.

## Plan de implementación

MVP metadata + graph materialization/delta; on-demand from diff; retention.

## Estrategia de pruebas

Idempotency; GC; corruption checksum; large snapshot.

## Métricas y SLOs de producto/ingeniería

Medir incremental size antes de compression complexity.



## Dependencias y cross-references

ADR-050; identity; XDG.
