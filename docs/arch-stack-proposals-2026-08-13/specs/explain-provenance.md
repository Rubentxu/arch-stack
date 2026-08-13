# Spec — Explain & Provenance

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Responder por qué existe una entidad/relación/violación sin explicación opaca.

## Scope

Element, relation, claim, violation, projection membership.



## Public surface

`archctl explain <id|selector> --json`; MCP architecture_explain.

## Modelo y semántica

Identity + statement + lineage + source refs + producer/version + confidence derivation + contradictions.



## Seguridad

Source excerpts opt-in y cap.

## Escenarios Given / When / Then

Given A→B, explain lista observation y file:line.
Given conflicto, muestra ambos.
Given no evidence, `unsubstantiated`; nunca inventa.

## Plan de implementación

Existing Evidence primero; Observation/Claim migration transparente.

## Estrategia de pruebas

Lineage graph, cycle guard, missing source, stale evidence.

## Métricas y SLOs de producto/ingeniería

100% machine claims con evidence o explicit reason.



## Dependencias y cross-references

ADR-049; evidence.rs.
