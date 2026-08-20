/**
 * CullingService — viewport-based visibility culling for G6 graphs.
 *
 * M21: Reduces overdraw on large graphs (1000+ nodes) by hiding
 * elements outside the visible viewport. Visibility is computed
 * based on element bounding boxes vs. the current viewport, with a
 * configurable margin to avoid pop-in at edges.
 *
 * REQUISITES (M21):
 * - REQ-M21-001: culling is opt-in (enableCulling flag)
 * - REQ-M21-004: debounced recompute on viewport changes
 * - REQ-M21-005: single setElementVisibility call per recompute (batch)
 * - REQ-M21-009: hidden elements are not hit-tested (selection contract)
 *
 * DECISION ANCHOR (D1):
 * - CullingService is a pure computation — no G6 state mutation except
 *   through setElementVisibility (a G6 API designed for exactly this)
 * - The service is a DI seam: RendererOptions accepts a cullingService
 *   instance so tests can inject a deterministic stub
 *
 * LayoutService pattern (M19) is the reference implementation for the
 * DI seam contract.
 */

import type { Graph, type NodeData } from "@antv/g6";
import type { RendererBundle } from "../types";
import type { Viewport } from "./g6";

/** Bounding box in canvas coordinates (top-left origin, y grows down). */
export interface BBox {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

/** Element visibility state. */
export type Visibility = "visible" | "hidden";

/** Full visibility map: element id → visibility. */
export type VisibilityMap = Record<string, Visibility>;

/** Culling service options. */
export interface CullingOptions {
  /** Debounce delay in ms for viewport-change events (default 100). */
  debounceMs?: number;
  /** Margin around viewport as fraction of viewport size (default 0.10 = 10%). */
  marginPct?: number;
  /** Whether culling is enabled (default true — false = no-op). */
  enabled?: boolean;
}

/**
 * Culling service interface — a DI seam that hides the G6 graph
 * internals from the culling logic, enabling deterministic stub injection
 * in tests.
 *
 * LayoutService (M19) is the reference pattern.
 */
export interface CullingService {
  /**
   * Recompute visibility for all elements based on the current viewport.
   * Calls graph.setElementVisibility with a full VisibilityMap exactly once.
   * Idempotent: calling while a previous recompute is debounced cancels
   * the pending call.
   */
  recompute(graph: Graph, bundle: RendererBundle, opts: CullingOptions): void;

  /** Tear down timers and subscriptions. Idempotent. */
  teardown(): void;
}

/** Default culling options. */
const DEFAULTS: Required<CullingOptions> = {
  debounceMs: 100,
  marginPct: 0.1,
  enabled: true,
};

/**
 * Pure predicate: is bounding box `b` within viewport `vp` extended by
 * margin `m` (fraction of viewport dimensions)?
 *
 * Defaults: margin = 0.10 (10% of viewport on each side = 20% total overscan)
 *
 * Boundary cases handled:
 * - Empty bbox (minX === maxX || minY === maxY) → outside (culled)
 * - Degenerate viewport (zero width/height) → outside
 */
export function isInViewport(b: BBox, vp: Viewport, m = 0.1): boolean {
  if (
    !isFinite(b.minX) ||
    !isFinite(b.minY) ||
    !isFinite(b.maxX) ||
    !isFinite(b.maxY)
  ) {
    return false;
  }
  if (b.maxX <= b.minX || b.maxY <= b.minY) return false;
  const vpW = vp.maxX - vp.minX;
  const vpH = vp.maxY - vp.minY;
  if (vpW <= 0 || vpH <= 0) return false;

  const marginX = vpW * m;
  const marginY = vpH * m;

  return (
    b.minX >= vp.minX - marginX &&
    b.maxX <= vp.maxX + marginX &&
    b.minY >= vp.minY - marginY &&
    b.maxY <= vp.maxY + marginY
  );
}

/**
 * Compute the bounding-box union of all nodes present in the bundle.
 * Returns a BBox that encloses every node, or a zero/empty BBox if
 * the node list is empty.
 */
export function computeBboxUnion(nodes: NodeData[]): BBox {
  if (nodes.length === 0) {
    return { minX: 0, minY: 0, maxX: 0, maxY: 0 };
  }
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const node of nodes) {
    const style = node.style;
    if (!style) continue;
    const x = style.x as number | undefined;
    const y = style.y as number | undefined;
    const size = (style.size as number | undefined) ?? 18;
    const half = size / 2;

    if (typeof x === "number" && typeof y === "number") {
      minX = Math.min(minX, x - half);
      minY = Math.min(minY, y - half);
      maxX = Math.max(maxX, x + half);
      maxY = Math.max(maxY, y + half);
    }
  }

