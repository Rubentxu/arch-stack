/**
 * Layout service — main-thread bridge to ELK layered running in a
 * Web Worker.
 *
 * M19 replaces the M17.1 dagre layout (G6 built-in, sync, in-process)
 * with ELK layered. ELK.js spawns its own Web Worker via the
 * `workerUrl` option, so all `elk.layout(...)` calls run off the
 * main thread. No custom worker file is needed: elkjs ships its own
 * `elk-worker.min.js` that Vite serves via `?url`.
 *
 * The service exposes a single function `computeLayout(bundle,
 * options)` returning `Map<nodeId, {x, y}>`. The renderer's
 * `setData` applies those positions as `style.x`/`style.y` on each
 * node, then tells G6 to use `layout: { type: 'none' }` so it does
 * NOT recompute. This avoids the G6 v5 custom-layout registration
 * surface and keeps the renderer's job (render) separate from
 * layout's job (place).
 *
 * Dependency-injection seam: `LayoutService` is an interface.
 * `GraphRenderer` accepts an optional `layoutService` in
 * `RendererOptions`; production uses `createLayoutService()`,
 * tests inject a stub that returns predetermined positions.
 *
 * Fallback: if the browser does not support `Worker` (very old
 * browsers, some test environments), the service runs ELK in the
 * main process. Layout will block the main thread, but the
 * `Promise`-based API is unchanged, so callers don't care.
 */

import ELK from "elkjs/lib/elk.bundled.js";
// Vite resolves `?url` to a string URL for the worker asset.
// In the Vitest (Node) environment, this import is replaced by the
// vite-node transform; the `?url` suffix yields a string, so the
// branch below safely handles an empty string (in-process fallback).
import ElkWorkerUrl from "elkjs/lib/elk-worker.min.js?url";
import type { RendererBundle } from "../types";
import { DEFAULT_LAYOUT } from "./layout-presets";

/** Position in canvas coordinates. Top-left origin, y grows down. */
export interface NodePosition {
  x: number;
  y: number;
}

/**
 * ELK layered algorithm options. Keys are passed through verbatim
 * to ELK as `layoutOptions`. See `layout-presets.ts` for the workbench
 * defaults and a link to the option reference.
 */
export type LayoutOptions = Record<string, string | number | boolean>;

/**
 * Layout service interface. The renderer depends on this, not on
 * ELK directly, so tests can swap a deterministic stub.
 */
export interface LayoutService {
  /**
   * Compute positions for the bundle's nodes. Returns a map from
   * node id to canvas coordinates. Nodes not in the map (orphan,
   * filtered) keep their previous positions on the next `setData`.
   */
  computeLayout(
    bundle: RendererBundle,
    options?: LayoutOptions,
  ): Promise<Map<string, NodePosition>>;
}

// Vite resolves `?url` to a string URL for the asset. In the Vitest
// (Node) environment, the import resolves to a stub URL; we detect
// that and fall back to the in-process ELK (no worker, but the
// async API is unchanged). The empty-string check below is the
// safe path used by tests; the production path picks up the real
// URL Vite serves from `node_modules/elkjs/lib/elk-worker.min.js`.
const ELK_WORKER_URL: string =
  typeof ElkWorkerUrl === "string" && ElkWorkerUrl.length > 0
    ? ElkWorkerUrl
    : "";

/** ELK node input — minimal shape ELK needs. */
interface ElkNodeInput {
  id: string;
  width: number;
  height: number;
}

/** ELK edge input — minimal shape ELK needs. */
interface ElkEdgeInput {
  id: string;
  sources: string[];
  targets: string[];
}

/**
 * Real layout service backed by ELK. Constructs the ELK instance
 * lazily so importing this module is cheap (no work in module init).
 *
 * The ELK constructor accepts a `workerUrl` which, when set, causes
 * all `elk.layout(...)` calls to be marshalled to a Web Worker.
 * Without it, ELK runs in-process (fallback).
 */
class ElkLayoutService implements LayoutService {
  private elkPromise: Promise<{
    layout: (graph: unknown) => Promise<{
      children?: Array<{ id: string; x?: number; y?: number }>;
    }>;
  }> | null = null;

  private getElk(): Promise<{
    layout: (graph: unknown) => Promise<{
      children?: Array<{ id: string; x?: number; y?: number }>;
    }>;
  }> {
    if (this.elkPromise) return this.elkPromise;
    const supportsWorker =
      typeof Worker !== "undefined" && ELK_WORKER_URL.length > 0;
    this.elkPromise = Promise.resolve(
      new ELK(
        supportsWorker ? { workerUrl: ELK_WORKER_URL } : {},
      ) as unknown as {
        layout: (graph: unknown) => Promise<{
          children?: Array<{ id: string; x?: number; y?: number }>;
        }>;
      },
    );
    return this.elkPromise;
  }

  async computeLayout(
    bundle: RendererBundle,
    options?: LayoutOptions,
  ): Promise<Map<string, NodePosition>> {
    const elk = await this.getElk();
    const merged: LayoutOptions = { ...DEFAULT_LAYOUT, ...(options ?? {}) };

    // ELK needs explicit width/height per node. We read the size
    // from the renderer's token (--g6-node-size) and assume
    // square-ish nodes. The renderer applies a labelBackground
    // that grows the visible bounding box; ELK uses pure node
    // dimensions for layout, not label extents — that's why the
    // G6 layout: "none" + pre-positioned nodes pattern works.
    const nodeSize = readNodeSize();
    const children: ElkNodeInput[] = bundle.nodes.map((n) => ({
      id: n.id,
      width: nodeSize,
      height: nodeSize,
    }));
    const edges: ElkEdgeInput[] = bundle.edges.map((e) => ({
      id: e.id,
      sources: [e.source],
      targets: [e.target],
    }));

    const result = await elk.layout({
      id: "root",
      layoutOptions: merged,
      children,
      edges,
    });

    const positions = new Map<string, NodePosition>();
    for (const child of result.children ?? []) {
      if (typeof child.x === "number" && typeof child.y === "number") {
        positions.set(child.id, { x: child.x, y: child.y });
      }
    }
    return positions;
  }
}

/**
 * Read the node size from the design-system token. Returns a
 * sensible default if the CSS variable is unset (e.g. in
 * non-DOM test environments).
 */
function readNodeSize(): number {
  if (typeof document === "undefined") return 24;
  const raw = getComputedStyle(document.documentElement)
    .getPropertyValue("--g6-node-size")
    .trim();
  if (!raw) return 24;
  // The token value is `var(--g6-node-size)` whose underlying
  // definition is a plain number (e.g. "14"). A simple
  // parseFloat suffices.
  const n = Number.parseFloat(raw);
  return Number.isFinite(n) && n > 0 ? n : 24;
}

/** Factory — creates the default (ELK-backed) layout service. */
export function createLayoutService(): LayoutService {
  return new ElkLayoutService();
}
