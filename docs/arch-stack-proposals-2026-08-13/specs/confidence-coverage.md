# Spec — Confidence & Coverage

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Representar incertidumbre, cobertura y contradicción como parte del producto.

## Scope

Claim confidence, subsystem coverage, unresolved calls, unsupported language, stale evidence.



## Public surface

`archctl architecture coverage --json`; overlay contract.

## Modelo y semántica

Unknown ≠ false; coverage siempre declara denominator/exclusions.





## Escenarios Given / When / Then

Given 100 funcs/70 resolved, denominator visible.
Given unsupported language, status unsupported, no 0%.
Given conflict, count visible.

## Plan de implementación

Define metrics → instrument extractors → report → archview overlay.

## Estrategia de pruebas

Known corpus; calibration regression.

## Métricas y SLOs de producto/ingeniería

Denominator no cambia silenciosamente.



## Dependencias y cross-references

ADR-049; capability registry; Explain.
