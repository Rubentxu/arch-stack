# ADR-010 — Concurrencia de LadybugDB y procesos `archctl`

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

LadybugDB es embebida. No deben existir simultáneamente objetos de base independientes sobre el mismo fichero si uno opera en escritura.

OpenCode puede ejecutar varios subagentes y varias invocaciones de `archctl`.

## Decisión del MVP

Cada comando:

1. resuelve el proyecto;
2. adquiere un bloqueo exclusivo por proyecto;
3. abre `architecture.lbdb`;
4. crea una conexión;
5. ejecuta una transacción corta;
6. cierra la base;
7. libera el bloqueo.

Ruta:

```text
$XDG_STATE_HOME/archctl/projects/<id>/locks/architecture.lock
```

Las extracciones costosas se ejecutan fuera del bloqueo.

Solo la importación y actualización final bloquean la base.

## Lecturas

El MVP serializa también lecturas para evitar combinaciones inseguras entre procesos independientes.

## Evolución

Si la serialización se convierte en cuello de botella se añadirá opcionalmente:

```text
archctld
```

Responsabilidades:

- mantener un único objeto `Database` abierto;
- crear múltiples conexiones;
- aceptar peticiones por Unix socket;
- conservar la misma API de dominio.

No será requisito para instalación ni uso inicial.

## Consecuencias

- Comportamiento seguro y simple.
- Menor paralelismo de consultas.
- No se necesita servidor en el MVP.
- La extracción puede continuar en paralelo antes de importar.
