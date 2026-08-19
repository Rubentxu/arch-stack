/**
 * Graph renderer — wraps G6 5.x for the workbench canvas.
 *
 * Performance budget (per ADR-019):
 * - TTFP <1s for <10k nodes
 * - Pan/zoom 60 FPS
 * - Filter <50ms
 *
 * The renderer is a thin wrapper around G6. It does NOT own
 * state — the SolidJS component drives bundle updates into the
 * renderer via `setData()` calls.
 *
 * M17.1: the renderer accepts a `nodeStyle` config (color by C4
 * level / by kind) so each view (C4, CallGraph, ClassDiagram,
 * etc.) can shape the same engine without owning G6 details.
 *
 * M19: the renderer no longer runs G6's dagre layout. Instead,
 * `setData` calls a `LayoutService` (default = ELK layered in a
 * Web Worker) to compute positions, embeds them on each node as
 * `style.x` / `style.y`, and asks G6 to use the custom
 * `preset` layout which respects the pre-set positions. The
 * dependency-injection seam (`LayoutService`) lets tests swap a
 * stub without touching the real ELK.
 */

import {
  Graph,
  NodeEvent,
  ExtensionCategory,
  register,
  type DisplayObject,
} from "@antv/g6";
import type { RendererBundle, RendererNode } from "../types";
import { PresetLayout } from "./preset-layout";
import {
  createLayoutService,
  type LayoutOptions,
  type LayoutService,
} from "./layout-client";
import { DEFAULT_LAYOUT } from "./layout-presets";

/** G6 v5 layout config — kept as `unknown` to avoid pulling the
 *  full G6 type tree into the renderer surface area. Views pass
 *  pre-shaped configs from the doc. */
export type G6Layout = unknown;

export interface RendererOptions {
  container: HTMLElement;
  width: number;
  height: number;
  /** Optional initial layout. M19: deprecated — the renderer
   *  ignores this and uses `LayoutService` (ELK) by default.
   *  Kept for backward compatibility with M17.x views that still
   *  pass `{ type: "dagre", ... }`. */
  layout?: G6Layout;
  /** Optional initial layout options (ELK). Defaults to TB layered. */
  layoutOptions?: LayoutOptions;
  /** Optional layout service (ELK by default). Inject a stub in
   *  tests; pass a custom impl for offline / different layout
   *  algorithms. */
  layoutService?: LayoutService;
  /** Optional per-node color/look config. */
  nodeStyle?: NodeStyleConfig;
  /** Optional click handler — fired with the clicked node id. */
  onNodeClick?: (nodeId: string) => void;
}

/**
 * Per-node style config. Either a static style (applied to every
 * node) or a function of node data, OR a `byLevel` / `byKind` map
 * (one color per C4 level or per `kind`).
 *
 * C4 levels:
 *   1 = context/dynamic/deployment (Person, SoftwareSystem)
 *   2 = container
 *   3 = component
 *   0 = unknown / out-of-band
 */
export interface NodeStyleConfig {
  byLevel?: Record<number, string>;
  byKind?: Record<string, string>;
  defaultFill?: string;
  defaultStroke?: string;
  selectedStroke?: string;
  /** Canvas background (G6's `background` option). */
  background?: string;
  /** Edge stroke color. */
  edgeStroke?: string;
  /** Label text color. */
  labelFill?: string;
  /** Label background fill. */
  labelBackgroundFill?: string;
}

/**
 * Read a CSS custom property from `:root`. The workbench's
 * design system (styles/tokens.css) defines these. Used by
 * the renderer's defaults so the graph picks up the
 * active theme — including the light-mode override under
 * `@media (prefers-color-scheme: light)`.
 */
function readCssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const v = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return v.length > 0 ? v : fallback;
}

/**
 * Like `readCssVar` but returns a number suitable for G6's
 * `size` and `labelFontSize`. The token value is often a
 * `clamp()` (e.g. `clamp(12px, 0.78rem, 13px)`), so a naive
 * `parseFloat` returns NaN. Instead, we ask the browser to
 * resolve the value into a concrete pixel measurement by
 * applying it to a hidden probe element and reading
 * `getComputedStyle().fontSize` / `getBoundingClientRect()`.
 *
 * Returns the `fallback` if the value is unparseable, or if
 * we are not in a DOM environment (jsdom unit tests).
 */
