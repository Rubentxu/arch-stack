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
 * M17.1: the renderer now accepts a `layout` config and a
 * `nodeStyle` config (color by C4 level / by kind) so each view
 * (C4, CallGraph, ClassDiagram, etc.) can shape the same engine
 * without owning G6 details (R3 of the renderer contract).
 */

import { Graph, NodeEvent, type DisplayObject } from "@antv/g6";
import type { RendererBundle, RendererNode } from "../types";

/** G6 v5 layout config — kept as `unknown` to avoid pulling the
 *  full G6 type tree into the renderer surface area. Views pass
 *  pre-shaped configs from the doc. */
export type G6Layout = unknown;

export interface RendererOptions {
  container: HTMLElement;
  width: number;
  height: number;
  /** Optional initial layout. Defaults to `d3-force` if omitted. */
  layout?: G6Layout;
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

export class GraphRenderer {
  private graph: Graph | null = null;
  private currentBundle: RendererBundle | null = null;
  private nodeStyle: NodeStyleConfig;
  private selectedNodeId: string | null = null;
  private onNodeClickHandler: ((id: string) => void) | null = null;

  constructor(private options: RendererOptions) {
    this.nodeStyle = { ...DEFAULT_NODE_STYLE, ...options.nodeStyle };
    this.onNodeClickHandler = options.onNodeClick ?? null;
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
      // M17.1 — layout is now passed by the caller (view-driven).
      // Default to d3-force if not provided.
      layout: (this.options.layout as Record<string, unknown> | undefined) ?? {
        type: "force",
        preventOverlap: true,
        nodeSize: 24,
        gravity: 0.5,
      },
      node: {
        type: "circle",
        style: {
          size: 18,
          fill: (d: { data?: { level?: number; kind?: string } }) =>
            this.colorForNode(d.data),
          stroke: this.nodeStyle.defaultStroke ?? "#1f3a5f",
          lineWidth: 1.5,
          labelText: (d: { data?: { label?: string } }) => d.data?.label ?? "",
          labelFill: this.nodeStyle.labelFill ?? "#e6edf3",
          labelFontSize: 11,
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

  setData(bundle: RendererBundle): void {
    this.currentBundle = bundle;
    if (!this.graph) return;
    void this.graph.setData({
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
      })),
      edges: bundle.edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        data: { label: e.label, kind: e.kind, ...e.meta },
      })),
    });
    void this.graph.draw().then(() => {
      // Auto-fit after first paint so the user sees the whole graph.
      void this.graph?.fitView();
    });
  }

  /**
   * Change the layout dynamically. Calls `graph.setLayout(...)` then
   * `graph.layout()` and re-centers the view. Use this when the user
   * toggles layout direction (TB / LR) or after a drill-in that
   * changes the graph's effective shape.
   */
  async setLayout(layout: G6Layout): Promise<void> {
    if (!this.graph) return;
    this.graph.setLayout(layout as Record<string, unknown>);
    await this.graph.layout();
    this.graph.fitCenter();
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
