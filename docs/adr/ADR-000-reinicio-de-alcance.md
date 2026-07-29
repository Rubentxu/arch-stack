# ADR-000 — Reinicio de alcance

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

Las propuestas anteriores ampliaron `archctl` hasta convertirlo en una plataforma de inteligencia arquitectónica, reporting y documentación general.

El objetivo real es utilizar OpenCode, agentes y skills para producir diagramas C4 y UML, con una CLI auxiliar para extracción y persistencia.

## Decisión

Se sustituyen las decisiones anteriores por este alcance:

- OpenCode es el entorno principal.
- Los agentes y skills interpretan y modelan.
- `archctl` persiste, consulta y proyecta.
- LadybugDB es la base de grafos embebida.
- Los diagramas son vistas del grafo.
- Los datos permanecen fuera del repositorio.
- Se reutilizan skills y herramientas existentes.
- No se construye un portal ni un motor de análisis propio.

## Consecuencias positivas

- Fronteras claras.
- Menor complejidad.
- Persistencia adecuada a relaciones y recorridos.
- C4 y UML comparten identidades.
- Valor temprano.

## Consecuencias negativas

- Es necesario mantener un metamodelo y migraciones.
- La colaboración entre máquinas requerirá bundles o almacenamiento compartido futuro.
- LadybugDB introduce una dependencia nativa.

## Criterio de ampliación

Una capacidad solo entra si mejora directamente:

1. extracción de evidencias;
2. modelado C4/UML;
3. generación o revisión de diagramas;
4. persistencia, consulta o recuperación del trabajo.
