# ADR-054 — Políticas y fitness functions sobre el grafo canónico

> **Estado:** Aceptado — 2026-08-13 (shipped as P2-05 Policy Metamodel v1.54.0 + P2-06 Fitness Evaluator v1.55.0; 6 closed rules per ADR-054 + SARIF 2.1.0 + JUnit XML output formats; `archctl architecture policy check [--policy <file>] [--fail-on ...]`)
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

El grafo informa dependencias reales, pero sin políticas no previene regresiones.
Existe policy engine cognitivo que puede reaprovecharse si el contrato permanece
determinista.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Modelo mínimo de reglas: `forbid_dependency`, `require_dependency`, `forbid_cycle`,
`max_fanout`, `evidence_required`, `confidence_min`, selectors, severity y waivers.
Outputs JSON/SARIF/JUnit.

## Superficie propuesta

```toml
[[rules]]
id="HEX-001"
type="forbid_dependency"
from="module:domain/**"
to="crate:reqwest"
severity="error"
```

## Rationale y beneficios

Architecture-as-code, CI e Intent vs Reality. Arch Stack puede dogfood su propia
hexagonal.

## Costes y consecuencias negativas

Falsos positivos y riesgo de DSL gigante.





## Estrategia de migración

TOML/YAML con rule set cerrado; evaluator puro; warn primero, enforce después.

## Verificación y criterios de aceptación

- determinista;
- violation con rule+IDs+evidence;
- SARIF a source;
- waiver expira;
- self-policy.

## Alternativas consideradas

A) OPA/Rego: runtime/DSL extra.
B) Cedar: otro dominio.
C) hardcoded Rust tests: no consumible.

## Referencias internas

cognitive/policy, selectors, ADR-038/047.

## Changelog

- 2026-08-13 | proposed | ADR-054 creado a partir de la auditoría de consolidación.
