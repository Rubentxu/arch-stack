# ADR-030 — Ejecutor local manual de GitHub workflows (multi-repo)

**Estado:** Aceptado (MVP local-first)
**Fecha:** 3 de agosto de 2026
**Aplica a:** `workflowctl` (CLI local) — herramienta de equipo para ejecutar manualmente `.github/workflows/*.yml` de repositorios locales antes del push.
**Complementa:** ADR-010 (sin daemon hasta que la concurrencia lo justifique), ADR-011 (renderers locales, sin servicios públicos).

> **Mejora futura (no implementada ahora):** runbook de promoción a un runner remoto efímero dedicado, e incorporación eventual de un coordinator híbrido. Mantener esos diseños aquí, no en archivos operativos, hasta que exista demanda real y un host dedicado.

## Contexto

Las conversaciones de julio–agosto de 2026 exploraron cómo ejecutar los workflows de GitHub Actions definidos en los repositorios del equipo sin depender de GitHub-hosted runners ni de push remoto. Las conclusiones operativas son:

- La ejecución es **siempre manual y bajo demanda**, nunca por commit ni por push automático.
- El objetivo inicial, `arch-stack/.github/workflows/ci.yml`, se amplía a todos los repos del equipo (p. ej. `agents-workflows/.github/workflows/e2e-tests.yml`, futuros CI en otros repos).
- El host disponible (`bazzite-rubentxu`) tiene potencia sobrada: 64 CPU, 94 GiB RAM, 740 GiB libres, `/dev/kvm`, cgroup v2 y SELinux en enforcing. Podman 5.8.4 rootless (socket `/run/user/1000/podman/podman.sock` con `0660`) y `gh act` 0.2.89 están instalados.
- `act` no implementa totalmente GitHub Actions: ignora `concurrency`, `timeout-minutes`, `job.permissions`, anotaciones y cancelación, y declara Podman como no soportado oficialmente aunque funcione vía API Docker-compatible. Sus defaults son peligrosos para un host multiusuario: 64 jobs concurrentes, red `host`, cache/artifact servers vinculados a la LAN.

Una de las alternativas discutidas fue centralizar el ejecutor en la propia máquina de trabajo. La idea es rechazada por tres motivos:

1. El host es una **estación personal** ligada al UID `rubentxu` y a un único rango `subuid/subgid`. No hay identidad de servicio, autenticación multiusuario, Vault, auditoría central ni SLO.
2. El radio de compromiso incluiría datos personales del propietario (incluido `SSH_AUTH_SOCK`) y repositorios del equipo mezclados en el mismo UID pool.
3. No existe demanda medida: solo `arch-stack` y `agents-workflows` tienen workflows hoy (2 de 8 repos). El PinP rootless y los jobs con `fetch-depth: 0` + `git worktree` siguen siendo igual de problemáticos en un runner central.

Una topología híbrida (centro + local) también se descarta como punto de partida porque introduciría dos backends y un coordinator sin evidencia de contención o de varios desarrolladores dependiendo de un runner común.

## Decisión

`workflowctl` se ejecuta **siempre en el host local del desarrollador**, activado manualmente y encapsulado en unidades transitorias de `systemd --user`. No es un servicio compartido ni una plataforma distribuida.

### Topología

```text
orden manual (humana, explícita)
   │
workflowctl (CLI)
   │
snapshot inmutable del repositorio (commit + estado git + dirty explícito opcional)
   │
actionlint + preflight (validación sintáctica y de capacidades)
   │
systemd-run --user --slice=workflowctl.slice (lifecycle + cgroups + journald)
   │
gh act (extensión de gh) vía DOCKER_HOST=unix:///run/user/<uid>/podman/podman.sock
   │
Podman rootless (crun, netavark, usuario actual)
```

### Componentes del MVP

- **Un binario `workflowctl`** que orquesta snapshot, preflight, `systemd-run`, `gh act`, logs y limpieza.
- **Registro local** de repositorios en `XDG_CONFIG_HOME/workflowctl/repos.toml`.
- **Snapshots inmutables** por ejecución, copiados a `XDG_RUNTIME_DIR/workflowctl/runs/<run-id>/repo`.
- **Unidad transitoria** de `systemd --user` por run, con límites de CPU, RAM, PIDs y timeout externos.
- **Logs y manifestos** en `XDG_STATE_HOME/workflowctl/runs/<run-id>`: comando efectivo, versiones resueltas (act, gh, Podman), digests de imágenes, duración, recursos consumidos y exit code.
- **Cache y artifacts** aislados por repositorio y por SHA, escuchando en `127.0.0.1`. Nunca en direcciones LAN.

