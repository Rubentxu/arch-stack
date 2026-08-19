/**
 * Tests for the ELK-backed layout service.
 *
 * The real `elk.bundled.js` constructor requires a Web Worker
 * (`new ELK({ workerUrl: ... })`), and the test environment is
 * node (no `Worker` global). We therefore mock the `elk.bundled.js`
 * module with `vi.mock` and assert that the service correctly
 * marshals the input graph + options and returns the positions
 * ELK produced.
 *
 * What we test:
 *  - `createLayoutService()` returns an object with a
 *    `computeLayout` function.
 *  - `computeLayout` passes the bundle's node ids and edge
 *    source/target pairs to ELK.
 *  - `computeLayout` returns a Map keyed by node id, with
 *    positions taken from the ELK result.
 *  - Per-call options override the defaults.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock the bundled ELK module. The service imports it as the
// default export; the mock replaces the constructor so we can
// observe its calls and return a synthetic layout result.
const layoutMock = vi.fn(async (graph: unknown) => {
  // Find children and edges from the input graph and emit
  // deterministic positions.
  const g = graph as {
    children?: Array<{ id: string }>;
    edges?: Array<{ id: string; sources: string[]; targets: string[] }>;
    layoutOptions?: Record<string, string>;
  };
  const children = g.children ?? [];
  return {
    children: children.map((c, i) => ({
      id: c.id,
      x: i * 100,
      y: 0,
    })),
    edges: g.edges,
    layoutOptions: g.layoutOptions,
  };
});

vi.mock("elkjs/lib/elk.bundled.js", () => {
  return {
    default: function MockElk() {
      return { layout: layoutMock };
    },
  };
});

import {
  createLayoutService,
  type LayoutService,
} from "../renderer/layout-client";
import type { RendererBundle, RendererNode, RendererEdge } from "../types";

function makeBundle(): RendererBundle {
  const nodes: RendererNode[] = [
    { id: "a", label: "A", kind: "container" },
    { id: "b", label: "B", kind: "container" },
    { id: "c", label: "C", kind: "container" },
  ];
  const edges: RendererEdge[] = [
    { id: "e1", source: "a", target: "b" },
    { id: "e2", source: "b", target: "c" },
  ];
  return {
    schemaVersion: "1.0",
    source: "test",
    loadedAt: "2026-01-01T00:00:00Z",
    nodes,
    edges,
    rawKind: "c4",
    strict: true,
  };
}

describe("createLayoutService", () => {
  let service: LayoutService;
  beforeEach(() => {
    layoutMock.mockClear();
    service = createLayoutService();
  });

  it("returns a service exposing a computeLayout function", () => {
    expect(typeof service.computeLayout).toBe("function");
  });

  it("passes bundle nodes and edges to ELK as a child graph", async () => {
    const bundle = makeBundle();
    await service.computeLayout(bundle);

    expect(layoutMock).toHaveBeenCalledTimes(1);
    const arg = layoutMock.mock.calls[0]?.[0] as {
      id?: string;
      children?: Array<{ id: string; width: number; height: number }>;
      edges?: Array<{ id: string; sources: string[]; targets: string[] }>;
    };
    expect(arg.id).toBe("root");
    expect(arg.children?.map((c) => c.id)).toEqual(["a", "b", "c"]);
    expect(arg.edges?.map((e) => e.id)).toEqual(["e1", "e2"]);
    // Edges keep source/target structure for ELK.
    expect(arg.edges?.[0]?.sources).toEqual(["a"]);
    expect(arg.edges?.[0]?.targets).toEqual(["b"]);
  });

  it("returns a Map keyed by node id with positions from ELK", async () => {
    const bundle = makeBundle();
    const positions = await service.computeLayout(bundle);
    expect(positions.size).toBe(3);
    expect(positions.get("a")).toEqual({ x: 0, y: 0 });
    expect(positions.get("b")).toEqual({ x: 100, y: 0 });
    expect(positions.get("c")).toEqual({ x: 200, y: 0 });
  });

  it("merges caller options over the defaults", async () => {
    const bundle = makeBundle();
    await service.computeLayout(bundle, {
      "elk.direction": "RIGHT",
      "elk.algorithm": "layered",
    });
    const arg = layoutMock.mock.calls[0]?.[0] as {
      layoutOptions?: Record<string, string>;
    };
    expect(arg.layoutOptions?.["elk.direction"]).toBe("RIGHT");
    expect(arg.layoutOptions?.["elk.algorithm"]).toBe("layered");
    // Default TB direction is overridden by caller-supplied "RIGHT".
    expect(arg.layoutOptions?.["elk.direction"]).not.toBe("DOWN");
  });

  it("uses default options when caller passes none", async () => {
    const bundle = makeBundle();
    await service.computeLayout(bundle);
    const arg = layoutMock.mock.calls[0]?.[0] as {
      layoutOptions?: Record<string, string>;
    };
    // DEFAULT_LAYOUT (TB_LAYERED) sets these.
    expect(arg.layoutOptions?.["elk.algorithm"]).toBe("layered");
    expect(arg.layoutOptions?.["elk.direction"]).toBe("DOWN");
  });

  it("handles an empty bundle without throwing", async () => {
    const empty: RendererBundle = {
      schemaVersion: "1.0",
      source: "test",
      loadedAt: "2026-01-01T00:00:00Z",
      nodes: [],
      edges: [],
      rawKind: "c4",
      strict: true,
    };
    const positions = await service.computeLayout(empty);
    expect(positions.size).toBe(0);
  });
});
