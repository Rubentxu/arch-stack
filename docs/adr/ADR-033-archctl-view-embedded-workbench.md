# ADR-033 — `archctl view`: workbench embebido como servicio local one-shot

> **Ciclo:** `stack-distribution` (post-v1.0.0)
> **Estado:** Aceptado
> **Fecha:** 2026-08-06
> **Refuerza:** [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md), [ADR-019](ADR-019-performance-budget.md)
> **Complementa:** [ADR-020](ADR-020-renderer-stack.md) (renderer stack)

## Contexto

El producto `arch-stack` tiene tres artefactos que se versionan juntos:
`archctl` (CLI sidecar Rust), `archview` (workbench web SolidJS + G6) y las
skills/agentes de OpenCode. Hasta ahora `archview` se distribuía como
proyecto fuente (dev server de Vite) — un usuario necesitaba clonar el
monorepo, instalar pnpm y ejecutar `pnpm dev` para ver un diagrama. Eso no es
distribución de producto.

Las opciones de empaquetado standalone para `archview`:

| Opción | Binarios | WebGPU | Peso | Evaluación |
|---|---|---|---|---|
| Electron | 1/OS | ✅ Chromium | 100MB+ | ❌ Viola ADR-019 (memory <500MB para 100k nodos) |
| Tauri | 1/OS | ⚠️ WebKitGTK Linux sin WebGPU → WebGL2 | 3-10MB | ⚠️ Rompe budget en Linux, el OS target principal |
| **`archctl view` (HTTP local one-shot)** | **1/OS (ya existe el binario)** | ✅ Navegador nativo | +~500KB | ✅ Cumple ADR-010/011/019 |

`archview` es un bundle estático (`dist/`, ~1.5MB sin sourcemaps). El
renderer G6 5.x WebGPU (ADR-020) requiere WebGPU — disponible en el navegador
del usuario (Chrome 113+, Safari 17+, Firefox 121+) pero NO en WebKitGTK
(webview de Tauri en Linux). Además, ADR-020 exige headers COOP/COEP para
SharedArrayBuffer (WASM multi-thread) — un servidor local controlado los
provee; `file://` no.

## Decisión

**`archctl view` sirve el workbench `archview` como servicio HTTP local
one-shot** (127.0.0.1, puerto efímero por defecto, se apaga con Ctrl+C):

- El `dist/` de archview se **embebe en el binario** vía `rust-embed`
  (feature `include-flate`, gzip). Sin artefactos sidecar: un binario por OS
  contiene CLI + servicio + workbench.
- Servidor HTTP mínimo con `tiny_http` (sync, sin tokio — coherente con
  ADR-010: no daemon hasta que la concurrencia lo justifique).
- Endpoints:
  - `GET /` → `index.html` del workbench
  - `GET /assets/*` → assets embebidos (js/css, sourcemaps excluidos)
  - `GET /samples/*` → bundles de ejemplo embebidos (demo sin grafo)
  - `GET /api/health` → `{"status":"ok","version":"..."}` (handshake)
  - `GET /api/export?selector=<s>` → ejecuta `diagram export` contra el
    proyecto activo y devuelve el bundle (interactividad real con el grafo)
- Headers por defecto: `COOP: same-origin`, `COEP: require-corp` (ADR-020),
  `Cross-Origin-Resource-Policy: same-origin` (ADR-011).
- Sin bind público: solo `127.0.0.1`. Cero escritura en el repo del usuario
  (ADR-004): el servidor lee el grafo desde XDG.

## Empaquetado

- Carpeta `archctl/assets-view/` (gitignored, excepto README) es el origen
  del embed. `scripts/embed-view.sh` copia `archview/dist` → `assets-view/`
  excluyendo `*.map`.
- Build local: `pnpm build` en archview → `scripts/embed-view.sh` → `cargo
  build`. Sin dist copiado, `archctl view` compila (embed con README) y
  devuelve error claro "view assets not embedded; run scripts/embed-view.sh".
- CI release: job `web` builda archview, copia, y el job `rust` compila con
  los assets embebidos. El binario resultante incluye el workbench.

## Consecuencias

Positivas:
- `archview` en cualquier OS donde corra `archctl` — cero packaging extra.
- WebGPU nativa del navegador + COOP/COEP correctos (ADR-019/020).
- El stack queda en 2 piezas versionadas juntas: binario (todo dentro) +
  skills.
- El workbench gana interactividad real: `/api/export` conecta la UI con el
  grafo LadybugDB, sin cambiar la arquitectura read-only de archview.

Negativas / trade-offs:
- `archctl view` no es una app de ventana propia (icono, dock). Si se
  necesita, PWA (manifest + service worker) es la vía futura sin packaging.
- El puerto efímero cambia entre runs; `--port` fijo disponible para
  integraciones.
- El tamaño del binario crece ~500KB-1.5MB (gzip embed).

## Alternativas descartadas

- **Electron**: memoria y bundle size violan ADR-019.
- **Tauri**: WebGPU ausente en WebKitGTK (Linux), el target primario;
  asimétrico entre OS.
- **PWA standalone ahora**: viable solo sobre un origen servido — `archctl
  view` es el servidor que la habilita (futuro).