### Límites por defecto (conservadores)

| Parámetro | Valor |
|---|---|
| Concurrencia global de `workflowctl` | 1 workflow, 2 jobs internos por workflow |
| Perfil estándar | 4 CPU, 8 GiB RAM |
| Perfil pesado | 8 CPU, 16 GiB RAM |
| Perfil benchmark | 12 CPU, 16 GiB RAM, ejecución **exclusiva** (sin otros runs) |
| Presupuesto global inicial | 32 CPU, 48 GiB RAM para conservar la mitad del host |
| Timeout externo por run | 30 min estándar, configurable; obligatorio porque `act` ignora `job.timeout-minutes` |
| Limpieza | siempre, también ante fallo; `--rm` por defecto |
| Socket del daemon | `--container-daemon-socket -` salvo nested-container justificado |
| Red | red transitoria por run; no `--network=host` salvo que el workflow lo exija y se documente |
| Bind mount | nunca `--bind`; siempre se ejecuta contra snapshot copiado |
| Imagen del runner | fijada por digest en `.actrc`; nunca `rust-latest` mutable |
| Secrets | por `--secret-file` mkstemp 0600; nunca inline; nunca reusar `gh auth token` |

### Compatibilidad por workflow

El MVP no tiene como objetivo paridad total con GitHub-hosted. La ejecución local es **validación previa al push**, no reemplazo de CI.

- `arch-stack/.github/workflows/ci.yml` — job `rust` y `web`: compatibles.
- `arch-stack/.github/workflows/ci.yml` — job `bench-smoke` y `bench-compare`: requieren perfil exclusivo; `bench-compare` necesita `git worktree` + `fetch-depth: 0` y se recomienda ejecutarlo via `scripts/bench-compare.sh` directamente.
- `agents-workflows/.github/workflows/e2e-tests.yml` — `test-unit`: compatible.
- `agents-workflows/.github/workflows/e2e-tests.yml` — `test-e2e`, `test-ui`, `test-all`: contienen **Podman-in-Podman**, incompatible con el modelo MVP. Clasificarlos como `nested-container`.

### Política para `nested-container`

Los jobs que invocan `podman build`/`podman run` dentro del propio workflow se marcan `nested-container` y tienen dos salidas explícitas:

1. Diferir el job y proponer su ejecución dentro de un **worker KVM efímero** dedicado (cuando exista); el host ya dispone de `/dev/kvm`.
2. Refactorizar el workflow para sustituir el Podman anidado por un build/load previo ejecutado fuera del job.

Mientras ninguna de las dos exista, el MVP **rechaza** ejecutar esos jobs. No se monta el socket del usuario ni se concede `--privileged` como solución.

## Mejoras futuras (diferidas, no implementadas ahora)

> Mantener estos puntos como **referencia de promoción**. No crear archivos de diseño, ramas, ADRs adicionales ni servicios hasta que se den todas las condiciones de la siguiente lista.

1. **Runner remoto efímero dedicado** — host bare-metal o VM con identidad de servicio sin `~/.ssh`, secret store (pass/vault), `systemd-run --user` dentro de un slice con `Delegate=cpu cpuset io memory pids`, y exposición del API solo mediante autenticación por runner group. Necesario cuando:

   - Haya contención recurrente que degrade el trabajo interactivo del desarrollador.
   - Varios hosts o desarrolladores necesiten la misma cola de ejecuciones.
   - Exista demanda medida para `bench-compare`, `nested-container` o workflows largos.

2. **Coordinator híbrido local + remoto** — añade solo si los dos puntos anteriores se cumplen. NO diseñar antes: el seam local/remoto introduce routing, autenticación distribuida, políticas de fallback y contrato de migración de contexto, problemas que no aportan valor con un solo consumidor.

3. **Imagen base propia `act-runner:arch`** — pre-instalando Rust 1.88, Node 22, Go 1.22, Python 3, git, gzip en una imagen firmada. Reduciría 1–2 min por run. Solo merece el esfuerzo cuando el número de runs por semana justifique la construcción y el mantenimiento.

