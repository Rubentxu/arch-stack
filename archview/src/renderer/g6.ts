/**
 * Graph renderer — wraps G6 5.x WebGPU for the workbench canvas.
 *
 * Performance budget (per ADR-019):
 * - TTFP <1s for <10k nodes
 * - Pan/zoom 60 FPS
 * - Filter <50ms
 *
 * The renderer is a thin wrapper around G6. It does NOT own
 * state — the SolidJS component drives bundle updates into the
 * renderer via `setData()` calls.
 */

import { Graph } from "@antv/g6";
import type { GraphBundle } from "../bundle/loader";

export interface RendererOptions {
  container: HTMLElement;
  width: number;
  height: number;
}

export class GraphRenderer {
  private graph: Graph | null = null;
  private currentBundle: GraphBundle | null = null;

  constructor(private options: RendererOptions) {
    this.init();
  }

  private init(): void {
    this.graph = new Graph({
      container: this.options.container,
      width: this.options.width,
      height: this.options.height,
      autoFit: "view",
      background: "#0e1116",
      data: { nodes: [], edges: [] },
      node: {
        style: { fill: "#5b8def", stroke: "#1f3a5f", lineWidth: 1 },
      },
      edge: {
        style: { stroke: "#7a8aa0", lineWidth: 1 },
      },
      behaviors: ["drag-canvas", "zoom-canvas", "drag-element"],
    });
    this.graph.render().catch((err: unknown) => {
      console.error("G6 render failed:", err);
    });
  }

  setData(bundle: GraphBundle): void {
    this.currentBundle = bundle;
    if (!this.graph) return;
    void this.graph.setData({
      nodes: bundle.nodes.map((n) => ({
        id: n.id,
        data: { label: n.label, kind: n.kind, ...n.meta },
      })),
      edges: bundle.edges.map((e) => ({
        id: e.id,
        source: e.source,
        target: e.target,
        data: { label: e.label, kind: e.kind, ...e.meta },
      })),
    });
    void this.graph.draw();
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

  get bundle(): GraphBundle | null {
    return this.currentBundle;
  }
}
