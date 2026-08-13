# Spec — Sanitized `.archbundle`

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Compartir conocimiento offline sin compartir repo.

## Scope

Manifest, graph slice, evidence metadata, policies, capabilities, optional diff/snapshot, redaction, checksums.



## Public surface

`archctl bundle export --profile strict`; inspect; archview read-only.

## Modelo y semántica

Container secundario; manifest/schema contrato. Deny-by-default para source/env/absolute paths/credentials.



## Seguridad

Allowlist > blacklist; unknown metadata excluded until classified safe.

## Escenarios Given / When / Then

Given secret fixture, no source bytes.
Given absolute path, relative/pseudonymized.
Given tamper, checksum fails.

## Plan de implementación

Schema → strict sanitizer → scanner → archview open → profiles later.

## Estrategia de pruebas

Secret corpus; path privacy; deterministic bundle; tamper.

## Métricas y SLOs de producto/ingeniería

0 known secret patterns strict; deterministic manifest hash excluding timestamps.



## Dependencias y cross-references

ADR-055; projection/evidence.
