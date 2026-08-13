# Spec — Plugin Security Hardening

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Cerrar bugs de path/staging e introducir supply-chain boundary segura.

## Scope

Spec parsing, install root, download verify, tar extract, activation, trust.



## Public surface

`archctl plugin inspect/install/verify`; identity value objects; manifest v2.

## Modelo y semántica

Resolve → download → verify → safe extract → manifest validate → atomic activate.

## Errores y comportamiento degradado

Fallo limpia staging y no cambia current.

## Seguridad

No seguir symlink escape; limitar expanded size/files; rechazar devices/FIFO.

## Escenarios Given / When / Then

SCN-PLG-01: `../../evil` se rechaza antes de I/O.
SCN-PLG-02: remote sin sha256 falla cerrado.
SCN-PLG-03: tar `../../outside` no escapa staging.
SCN-PLG-04: first install crea root y funciona.

## Plan de implementación

P0 bugs/tests; P1 manifest; P3 signatures/capability enforcement.

## Estrategia de pruebas

Property tests IDs; malicious tar; mocked HTTP; first-install E2E.

## Métricas y SLOs de producto/ingeniería

100% remote installs verificadas; 0 writes fuera de root.

## Rollout y rollback

Legacy readable como `legacy-unverified`.

## Dependencias y cross-references

ADR-046, ADR-004, plugin modules.
