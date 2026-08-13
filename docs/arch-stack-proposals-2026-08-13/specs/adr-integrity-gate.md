# Spec — ADR Integrity Gate

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Garantizar identidad única y referencias consistentes en docs/adr.

## Scope

Filename/H1 ID, duplicates, status, links, index, supersedes/complements.



## Public surface

`scripts/check-adr-integrity [--json]`; exit 0 valid, 2 invalid.

## Modelo y semántica

ID = clave única. Colisión histórica se resuelve con mapping/tombstone, nunca silenciosamente.





## Escenarios Given / When / Then

Given duplicate 040, reporta ambos.
Given link missing, identifica source.
Given ADR nuevo no indexado, falla/warn según policy.

## Plan de implementación

Parser Markdown mínimo; fixtures; resolver 040/041; activar PR CI.

## Estrategia de pruebas

Fixtures valid/duplicate/broken/status; snapshot output.

## Métricas y SLOs de producto/ingeniería

<2s; 0 falsos positivos tras remediation.



## Dependencias y cross-references

ADR-047; docs/adr/README.md.

## Ejemplos

Mantener los IDs más arraigados en releases y reasignar los menos referenciados mediante PR explícito.
