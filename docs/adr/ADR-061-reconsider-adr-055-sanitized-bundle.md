# ADR-061 — Reconsiderar ADR-055: Sanitized Architecture Bundle como Feature de Seguridad

> **Estado:** Aceptado — 2026-08-18
> **Supersedes**: ADR-055 (reconsideración)
> **Baseline**: `main@ed5b6cb` (v1.59.0)
> **Ámbito**: Desbloqueo de Item 28 Wave 3 (strict ArchBundle)
> **Propietario de decisión**: maintainers de Arch Stack

---

## Resumen ejecutivo

ADR-055 (Sanitized Architecture Bundle) fue diferido el 2026-08-18 con reopen triggers diseñados para un escenario de distribución pública. Sin embargo, los triggers son demasiado restrictivos para el caso de uso real de **compartición interna de arquitectura con terceros (auditores, consultores, stakeholders)**. Este ADR propone reabrir ADR-055 con triggers adaptados que incluyen casos de uso internos, y define Item 28 (strict ArchBundle) como feature de seguridad legítima.

---

## Problema

### ADR-055 original: triggers demasiado restrictivos

ADR-055 original tiene estos reopen triggers:
1. **ADR-019 perf budget breach** (bundle >10MB) — esto es una **regresión**, no un trigger de feature
2. **≥1 external-distribution consumer requesting redacted form** — excluye completamente el caso de uso interno

**Bundle actual verificado (2026-08-18)**:
```
$ du -sh /tmp/bundle-test
72K
```
72KB << 10MB. El trigger de perf no se cumple ni es deseable que se cumpla.

### El caso de uso real está siendo ignorado

El Contexto de ADR-055 reconoce explícitamente:
> "Para onboarding/consultoría/agentes interesa compartir arquitectura sin entregar source, paths sensibles, secretos ni DB completa."

Este caso de uso NO requiere un "external distribution consumer". Un consultor externo que recibe un bundle sanitizado es un **consumidor interno del workflow del equipo**, no un canal de distribución pública.

---

## Decisión propuesta

### Nuevos reopen triggers para ADR-055

Reemplazar los triggers originales:

| Trigger original | Problema | Nuevo trigger |
|-----------------|----------|---------------|
| ADR-019 perf breach (>10MB) | Causar breach = romper producto | **Eliminado** — es un riesgo, no un feature trigger |
| ≥1 external-distribution consumer | Excluye uso interno legítimo | **≥1 stakeholder (interno o externo) que necesite compartir arquitectura sin código fuente** |

### Criterio de decisión

La decisión de implementar sanitized bundle NO debe depender de:
- Una regresión de perf (ADR-019 breach)
- Una definición restrictiva de "external consumer"

La decisión debe depender de:
- Existencia de un caso de uso legítimo (auditoría, consultoría, onboarding)
- Viabilidad técnica (allowlist scanner, checksum, etc.)
- Sin regresión de los invariantes del producto (local-first, evidence-first)

---

## Argumentos para reabrir ADR-055

### 1. El trigger "external consumer" es una definición demasiado estrecha

Excluye:
- Auditores internos que revisan la arquitectura de un proyecto
- Consultores externos bajo NDA que necesitan solo el grafo de arquitectura
- Stakeholders no-técnicos que revisan diagramas sin código fuente
- Equipos de due diligence que evalúan arquitectura sin acceso al código

Todos estos son **casos de uso legítimos y reales** que ADR-055 reconoce pero sus triggers excluyen.

### 2. ADR-011 permite excepciones con opt-in explícito

ADR-011 establece:
> "Renderers públicos bloqueados por defecto" — pero incluye opt-in explícito.

La misma lógica aplica a sanitized bundle: es una **herramienta de seguridad** que debe estar disponible cuando el usuario lo necesita, no solo cuando un "consumer externo" lo pide.

### 3. El riesgo de NO implementar es mayor

El Risk Register (06-RISK-REGISTER.md) lista:
> **ArchBundle filtra secretos** — Prob: Baja/Media, Impacto: Crítico

El riesgo de que un bundle sin sanitización filtre secretos es **Crítico**. No implementar la feature no elimina el riesgo — solo lo deja sin mitigación.

### 4. El MVP es pequeño y de bajo riesgo

El plan de implementación de ADR-055 es:
> MVP export strict manual; archview read-only; profiles custom/firma después.

Esto es un feature flag (`--profile strict`) que no afecta el comportamiento default. El riesgo es contenido.

---

## Fuerzas de diseño (revisadas)

Mantener de ADR-055 original:
- Preservar local-first, evidence-first, source-read-only
- Mantener grafo canónico como única fuente semántica
- Favor determinismo, testabilidad, reversibilidad

**Nuevas**:
- Sanitización deny-by-default para compartir bundles con terceros
- Allowlist > blacklist para metadata
- Checksums para detectar tampering
- Source bytes excluidos por defecto (ya era la decisión)

**Descartar**:
- La distinción entre "internal" y "external" consumer para el trigger

---

## Estrategia de implementación (MVP)

Igual que ADR-055 original:
1. Schema del bundle sanitizado
2. Strict sanitizer (allowlist de metadata permitida)
3. Scanner anti-secretos
4. archview read-only mode
5. Profiles (strict/custom/firma) — fase 2

---

## Criterios de aceptación

- [ ] `--profile strict` exporta sin source paths, secrets, absolute paths
- [ ] Allowlist de metadata definido y documentado
- [ ] Scanner detecta secretos conocidos y los excluye
- [ ] Checksum SHA-256 del bundle para verificar integridad
- [ ] archview abre bundle strict en modo read-only
- [ ] 0 regression en bundle size para perfil default (no-strict)

---

## Changelog

- 2026-08-18 | accepted | Triggers aceptados; ADR-055 reopened; Item 28 Wave 3 desbloqueado
