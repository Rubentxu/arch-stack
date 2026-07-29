# Informe de verificación final

## Resultado

**PASS — coherencia 97/100 (Gate 1.6).** La base documental es consistente y apta para decisión. Esto valida el plan, no la hipótesis de producto ni una implementación inexistente.

## Lentes ejecutadas

| Lente | Resultado principal |
|---|---|
| Coherencia documental | Corrigió ADRs fantasma, aritmética de tareas, nombres, umbrales y referencias |
| Estándares actuales | Confirmó OpenCode `mcp`, rutas plurales, Mermaid C4 experimental y Structurizr Lite EOL |
| Escepticismo/YAGNI | Reforzó Gate Zero para probar recuperación semántica, no solo carga de skills |
| Seguridad | Incorporó datos-no-instrucciones, confinamiento canónico, rechazo de symlinks y pinning de ejecutables |
| Operabilidad | Añadió smoke probe previo, snapshots fijados, semántica segura del ledger y ventanas realistas |
| Valor de usuario | Exigió resumen ejecutivo, siguiente acción barata y valor residual si la hipótesis falla |

## Problemas encontrados y resueltos

- La propuesta listaba doce ADRs, pero existían ocho: ahora usa únicamente ADR-0001…ADR-0008.
- El total de tareas era incorrecto: ahora son **38 = 4 + 12 + 16 + 6 diferidas**.
- Se mezclaban `archcode`/`archctl`, Go/TypeScript y ADRs de tres/cuatro dígitos: normalizado a `archctl`, TypeScript M0–M2 y ADR-000N.
- La identidad dependía universalmente de Git: ahora `SourceIdentity` discrimina `git | directory` y usa `projectId` portable.
- Los umbrales permitían afirmaciones sin evidencia en ciertos repos: ahora alta confianza sin evidencia siempre falla.
- Gate Zero solo comprobaba compatibilidad: ahora ejecuta una micro-recuperación contra gold set y valida IR/proyección/render.
- Las rutas OpenCode eran singulares: corregidas a directorios plurales; las claves JSON permanecen singulares.
- `Structurizr local` se describía imprecisamente: ahora se distingue visor local de validación/export headless fijada por versión.

## Bloat rechazado

No se añadieron nuevas plataformas de observabilidad, compliance empresarial, almacenamiento temporal, firma de bundles, políticas JIT ni cinco ADRs especulativos. Las defensas mínimas se integraron en ADR-0004/0008, especificación y criterios de tareas. Las capacidades avanzadas quedan condicionadas a evidencia de M1/M2.

## Riesgos residuales

1. La ingeniería inversa fiable sigue sin validarse: Gate Zero y M1 deben intentar refutarla.
2. El método definitivo de calibración de confianza sigue abierto; `heuristic-v1` debe declararse como tal.
3. Los hooks y permisos de OpenCode requieren probe runtime además del schema-contract.
4. El IR es un hub de acoplamiento: cualquier ampliación debe tener consumidor real y migración versionada.
5. Los ocho ADRs siguen `Proposed` y requieren aceptación humana antes de aplicar.

## Evidencia de cierre

- Tareas: 38, TypeScript M0–M2.
- Milestones: M0 3–4 días; M1 2–4 semanas; M2 4–6 semanas.
- ADRs: 8, todos Proposed.
- Gate de coherencia final: **1.6, 97/100, PASS**.
