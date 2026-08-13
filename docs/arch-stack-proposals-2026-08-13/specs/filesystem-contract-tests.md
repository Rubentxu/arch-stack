# Spec — Filesystem Adapter Contract Tests

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Asegurar LSP entre SystemFilesystem y MemoryFilesystem.

## Scope

read/write/exists/walk/canonicalize/path containment/errors.



## Public surface

`FilesystemContractSuite` reusable con factory/root.

## Modelo y semántica

Contrato explícito para existing/nonexisting/symlink/traversal y ordering.





## Escenarios Given / When / Then

Given nonexistent nested path, ambos adapters mismo resultado/error class.
Given symlink escape, ambos rechazan.
Given walk, ordering determinista.

## Plan de implementación

Formalizar semantics → ejecutar suite → corregir adapters.

## Estrategia de pruebas

Tempdir SystemFS + MemoryFS; cfg platform; property paths.

## Métricas y SLOs de producto/ingeniería

100% scenarios pasan en adapters.



## Dependencias y cross-references

filesystem-port spec; ADR-043.
