# Spec — License Coherence

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Eliminar contradicción README/Cargo/files legales.

## Scope

Root licenses, Cargo license, README, release artifacts.

## Non-goals

No asesoramiento jurídico.

## Public surface

`check-license-coherence` compara expresión SPDX y archivos requeridos.

## Modelo y semántica

El gate valida coherencia textual/estructural, no compatibilidad jurídica.





## Escenarios Given / When / Then

Given Cargo MIT y README MIT OR Apache, fail.
Given dual metadata + ambos license files, pass.

## Plan de implementación

Maintainers eligen licencia efectiva; files + metadata + gate.

## Estrategia de pruebas

Parser TOML + fixture root.

## Métricas y SLOs de producto/ingeniería

0 diferencias SPDX.



## Dependencias y cross-references

README, README-es, Cargo.toml.
