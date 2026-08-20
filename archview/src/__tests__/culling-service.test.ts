// @vitest-environment jsdom
/**
 * T2 tests — CullingService DI seam (M21).
 *
 * Tests the pure functions and the CullingService interface.
 * Uses vi.useFakeTimers for debounce timing tests.
 *
 * LayoutService pattern (M19) is the reference for DI seam tests.
 */
import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import {
  isInViewport,
  computeBboxUnion,
  noopCullingService,
  createCullingService,
  type BBox,
  type Viewport,
} from "../renderer/culling-service";

describe("isInViewport", () => {
  const vp: Viewport = { minX: 0, minY: 0, maxX: 100, maxY: 100 };

  it("returns true when bbox is inside viewport", () => {
    const bbox: BBox = { minX: 10, minY: 10, maxX: 50, maxY: 50 };
    expect(isInViewport(bbox, vp)).toBe(true);
  });

  it("returns true when bbox touches viewport edge", () => {
    const bbox: BBox = { minX: 0, minY: 0, maxX: 100, maxY: 100 };
    expect(isInViewport(bbox, vp)).toBe(true);
  });

  it("returns false when bbox is entirely outside (right)", () => {
    const bbox: BBox = { minX: 150, minY: 10, maxX: 200, maxY: 50 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false when bbox is entirely outside (left)", () => {
    const bbox: BBox = { minX: -100, minY: 10, maxX: -50, maxY: 50 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false when bbox is entirely outside (above)", () => {
    const bbox: BBox = { minX: 10, minY: -100, maxX: 50, maxY: -50 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false when bbox is entirely outside (below)", () => {
    const bbox: BBox = { minX: 10, minY: 150, maxX: 50, maxY: 200 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false for empty bbox (zero width)", () => {
    const bbox: BBox = { minX: 10, minY: 10, maxX: 10, maxY: 50 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false for empty bbox (zero height)", () => {
    const bbox: BBox = { minX: 10, minY: 10, maxX: 50, maxY: 10 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });

  it("returns false when viewport has zero width", () => {
    const zeroVp: Viewport = { minX: 0, minY: 0, maxX: 0, maxY: 100 };
    const bbox: BBox = { minX: 10, minY: 10, maxX: 50, maxY: 50 };
    expect(isInViewport(bbox, zeroVp)).toBe(false);
  });

  it("applies margin correctly (10% default)", () => {
    // Viewport effective bounds with 10% margin: [-10, -10] to [110, 110]
    const bbox: BBox = { minX: -5, minY: -5, maxX: 5, maxY: 5 };
    expect(isInViewport(bbox, vp)).toBe(true);
    // Just outside 10% margin
    const bbox2: BBox = { minX: -15, minY: -15, maxX: -10, maxY: -10 };
    expect(isInViewport(bbox2, vp)).toBe(false);
  });

  it("respects custom margin parameter", () => {
    // Viewport effective bounds with 20% margin: [-20, -20] to [120, 120]
    const bbox: BBox = { minX: -15, minY: -15, maxX: 115, maxY: 115 };
    expect(isInViewport(bbox, vp, 0.2)).toBe(true);
    expect(isInViewport(bbox, vp, 0.1)).toBe(false);
  });

  it("returns false for non-finite bbox coordinates", () => {
    const bbox: BBox = { minX: Infinity, minY: 10, maxX: 50, maxY: 50 };
    expect(isInViewport(bbox, vp)).toBe(false);
  });
});

describe("computeBboxUnion", () => {
  it("returns zero bbox for empty array", () => {
    const result = computeBboxUnion([]);
    expect(result).toEqual({ minX: 0, minY: 0, maxX: 0, maxY: 0 });
  });

  it("returns correct bbox for single node", () => {
    const nodes = [
      {
        id: "a",
        style: { x: 100, y: 200, size: 20 },
      } as unknown as import("@antv/g6").NodeData,
    ];
    const result = computeBboxUnion(nodes);
    // size=20, half=10 → minX=90, maxX=110, minY=190, maxY=210
    expect(result.minX).toBe(90);
    expect(result.maxX).toBe(110);
    expect(result.minY).toBe(190);
    expect(result.maxY).toBe(210);
  });

  it("returns correct union bbox for multiple nodes", () => {
    const nodes = [
      {
        id: "a",
        style: { x: 0, y: 0, size: 10 },
      } as unknown as import("@antv/g6").NodeData,
      {
        id: "b",
        style: { x: 100, y: 100, size: 10 },
      } as unknown as import("@antv/g6").NodeData,
    ];
    const result = computeBboxUnion(nodes);
    // First: [-5, -5, 5, 5], Second: [95, 95, 105, 105]
    // Union: [-5, -5, 105, 105]
    expect(result.minX).toBe(-5);
    expect(result.minY).toBe(-5);
    expect(result.maxX).toBe(105);
    expect(result.maxY).toBe(105);
  });

  it("uses default size when style.size is missing", () => {
    const nodes = [
      {
        id: "a",
        style: { x: 50, y: 50 },
      } as unknown as import("@antv/g6").NodeData,
    ];
    const result = computeBboxUnion(nodes);
    // Default size 18, half=9 → [41, 41, 59, 59]
    expect(result.minX).toBe(41);
    expect(result.minY).toBe(41);
    expect(result.maxX).toBe(59);
    expect(result.maxY).toBe(59);
  });

  it("ignores nodes without position data", () => {
    const nodes = [
      { id: "a", style: {} } as unknown as import("@antv/g6").NodeData,
      {
        id: "b",
        style: { x: 50, y: 50, size: 10 },
      } as unknown as import("@antv/g6").NodeData,
    ];
    const result = computeBboxUnion(nodes);
    // Only node b contributes → [45, 45, 55, 55]
    expect(result.minX).toBe(45);
    expect(result.minY).toBe(45);
    expect(result.maxX).toBe(55);
    expect(result.maxY).toBe(55);
  });
});

describe("noopCullingService", () => {
  it("recompute is a no-op (does not throw)", () => {
    expect(() =>
      noopCullingService.recompute(
        {} as import("@antv/g6").Graph,
        {} as import("../types").RendererBundle,
        {},
      ),
    ).not.toThrow();
  });

  it("teardown is idempotent (does not throw)", () => {
    expect(() => noopCullingService.teardown()).not.toThrow();
    expect(() => noopCullingService.teardown()).not.toThrow();
  });
});

describe("createCullingService", () => {
  it("returns noopCullingService when enabled=false", () => {
    const svc = createCullingService({ enabled: false });
    expect(svc).toBe(noopCullingService);
  });

  it("returns a RealCullingService instance when enabled=true", () => {
    const svc = createCullingService({ enabled: true });
    expect(svc).not.toBe(noopCullingService);
    expect(typeof svc.recompute).toBe("function");
    expect(typeof svc.teardown).toBe("function");
  });

  it("defaults to RealCullingService when enabled is undefined", () => {
    const svc = createCullingService({});
    expect(svc).not.toBe(noopCullingService);
  });
});

describe("CullingService debounce (fake timers)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("debounces recompute calls by 100ms", () => {
    const svc = createCullingService({ enabled: true, debounceMs: 100 });
    const graph = {
      getCanvas: () => ({
        getViewport: () => ({ minX: 0, minY: 0, maxX: 100, maxY: 100 }),
      }),
      getNodeData: () => [],
      getEdgeData: () => [],
      setElementVisibility: vi.fn(),
      getContainer: () => ({ clientWidth: 100, clientHeight: 100 }),
    } as unknown as import("@antv/g6").Graph;

    svc.recompute(
      graph,
      { nodes: [], edges: [] } as import("../types").RendererBundle,
      {},
    );

    // setElementVisibility should NOT have been called yet (still debouncing)
    expect(graph.setElementVisibility).not.toHaveBeenCalled();

    // Advance time by 99ms — still debouncing
    vi.advanceTimersByTime(99);
    expect(graph.setElementVisibility).not.toHaveBeenCalled();

    // Advance to 100ms — debounce fires
    vi.advanceTimersByTime(1);
    expect(graph.setElementVisibility).toHaveBeenCalledTimes(1);
  });

  it("cancels previous pending recompute when called again within debounce window", () => {
    const svc = createCullingService({ enabled: true, debounceMs: 100 });
    const graph = {
      getCanvas: () => ({
        getViewport: () => ({ minX: 0, minY: 0, maxX: 100, maxY: 100 }),
      }),
      getNodeData: () => [],
      getEdgeData: () => [],
      setElementVisibility: vi.fn(),
      getContainer: () => ({ clientWidth: 100, clientHeight: 100 }),
    } as unknown as import("@antv/g6").Graph;

    svc.recompute(
      graph,
      { nodes: [], edges: [] } as import("../types").RendererBundle,
      {},
    );
    vi.advanceTimersByTime(50);
    svc.recompute(
      graph,
      { nodes: [], edges: [] } as import("../types").RendererBundle,
      {},
    );
    vi.advanceTimersByTime(50);
    // Second call at t=50 should cancel first (at t=0) and reschedule
    expect(graph.setElementVisibility).not.toHaveBeenCalled();
    vi.advanceTimersByTime(50); // t=100, second call fires
    expect(graph.setElementVisibility).toHaveBeenCalledTimes(1);
  });

  it("teardown cancels any pending timer", () => {
    const svc = createCullingService({ enabled: true, debounceMs: 100 });
    const graph = {
      getCanvas: () => ({
        getViewport: () => ({ minX: 0, minY: 0, maxX: 100, maxY: 100 }),
      }),
      getNodeData: () => [],
      getEdgeData: () => [],
      setElementVisibility: vi.fn(),
      getContainer: () => ({ clientWidth: 100, clientHeight: 100 }),
    } as unknown as import("@antv/g6").Graph;

    svc.recompute(
      graph,
      { nodes: [], edges: [] } as import("../types").RendererBundle,
      {},
    );
    svc.teardown();
    vi.advanceTimersByTime(200);
    expect(graph.setElementVisibility).not.toHaveBeenCalled();
  });

  it("setElementVisibility is called exactly once per debounced recompute (batch guarantee)", async () => {
    const svc = createCullingService({ enabled: true, debounceMs: 50 });
    const nodeData = Array.from({ length: 200 }, (_, i) => ({
      id: `node-${i}`,
      style: { x: i * 10, y: i * 10, size: 18 },
    }));
    // Use container fallback (no getCanvas)
    const graph = {
      getNodeData: () => nodeData as unknown as import("@antv/g6").NodeData[],
      getEdgeData: () => [],
      setElementVisibility: vi.fn(),
      getContainer: () => ({ clientWidth: 100, clientHeight: 100 }),
    } as unknown as import("@antv/g6").Graph;

    svc.recompute(
      graph,
      { nodes: [], edges: [] } as import("../types").RendererBundle,
      {},
    );
    // Use runAllTimers to flush pending setTimeout
    vi.runAllTimers();

    // Exactly ONE call with a map of all 200 nodes
    expect(graph.setElementVisibility).toHaveBeenCalledTimes(1);
    const [visibilityMap] = graph.setElementVisibility.mock.calls[0];
    expect(Object.keys(visibilityMap).length).toBe(200);
  });
});
