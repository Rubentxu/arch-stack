import { describe, it, expect } from "vitest";
import { normalizeBundle, c4LevelForType } from "../bundle/loader";

/**
 * R1/R2/R7 — canonical `viewer-bundle` ingestion (spec m17-contract-alignment).
 *
 * A canonical bundle has four sections: `manifest`, `projection`,
 * `evidence`, `styles`. The loader maps projection `type` → C4 level,
 * `predicate` → edge label, `evidenceRefs` → node meta, and derives
 * `parentId` from slash-delimited `canonicalKey` namespaces.
 */

const MINIMAL_BUNDLE = {
  manifest: {
    schemaVersion: "1.0.0",
    format: "viewer-bundle",
    viewSelector: "container:*",
    baseRevision:
      "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    generatedAt: "2026-08-03T12:00:00Z",
    elementCount: 3,
    edgeCount: 1,
    evidenceCount: 1,
  },
  projection: {
    nodes: [
      {
        id: "n1",
        type: "container",
        name: "orders",
        canonicalKey: "orders",
        description: "Order service",
        evidenceRefs: ["ev:1"],
      },
      {
        id: "n2",
        type: "container",
        name: "orders-api",
        canonicalKey: "orders/api",
      },
      {
        id: "n3",
        type: "component",
        name: "orders-api-application",
        canonicalKey: "orders/api/application",
      },
    ],
    edges: [
      { id: "e1", source: "n2", target: "n1", predicate: "uses" },
    ],
  },
  evidence: {
    evidence: [
      {
        id: "ev:1",
        kind: "structural",
        claim: "orders container declared",
        path: "Cargo.toml",
        startLine: 1,
        endLine: 5,
        toolName: "archctl",
        toolVersion: "0.13.8",
        ruleId: "cargo-workspace",
        contentHash: "abc",
        observedAt: "2026-08-03T12:00:00Z",
      },
    ],
  },
  styles: {
    theme: "default",
    version: "1.0.0",
    elementColors: {
      context: "#1168bd",
      container: "#438dd5",
      component: "#85b8e8",
      dynamic: "#2694ab",
      deployment: "#999999",
    },
    edgeColors: { default: "#707070" },
  },
};

describe("canonical viewer-bundle loader (R1)", () => {
  it("loads a complete bundle as c4 with schema version from manifest", () => {
    const bundle = normalizeBundle(MINIMAL_BUNDLE, "test");
    expect(bundle.rawKind).toBe("c4");
    expect(bundle.schemaVersion).toBe("1.0.0");
    expect(bundle.nodes).toHaveLength(3);
    expect(bundle.edges).toHaveLength(1);
    // evidence + styles preserved for consumers
    expect(bundle.evidence).toBeDefined();
    expect(bundle.styles).toBeDefined();
  });

  it("maps c4 types to levels: context 1, container 2, component 3, dynamic 1, deployment 1", () => {
    const raw = {
      ...MINIMAL_BUNDLE,
      manifest: { ...MINIMAL_BUNDLE.manifest, elementCount: 5 },
      projection: {
        nodes: [
          { id: "a", type: "context", name: "A" },
          { id: "b", type: "container", name: "B" },
          { id: "c", type: "component", name: "C" },
          { id: "d", type: "dynamic", name: "D" },
          { id: "e", type: "deployment", name: "E" },
        ],
        edges: [],
      },
    };
    const bundle = normalizeBundle(raw, "test");
    const levelOf = (id: string) =>
      bundle.nodes.find((n) => n.id === id)?.level;
    expect(levelOf("a")).toBe(1);
    expect(levelOf("b")).toBe(2);
    expect(levelOf("c")).toBe(3);
    expect(levelOf("d")).toBe(1);
    expect(levelOf("e")).toBe(1);
  });

  it("turns predicate into the normalized edge kind and label", () => {
    const bundle = normalizeBundle(MINIMAL_BUNDLE, "test");
    const edge = bundle.edges[0];
    expect(edge.id).toBe("e1");
    expect(edge.source).toBe("n2");
    expect(edge.target).toBe("n1");
    expect(edge.kind).toBe("uses");
    expect(edge.label).toBe("uses");
  });

  it("keeps evidenceRefs on node meta and preserves evidence for consumers", () => {
    const bundle = normalizeBundle(MINIMAL_BUNDLE, "test");
    const orders = bundle.nodes.find((n) => n.id === "n1")!;
    expect(orders.meta).toBeDefined();
    expect(orders.meta!.evidenceRefs).toEqual(["ev:1"]);
    expect(bundle.evidence).toEqual(MINIMAL_BUNDLE.evidence);
  });

  it("rejects an incomplete bundle naming the missing section", () => {
    const raw = { ...MINIMAL_BUNDLE };
    delete (raw as Record<string, unknown>).styles;
    expect(() => normalizeBundle(raw, "test")).toThrow(/styles/);
  });
});

describe("deterministic load metadata (R2)", () => {
  it("sets loadedAt to manifest.generatedAt and normalizes identically", () => {
    const first = normalizeBundle(MINIMAL_BUNDLE, "src://bundle");
    const second = normalizeBundle(MINIMAL_BUNDLE, "src://bundle");
    expect(first.loadedAt).toBe("2026-08-03T12:00:00Z");
    expect(second.loadedAt).toBe("2026-08-03T12:00:00Z");
    expect(first).toEqual(second);
    // Node order, edge order, and metadata are all preserved.
    expect(first.nodes.map((n) => n.id)).toEqual(["n1", "n2", "n3"]);
  });
});

describe("namespace-derived hierarchy (R7)", () => {
  it("derives parentId from the closest exact canonicalKey prefix", () => {
    const bundle = normalizeBundle(MINIMAL_BUNDLE, "test");
    const byId = new Map(bundle.nodes.map((n) => [n.id, n]));
    expect(byId.get("n2")!.parentId).toBe("n1"); // orders/api → orders
    expect(byId.get("n3")!.parentId).toBe("n2"); // orders/api/application → orders/api
  });

  it("leaves flat or unmatched keys without parentId", () => {
    const raw = {
      ...MINIMAL_BUNDLE,
      projection: {
        nodes: [
          { id: "x", type: "container", name: "flat", canonicalKey: "flat" },
          { id: "y", type: "container", name: "orphan", canonicalKey: "no/prefix/match" },
          { id: "z", type: "context", name: "no-key" },
        ],
        edges: [],
      },
    };
    const bundle = normalizeBundle(raw, "test");
    for (const n of bundle.nodes) {
      expect(n.parentId).toBeUndefined();
    }
  });
});
