import { describe, it, expect } from "vitest";
import { normalizeBundle } from "../bundle/loader";

/**
 * D4: Contract alignment test — normalizeBundle accepts schema-valid fixture.
 *
 * Loads a fixture that mirrors `build_bundle(&store, "context:*", clock)`
 * output and validates that:
 *   1. normalizeBundle does not throw
 *   2. nodes.length > 0
 *   3. schemaVersion is populated from manifest
 *   4. rawKind is a known C4 value
 *
 * The fixture field names (canonicalKey, evidenceRefs, viewSelector) are
 * validated by the Rust contract_alignment.rs test.
 */

const CONTRACT_FIXTURE = {
  manifest: {
    schemaVersion: "1.0.0",
    format: "viewer-bundle",
    viewSelector: "context:*",
    baseRevision:
      "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    generatedAt: "2026-07-30T12:00:00Z",
    elementCount: 1,
    edgeCount: 0,
    evidenceCount: 0,
  },
  projection: {
    nodes: [
      {
        id: "el:1",
        type: "context",
        name: "Platform",
        canonicalKey: "platform",
        description: "System boundary",
        evidenceRefs: [],
      },
    ],
    edges: [],
  },
  evidence: {
    evidence: [],
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
    edgeColors: {
      default: "#707070",
    },
  },
};

describe("contract alignment — executable bundle (D4)", () => {
  it("normalizeBundle does not throw on schema-valid fixture", () => {
    expect(() => normalizeBundle(CONTRACT_FIXTURE, "contract-fixture")).not.toThrow();
  });

  it("nodes.length > 0 after normalization", () => {
    const bundle = normalizeBundle(CONTRACT_FIXTURE, "contract-fixture");
    expect(bundle.nodes.length).toBeGreaterThan(0);
  });

  it("schemaVersion populated from manifest", () => {
    const bundle = normalizeBundle(CONTRACT_FIXTURE, "contract-fixture");
    expect(bundle.schemaVersion).toBe("1.0.0");
  });

  it("rawKind is a known c4 value", () => {
    const bundle = normalizeBundle(CONTRACT_FIXTURE, "contract-fixture");
    // rawKind should be "c4" for any C4 view bundle (context/container/etc.)
    expect(bundle.rawKind).toBe("c4");
  });

  it("first node has id, label, and level derived from type", () => {
    const bundle = normalizeBundle(CONTRACT_FIXTURE, "contract-fixture");
    const node = bundle.nodes[0];
    expect(node.id).toBe("el:1");
    expect(node.label).toBe("Platform");
    expect(node.level).toBe(1); // context = level 1
  });

  it("loadedAt derives from manifest.generatedAt (deterministic, no wall clock)", () => {
    const bundle = normalizeBundle(CONTRACT_FIXTURE, "contract-fixture");
    expect(bundle.loadedAt).toBe("2026-07-30T12:00:00Z");
  });
});
