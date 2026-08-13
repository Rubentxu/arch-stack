# Spec — Ladybug Compatibility Doctor

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Detectar incompatibilidad nativa antes de compile/link/open DB failures.

## Scope

Build metadata, runtime probe, schema/migration compatibility.



## Public surface

`archctl doctor --scope storage --json`.

## Modelo y semántica

Tuple: archctl, lbug crate, native version/source digest, target, compiler, stdlib, schema.

## Errores y comportamiento degradado

Unknown no equivale a compatible en release.



## Escenarios Given / When / Then

Given tuple compatible, ok.
Given mismatch, critical.
Given unknown, warning + remediation; release treats unknown as failure.

## Plan de implementación

Pin native; expose metadata; probe; release smoke.

## Estrategia de pruebas

Compatibility table + actual runners + CRUD/migration smoke.

## Métricas y SLOs de producto/ingeniería

Cada Tier-1 release conserva evidencia storage probe.



## Dependencias y cross-references

ADR-048; release; DATA-MODEL-LADYBUGDB.
