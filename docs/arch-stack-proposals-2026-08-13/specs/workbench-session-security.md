# Spec — Workbench Session Security

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Proteger side-effect endpoints loopback sin auth de usuario.

## Scope

Secret lifecycle, bootstrap, auth, Origin/Host, logging.



## Public surface

`ViewSession { token }`; side effect API requiere secret header.

## Modelo y semántica

Secret per process; nunca workspace state; health puede público.



## Seguridad

No query token que pueda filtrar Referer; usar fragment/bootstrap/header.

## Escenarios Given / When / Then

Given missing token POST open-editor →403.
Given restart old token invalid.
Given static asset →works.
Given non-loopback Host →reject.

## Plan de implementación

Inject session → guard endpoints → bootstrap frontend → tests.

## Estrategia de pruebas

Handler tests + browser integration; no persistent token logs.

## Métricas y SLOs de producto/ingeniería

0 side-effect endpoints sin guard.



## Dependencias y cross-references

ADR-051; view.rs.
