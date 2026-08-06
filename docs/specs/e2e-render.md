# Spec — E2E Render Suite (`e2e/render_e2e.py`)

> **Referencia:** [ADR-034](adr/ADR-034-e2e-coverage-expansion.md) §2
> **Estado:** Propuesta — 2026-08-06
> **Milestone:** M29 (E2E coverage expansion)

## Objetivo

Verificar que el workbench `archview` (servido por `archctl view`) **muestra
lo que el grafo contiene** — el render es fiel al bundle. La suite abre el
workbench en un browser headless (playwright), carga bundles (samples y
reales) y **asserta el DOM**: nodes visibles, labels, relaciones, vista
activa. Cero errores de consola JS.

## Por qué esto es crítico

El bug de `detectKind` (2026-08-06, PR #57) clasificaba call-graphs como
class-diagrams — la vista call-graph **nunca montaba** y nadie lo notó hasta
la verificación manual con playwright. Una suite de render versionada habría
detectado el fallo en el primer run.

## Alcance

### 1. Bundles de muestra (samples embebidos)

| Bundle | Vista esperada | Assert DOM |
|---|---|---|
| `c4-context.json` | C4View | `.c4-view` presente, ≥1 `.c4-element`, labels del contexto |
| `c4-container.json` | C4View | `.c4-level` container, ≥1 `.c4-element` |
| `sequence.json` | SequenceView | "N participants · M interactions", rows de interacción |
| `class-diagram.json` | ClassDiagramView | "N classes · M relations", nombres de clases |
| `call-graph.json` | CallGraphView/Impact | nodos del call-graph visibles, `rawKind == call-graph` |

### 2. Bundles reales (multi-lenguaje)

Exportar con el binario release desde repos reales (cache `~/.cache/archctl-smoke`):

| Repo | Lenguaje | Extractores previos | Bundle |
|---|---|---|---|
| tokio-rs/axum | rust | c4-discover --apply | container:* |
| BurntSushi/ripgrep | rust | c4-discover --apply | container:* |
| pmndrs/zustand | typescript | c4-discover --apply | container:* |
| expressjs/express | javascript | c4-discover --apply | container:* |
| labstack/echo | go | call-graph --apply | (call-graph) |
| psf/requests | python | class-diagram --apply | (class) |

Para cada bundle: cargar vía `/api/export` (si hay grafo) o ruta de archivo,
assertar contenido (≥1 elemento si el repo tiene containers detectables).

### 3. Invariantes globales

- `GET /api/health` → 200 `{"status":"ok",...}`.
- Headers COOP/COEP/CORP presentes en respuestas estáticas (ADR-020/011).
- Cero errores de consola JS (collect + assert vacío).
- Screenshot por bundle como artifact (`e2e/artifacts/<bundle>.png`).

## Fuera de alcance

- Verificación pixel-perfect / golden images — la suite asserta DOM
  (semántica), no píxeles. Golden visuales se añadirán si se detectan
  regresiones estilísticas (M29.2 opcional).
- Interacción avanzada (drag, zoom, drill-down) — M29.2.
- A11y audit — suite separada (skill accessibility).

## Procedimiento

```bash
# Prerrequisito: binario release + repos cacheados (o --samples-only)
ARCHCTL_BIN=... python3 e2e/render_e2e.py [--samples-only] [--repo <name>]
```

```python
# Pseudo-estructura
for bundle in BUNDLES:
    server = start_view(port=ephemeral, cwd=repo_dir)
    page = browser.new_page()
    page.goto(server.url)
    page.locator("input[placeholder*='bundle URL']").fill(bundle.url)
    page.keyboard.press("Enter")
    page.wait_for_selector(".c4-view, .sequence-view, .class-view, .impact-view",
                           timeout=8000)
    assert_dom(bundle.expected)
    assert_no_console_errors(page)
    page.screenshot(path=f"e2e/artifacts/{bundle.name}.png")
```

## Criterios de aceptación

| # | Criterio | Método |
|---|---|---|
| 1 | Todos los samples renderizan su vista esperada | selector + contenido |
| 2 | Bundles reales renderizan ≥1 elemento cuando el repo tiene containers | DOM |
| 3 | `rawKind` correcto por bundle (regresión detectKind) | bundle-meta |
| 4 | 0 errores de consola JS | listener console |
| 5 | Screenshots generados | filesystem |
| 6 | Exit 0 solo si todas las aserciones pasan | script |

## Entregables

1. `e2e/render_e2e.py` (playwright, sync API, ~200 LOC).
2. `e2e/requirements.txt` (playwright) o documentación de instalación.
3. `e2e/artifacts/` (gitignored) — screenshots de los runs.
4. Integración en `verify-local.sh --full` (si playwright disponible).

## Referencias

- ADR-033 (view embebido), ADR-034 (decisión), ADR-020 (renderer), M29
