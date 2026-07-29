# ADR-004 — Persistencia externa XDG por proyecto y worktree

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

El repositorio analizado no debe llenarse de configuración de agentes, bases, modelos, renders, snapshots o cachés.

## Decisión

No se escribe dentro del repositorio por defecto.

### Configuración

```text
$XDG_CONFIG_HOME/opencode-architecture/
$XDG_CONFIG_HOME/archctl/
```

### Datos persistentes

```text
$XDG_DATA_HOME/archctl/projects/
└── <host>/<owner>/<repo>--<repository-id>/
    ├── architecture.lbdb
    ├── project.json
    ├── models/
    ├── diagrams/
    ├── rendered/
    ├── exports/
    └── worktrees/
```

### Estado

```text
$XDG_STATE_HOME/archctl/projects/<repository-id>/
├── locks/
└── runs/
```

### Caché

```text
$XDG_CACHE_HOME/archctl/projects/<repository-id>/
├── ast-grep/
├── semantic-indexes/
├── imports/
└── renders/
```

## Identidad

```text
repository-id = hash(remote-normalizado + primer-commit)
worktree-id   = hash(repository-id + ruta-canónica)
```

Fallback sin remoto:

```text
hash(git-common-dir + primer-commit)
```

## Espejo lógico

Las evidencias conservan rutas relativas:

```text
src/orders/service.rs
```

No se duplica el fichero fuente completo salvo configuración explícita.

## Overlays

Los cambios no confirmados se asocian a snapshots `worktree_overlay`.

## Consecuencias

- `git status` permanece limpio.
- Varios worktrees no se pisan.
- Clones del mismo repositorio pueden localizar el mismo proyecto lógico.
- OpenCode necesita permiso `external_directory` para el área XDG.