function readCssVarNumber(name: string, fallback: number): number {
  if (typeof document === "undefined") return fallback;
  const root = document.documentElement;
  // Get the raw token string. If the token is `var(--fs-sm)`,
  // getPropertyValue already resolves one level; if `--fs-sm` is
  // itself a clamp, we still get the clamp string and need the
  // probe element below to materialise the final pixel value.
  const raw = getComputedStyle(root).getPropertyValue(name).trim();
  if (!raw) return fallback;
  // Cheap path: a plain number or "<n>px" string.
  const direct = Number.parseFloat(raw);
  if (Number.isFinite(direct) && /^\s*[\d.]+\s*(px)?\s*$/.test(raw)) {
    return Math.round(direct);
  }
  // Fallback path: render the token through a probe element so
  // the browser resolves clamp() / rem / em into a real pixel
  // measurement. The element is not inserted into the document.
  const probe = document.createElement("div");
  probe.style.position = "absolute";
  probe.style.visibility = "hidden";
  probe.style.fontSize = `var(${name})`;
  root.appendChild(probe);
  const resolved = Number.parseFloat(getComputedStyle(probe).fontSize);
  root.removeChild(probe);
  return Number.isFinite(resolved) ? Math.round(resolved) : fallback;
}

const DEFAULT_NODE_STYLE: NodeStyleConfig = {
  byLevel: {
    1: readCssVar("--c4-context", "#5b8def"),
    2: readCssVar("--c4-container", "#7ab8ff"),
    3: readCssVar("--c4-component", "#9ec9ff"),
    0: readCssVar("--c4-default", "#7a8aa0"),
  },
  byKind: {
    person: readCssVar("--warn", "#f59e0b"),
    software_system: readCssVar("--c4-context", "#5b8def"),
    container: readCssVar("--c4-container", "#7ab8ff"),
    component: readCssVar("--c4-component", "#9ec9ff"),
    function: readCssVar("--c4-context", "#5b8def"),
    method: readCssVar("--c4-container", "#7ab8ff"),
    class: readCssVar("--c4-component", "#9ec9ff"),
    interface: readCssVar("--accent-2", "#a78bfa"),
    trait: readCssVar("--accent-2", "#a78bfa"),
    enum: readCssVar("--warn", "#fbbf24"),
  },
  defaultFill: readCssVar("--c4-context", "#5b8def"),
  defaultStroke: readCssVar("--border-strong", "#1f3a5f"),
  selectedStroke: readCssVar("--warn", "#fbbf24"),
  background: readCssVar("--bg-0", "#0e1116"),
  edgeStroke: readCssVar("--fg-1", "#7a8aa0"),
  labelFill: readCssVar("--fg-0", "#e6edf3"),
  labelBackgroundFill: readCssVar("--bg-0", "#0e1116"),
};

// Register the `preset` layout exactly once. Vite's HMR may
// re-evaluate this module in dev mode, so guard with a module
// flag. G6's `register` warns on duplicate registration; the
// flag avoids the warning and the small overhead.
let _presetRegistered = false;
function ensurePresetLayoutRegistered(): void {
  if (_presetRegistered) return;
  register(ExtensionCategory.LAYOUT, "preset", PresetLayout);
  _presetRegistered = true;
}

export class GraphRenderer {
  private graph: Graph | null = null;
  private currentBundle: RendererBundle | null = null;
  private nodeStyle: NodeStyleConfig;
  private selectedNodeId: string | null = null;
  private onNodeClickHandler: ((id: string) => void) | null = null;
  private layoutService: LayoutService;
  private currentLayoutOptions: LayoutOptions;
  /**
   * Latest setData promise. New setData calls await the
   * previous one so the graph doesn't receive out-of-order
   * layouts. If a new call comes in while one is in flight,
   * the in-flight one is still awaited (its result will be
   * discarded by the generation check below).
   */
  private renderChain: Promise<void> = Promise.resolve();
  /** Counter — incremented on every setData. A render step
   *  checks this against its captured value and bails if a
   *  newer setData superseded it. */
  private generation = 0;

