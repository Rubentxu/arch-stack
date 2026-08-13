# Spec — Architecture Consolidation Program

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Reducir acoplamiento estructural sin reescribir ni romper superficies públicas.

## Scope

Composition root, usecases, ports, module boundaries, dependency rules y criterio de crates.

## Non-goals

No reescribir archctl; no DI framework; no microservices.

## Public surface

`Runtime/AppServices`, usecase inputs/outputs y ports semánticos; CLI externa estable.

## Modelo y semántica

Bounded capability como unidad. Movimiento estructural debe demostrar equivalencia.





## Escenarios Given / When / Then

### SCN-AC-01
Given golden output existente, when comando se migra a usecase, then exit/JSON igual.

### SCN-AC-02
Given application module, when importa lbug/reqwest/tiny_http, then gate falla.

## Plan de implementación

Baseline → composition root → migrate command → repositories → gates → repeat.

## Estrategia de pruebas

Unit con fakes; golden CLI; dependency tests; compile tests.

## Métricas y SLOs de producto/ingeniería

0 infra imports en domain/application; 0 Cypher en usecases; >80% core usecases sin I/O.



## Dependencias y cross-references

ADR-043, ADR-044, ADR-047.