4. **`forgejo-runner` o similar** — si el equipo decide pasar de "ejecutor manual" a "runner con cola persistente, reintentos y cancelación". Implica mover los workflows a un orquestador distinto, no añadir compatibilidad dentro del MVP.

5. **Perfiles por repositorio** declarativos en `repos.toml`: recursos, timeouts, comandos extra. Hoy los defaults globales son suficientes para los dos repos con workflows.

## Regla de promoción

Pasar de MVP local a cualquier topología distribuida exige, **simultáneamente**:

- Host dedicado, identidad de servicio sin home personal, secret store, propietario operativo y SLO definidos.
- Demanda medida en logs o métricas (contención, varios hosts pidiendo cola, jobs bloqueados por workstation apagado).
- Suite de compatibilidad que verifique que local y remoto interpretan igual el subconjunto soportado de workflows.
- Pin por SHA completo en todas las actions; nada de `@v4` ni `@main`.
- Política canónica única de routing; no condicionales local/remoto dispersos por los workflows.
- Subuid/subgid disjuntos por usuario y auditoría SELinux operativa.

Si falta una sola de estas condiciones, **se mantiene el MVP local**. No hay paso intermedio.

## Consecuencias

### Positivas

- Sin daemon compartido: ciclo de vida por run, sin estado persistente que mantener.
- Sin credenciales multiusuario: el UID del desarrollador ejecuta sus propios workflows con sus propios secretos.
- Sin red compartida: cada run crea una red Podman transitoria.
- Compatible con ADR-010: si la concurrencia lo justifica, el MVP puede evolucionar sin reescribir el flujo principal.
- Reversible: añadir después un runner remoto no obliga a migrar ejecuciones locales.

### Negativas

- Los jobs pesados (`bench-*`, `test-e2e`) compiten con el trabajo interactivo si se ejecutan con perfil estándar.
- El workstation apagado = ejecuciones interrumpidas. No hay disponibilidad cuando el host no esté encendido.
- Cache y artifacts no se comparten entre hosts; cada desarrollador cold-compila su Rust al menos la primera vez.
- Drift de entorno entre hosts: la imagen del runner y las versiones se fijan, pero el resto puede variar.
- `act` no es paridad con GitHub: `permissions`, `timeout-minutes`, cancellation, annotations y `concurrency` siguen siendo huecos. CI real sigue siendo GitHub-hosted.

### Métricas de éxito del MVP

- Latencia del preflight: < 2 s en hosts típicos.
- Tiempo de run consistente entre ejecuciones idénticas (mediana): tolerancia ± 15 % con cache pre-poblada.
- Tasa de jobs rechazados por unsafe flags: ≥ 0 (cero runs con `--bind` o red `host` fuera de whitelist).
- Residuos de Podman tras 1 semana: < 1 GB sin uso de imágenes dangling y 0 volúmenes huérfanos.
- Runs fallidos por timeout externo: trazables al job concreto, no a `act` colgándose.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| Ejecutar solo local | Promover a runner remoto efímero dedicado cuando las condiciones de la regla de promoción se cumplan. |
| `systemd-run --user` por run | Reemplazar por un daemon persistente cuando la concurrencia lo justifique (ADR-010). |
| Snapshot copiado, sin `--bind` | Permitir `--bind` solo bajo whitelist explícita por repositorio. |
| Imagen mutable `catthehacker/ubuntu:rust-latest` | Cambiar a digest fijo en `.actrc`; construir imagen propia cuando el coste lo justifique. |
| Sin coordinación central | Sustituir por coordinator híbrido con autenticación, cuotas y auditoría. |

## Referencias

- ADR-010 — concurrencia sin daemon hasta que se justifique.
- ADR-011 — renderers y servicios locales; ningún egress a Internet sin opt-in.
- `docs/ROADMAP.md` — namespace reservado `Mejoras futuras workflowctl`.
- `gh act --help` (v0.2.89) — defaults peligrosos que este ADR corrige (concurrencia 64, direcciones LAN para cache/artifact).
- [nektosact.com — Unsupported functionality](https://nektosact.com/not_supported.html).
- [GitHub Docs — Hardening for self-hosted runners](https://docs.github.com/en/actions/reference/security/secure-use).