  constructor(private options: RendererOptions) {
    this.nodeStyle = { ...DEFAULT_NODE_STYLE, ...options.nodeStyle };
    this.onNodeClickHandler = options.onNodeClick ?? null;
    this.layoutService = options.layoutService ?? createLayoutService();
    this.currentLayoutOptions = options.layoutOptions ?? DEFAULT_LAYOUT;
    ensurePresetLayoutRegistered();
    this.init();
  }

  private init(): void {
    this.graph = new Graph({
      container: this.options.container,
      width: this.options.width,
      height: this.options.height,
      autoFit: "view",
      background: this.nodeStyle.background ?? "#0e1116",
      data: { nodes: [], edges: [] },
      // M19 — layout is now `preset` (no-op). Positions come from
      // the model, populated by the LayoutService in `setData`.
      layout: { type: "preset" },
      node: {
        type: "circle",
        style: {
          // M17.C2 / F5: node size and label font size are now read
          // from CSS custom properties so they participate in the
          // design system (and follow light mode). Fallbacks keep
          // the renderer usable in non-DOM environments (jsdom
          // unit tests).
          size: readCssVarNumber("--g6-node-size", 18),
          fill: (d: { data?: { level?: number; kind?: string } }) =>
            this.colorForNode(d.data),
          stroke: this.nodeStyle.defaultStroke ?? "#1f3a5f",
          lineWidth: 1.5,
          labelText: (d: { data?: { label?: string } }) => d.data?.label ?? "",
          labelFill: this.nodeStyle.labelFill ?? "#e6edf3",
          labelFontSize: readCssVarNumber(
            "--g6-label-font-size",
            11,
            // The token value is `var(--fs-sm)` which itself is
            // a `clamp()` expression. Browsers resolve it to a
            // computed pixel value when we read it via
            // getComputedStyle, but G6 wants an integer, so we
            // round.
          ),
          labelBackground: true,
          labelBackgroundFill: this.nodeStyle.labelBackgroundFill ?? "#0e1116",
          labelBackgroundOpacity: 0.7,
          labelPadding: [2, 4, 2, 4] as [number, number, number, number],
        },
      },
      edge: {
        type: "line",
        style: {
          stroke: this.nodeStyle.edgeStroke ?? "#7a8aa0",
          lineWidth: 1,
          endArrow: true,
          endArrowSize: 8,
        },
      },
      behaviors: ["drag-canvas", "zoom-canvas", "drag-element"],
    });
    // M17.1 — wire node click so views can drive selection.
    this.graph.on(NodeEvent.CLICK, (e: { target?: DisplayObject }) => {
      const id = e.target?.id;
      if (typeof id === "string" && id.length > 0) {
        this.onNodeClickHandler?.(id);
      }
    });
    void this.graph.render().catch((err: unknown) => {
      console.error("G6 render failed:", err);
    });
  }

  /**
   * Push a new bundle into the renderer. M19: first asks the
   * `LayoutService` (ELK by default, in a Web Worker) to compute
   * positions, then embeds them on each node, then asks G6 to
   * re-render. Calls are serialised so a fast double-click on
   * a C4 pill does not race the previous layout.
   *
   * Returns void — the work is fire-and-forget from the caller's
   * perspective, but errors are logged via `console.error`.
   */
  setData(bundle: RendererBundle, options?: LayoutOptions): void {
    this.currentBundle = bundle;
    if (options) this.currentLayoutOptions = options;
    const opts = this.currentLayoutOptions;
    const myGen = ++this.generation;
    this.renderChain = this.renderChain
      .then(async () => {
        if (myGen !== this.generation) return; // superseded
        if (!this.graph) return;
        const positions = await this.layoutService.computeLayout(bundle, opts);
        if (myGen !== this.generation) return; // superseded during layout
        this.graph.setData({
          nodes: bundle.nodes.map((n) => ({
            id: n.id,
            data: {
              label: n.label,
              kind: n.kind,
              level: n.level,
              file: n.file,
              parentId: n.parentId,
              ...n.meta,
            },
            style: {
              x: positions.get(n.id)?.x,
              y: positions.get(n.id)?.y,
            },
          })),
          edges: bundle.edges.map((e) => ({
            id: e.id,
            source: e.source,
            target: e.target,
            data: { label: e.label, kind: e.kind, ...e.meta },
          })),
        });
        await this.graph.draw();
        if (myGen === this.generation) {
          this.graph.fitView();
        }
      })
      .catch((err: unknown) => {
        // We swallow the error here because the renderChain is
        // shared across setData calls; a single failure must
        // not poison subsequent layouts. The view layer
        // observes empty/error states by other means.
        console.error("Layout/render failed:", err);
      });
  }