  return { minX, minY, maxX, maxY };
}

/** No-op culling service stub — used when enableCulling is false. */
export const noopCullingService: CullingService = {
  recompute() {
    // no-op
  },
  teardown() {
    // no-op
  },
};

/**
 * Real culling service backed by G6's setElementVisibility API.
 *
 * Debouncing: each recompute is debounced — a subsequent call within
 * `debounceMs` cancels the previous pending call.
 */
class RealCullingService implements CullingService {
  private pendingTimer: ReturnType<typeof setTimeout> | null = null;
  private lastBounds: BBox | null = null;

  recompute(graph: Graph, bundle: RendererBundle, opts: CullingOptions): void {
    const { debounceMs, marginPct, enabled } = { ...DEFAULTS, ...opts };
    if (!enabled) return;
    if (!graph) return;

    // Cancel any pending debounced recompute.
    if (this.pendingTimer !== null) {
      clearTimeout(this.pendingTimer);
      this.pendingTimer = null;
    }

    const doRecompute = () => {
      const viewport = this.getViewport(graph);
      const visibility = this.computeVisibility(
        graph,
        bundle,
        viewport,
        marginPct,
      );
      graph.setElementVisibility(visibility);
      this.pendingTimer = null;
    };

    this.pendingTimer = setTimeout(doRecompute, debounceMs);
  }

  teardown(): void {
    if (this.pendingTimer !== null) {
      clearTimeout(this.pendingTimer);
      this.pendingTimer = null;
    }
    this.lastBounds = null;
  }

  /** Extract the current viewport from the G6 graph. */
  private getViewport(graph: Graph): Viewport {
    // G6 v5: graph.getCanvas().getViewport() returns a viewport object.
    // Fallback: derive from graph.getZoom() + container size.
    const canvas = graph.getCanvas?.();
    if (canvas && typeof canvas.getViewport === "function") {
      const vp = canvas.getViewport();
      if (vp && typeof vp.minX === "number") {
        return vp as Viewport;
      }
    }
    // Fallback: use fitView bounds or a sentinel for "everything visible".
    const container = graph.getContainer?.();
    if (container) {
      return {
        minX: 0,
        minY: 0,
        maxX: container.clientWidth,
        maxY: container.clientHeight,
      };
    }
    // Last resort: return a large viewport so nothing is culled.
    return { minX: -1e6, minY: -1e6, maxX: 1e6, maxY: 1e6 };
  }

  /**
   * Compute a full VisibilityMap for all nodes and edges in the bundle.
   * REQ-M21-005: exactly one setElementVisibility call per recompute.
   */
  private computeVisibility(
    graph: Graph,
    bundle: RendererBundle,
    viewport: Viewport,
    marginPct: number,
  ): VisibilityMap {
    const result: VisibilityMap = {};
    const nodeData = graph.getNodeData();
    const edgeData = graph.getEdgeData();

    // Compute visibility for each node.
    for (const node of nodeData) {
      const id = node.id as string;
      const style = node.style ?? {};
      const x = style.x as number | undefined;
      const y = style.y as number | undefined;
      const size = (style.size as number | undefined) ?? 18;
      const half = size / 2;

      if (typeof x === "number" && typeof y === "number") {
        const bbox: BBox = {
          minX: x - half,
          minY: y - half,
          maxX: x + half,
          maxY: y + half,
        };
        result[id] = isInViewport(bbox, viewport, marginPct)
          ? "visible"
          : "hidden";
      } else {
        // Nodes without position data are treated as visible.
        result[id] = "visible";
      }
    }

    // Compute visibility for each edge — culled only when BOTH endpoints are hidden.
    for (const edge of edgeData) {
      const id = edge.id as string;
      const source = edge.source as string;
      const target = edge.target as string;
      const sourceVis = result[source];
      const targetVis = result[target];
      // Edge is visible if at least one endpoint is visible (prevents floating edges).
      result[id] =
        sourceVis === "visible" || targetVis === "visible"
          ? "visible"
          : "hidden";
    }

    return result;
  }
}

/**
 * Factory — creates a CullingService instance.
 * Returns noopCullingService when opts.enabled === false.
 */
export function createCullingService(
  opts: CullingOptions = {},
): CullingService {
  if (opts.enabled === false) {
    return noopCullingService;
  }
  return new RealCullingService();
}
