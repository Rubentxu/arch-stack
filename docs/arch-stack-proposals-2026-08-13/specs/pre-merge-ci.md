# Spec — Pre-merge CI

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Impedir que fallos reproducibles entren en main sin benchmarks largos en cada PR.

## Scope

pull_request, post-merge, branch protection, release dependency.



## Public surface

Check names estables; fast workflow y heavy evidence workflow.

## Modelo y semántica

Fast deterministic required; perf/real corpus post-merge o opt-in.





## Escenarios Given / When / Then

Given compile fail, PR red.
Given ADR duplicate, PR falla rápido.
Given perf-only regression, post-merge signal visible.

## Plan de implementación

Add trigger/jobs; cache; observar un ciclo; required; release gate.

## Estrategia de pruebas

Workflow scripts + branch protection verification.

## Métricas y SLOs de producto/ingeniería

Fast median target <10 min; docs gates <1 min.



## Dependencias y cross-references

ADR-047; CI scripts.