  /**
   * Change the layout dynamically. M19: re-runs the layout
   * service with new options and pushes the new positions to
   * G6 via `updateNodeData`. Returns the promise so callers
   * can await the re-layout (e.g. before re-centering).
   */
  async setLayout(options: LayoutOptions): Promise<void> {
    this.currentLayoutOptions = options;
    const bundle = this.currentBundle;
    if (!this.graph || !bundle) return;
    const myGen = ++this.generation;
    this.renderChain = this.renderChain
      .then(async () => {
        if (myGen !== this.generation) return;
        const positions = await this.layoutService.computeLayout(
          bundle,
          options,
        );
        if (myGen !== this.generation) return;
        const updates = bundle.nodes
          .map((n) => {
            const pos = positions.get(n.id);
            if (!pos) return null;
            return { id: n.id, style: { x: pos.x, y: pos.y } };
          })
          .filter(<T>(x: T | null): x is T => x !== null);
        await this.graph!.updateNodeData(updates);
        await this.graph!.draw();
        if (myGen === this.generation) this.graph!.fitCenter();
      })
      .catch((err: unknown) => {
        console.error("setLayout failed:", err);
      });
    return this.renderChain;
  }

  /**
   * Update the per-node color config. Re-renders with the new
   * style function.
   */
  setNodeStyle(config: NodeStyleConfig): void {
    this.nodeStyle = { ...DEFAULT_NODE_STYLE, ...config };
    if (!this.graph) return;
    // G6 v5: re-render with the new style. The cleanest path is
    // to call setData() with the current bundle; the closure in
    // node.style reads `this.nodeStyle` lazily on each render.
    const bundle = this.currentBundle;
    if (bundle) this.setData(bundle);
  }

  /**
   * Set the node click handler. Replaces the previous one.
   */
  setOnNodeClick(handler: (id: string) => void | null): void {
    this.onNodeClickHandler = handler;
  }

  /**
   * Highlight a single node and fit the view to it. Used by
   * C4View drill-in: when the user clicks a container, the
   * graph focuses on it and its descendants.
   */
  async focusNode(nodeId: string): Promise<void> {
    if (!this.graph) return;
    this.selectedNodeId = nodeId;
    await this.graph.updateNodeData([
      {
        id: nodeId,
        style: {
          stroke: this.nodeStyle.selectedStroke ?? "#fbbf24",
          lineWidth: 3,
        },
      },
    ]);
    void this.graph.draw();
    this.graph.fitView();
  }

  /**
   * Clear the current focus. Resets all node strokes to default.
   */
  async clearFocus(): Promise<void> {
    if (!this.graph) return;
    if (this.selectedNodeId) {
      await this.graph.updateNodeData([
        {
          id: this.selectedNodeId,
          style: {
            stroke: this.nodeStyle.defaultStroke ?? "#1f3a5f",
            lineWidth: 1.5,
          },
        },
      ]);
      void this.graph.draw();
    }
    this.selectedNodeId = null;
  }

  resize(width: number, height: number): void {
    this.options.width = width;
    this.options.height = height;
    if (this.graph) {
      this.graph.setSize(width, height);
    }
  }

  destroy(): void {
    if (this.graph) {
      this.graph.destroy();
      this.graph = null;
    }
  }

  get bundle(): RendererBundle | null {
    return this.currentBundle;
  }

  /** Resolve a fill color for a node based on level/kind. */
  private colorForNode(
    data: { level?: number; kind?: string } | undefined,
  ): string {
    const level = data?.level;
    const kind = data?.kind;
    if (typeof level === "number" && this.nodeStyle.byLevel) {
      const c = this.nodeStyle.byLevel[level];
      if (c) return c;
    }
    if (typeof kind === "string" && this.nodeStyle.byKind) {
      const c = this.nodeStyle.byKind[kind];
      if (c) return c;
    }
    return this.nodeStyle.defaultFill ?? "#5b8def";
  }
}

// re-export RendererNode for views that want to extend the type
export type { RendererNode };
