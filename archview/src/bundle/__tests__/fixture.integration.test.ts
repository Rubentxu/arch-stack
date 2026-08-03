import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { normalizeBundle } from "../loader";

/**
 * R6 — Generated end-to-end fixture.
 *
 * The fixture is a canonical `viewer-bundle` assembled from its four
 * JSON files (`manifest.json`, `projection.json`, `evidence.json`,
 * `styles.json`) — the same layout `archctl diagram export` emits and
 * `archctl diagram validate` accepts. The four files are validated
 * against the canonical schema by the real `archctl diagram validate`
 * before being committed (see apply-progress for provenance details).
 */

const FIXTURE_DIR = resolve(import.meta.dirname, "fixtures/viewer-bundle");

function readJson(name: string): Record<string, unknown> {
  return JSON.parse(readFileSync(resolve(FIXTURE_DIR, name), "utf8")) as Record<
    string,
    unknown
  >;
}

function assembleBundle(): Record<string, unknown> {
  return {
    manifest: readJson("manifest.json"),
    projection: readJson("projection.json"),
    evidence: readJson("evidence.json"),
    styles: readJson("styles.json"),
  };
}

describe("viewer-bundle fixture integration (R6)", () => {
  it("loads the fixture and produces the exported node and edge IDs", () => {
    const bundle = normalizeBundle(assembleBundle(), "fixture://viewer-bundle");

    expect(bundle.rawKind).toBe("c4");
    // Exported element and edge counts from the manifest.
    expect(bundle.nodes).toHaveLength(4);
    expect(bundle.edges).toHaveLength(3);

    const nodeIds = bundle.nodes.map((n) => n.id).sort();
    expect(nodeIds).toEqual([
      "c4:component:auth:login",
      "c4:container:api",
      "c4:container:auth",
      "c4:container:shared",
    ]);
  });

  it("preserves edge endpoints and predicate labels from the fixture", () => {
    const bundle = normalizeBundle(assembleBundle(), "fixture://viewer-bundle");

    const edge = bundle.edges.find(
      (e) => e.id === "rel:c4:container:api:c4:container:auth",
    );
    expect(edge).toBeDefined();
    expect(edge!.source).toBe("c4:container:api");
    expect(edge!.target).toBe("c4:container:auth");
    expect(edge!.label).toBe("uses");
    expect(edge!.kind).toBe("uses");
  });

  it("retains evidence and styles metadata for consumers", () => {
    const bundle = normalizeBundle(assembleBundle(), "fixture://viewer-bundle");

    const evidence = bundle.evidence as { evidence: unknown[] };
    expect(Array.isArray(evidence.evidence)).toBe(true);
    expect(evidence.evidence).toHaveLength(3);

    const styles = bundle.styles as { theme: string };
    expect(styles.theme).toBe("default");

    // Node metadata carries its evidenceRefs.
    const api = bundle.nodes.find((n) => n.id === "c4:container:api")!;
    expect(api.meta?.evidenceRefs).toEqual(["ev:1"]);
  });
});
