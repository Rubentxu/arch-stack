# Spec — Capability Registry

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Unificar feature/language/maturity/requisitos y eliminar drift.

## Scope

Extractors, projections, render outputs, views, MCP y plugin extensions.



## Public surface

`archctl capabilities --json`; registry tipado; schema v1.

## Modelo y semántica

Key estable + providers + maturity + deterministic + requirements + output schema.





## Escenarios Given / When / Then

Given Kotlin provider, registry lo lista.
Given provider sin entry, alignment test falla.
Given dependency ausente, status unavailable con reason.

## Plan de implementación

Inventory → registry → alignment → CLI/MCP → generated docs.

## Estrategia de pruebas

Golden JSON; provider alignment; generated docs diff.

## Métricas y SLOs de producto/ingeniería

0 matrices manuales duplicadas tras rollout.



## Dependencias y cross-references

ADR-045; manifests; specs index.

## Ejemplos

Ver `../examples/capability-registry.yaml`.
