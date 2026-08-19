/**
 * M17.C / F2 — Real bundle fixture with edge / node id mismatch.
 *
 * The `archctl/tests/fixtures/class-diagram/gold.json` bundle
 * is what the production archctl emits. It has a known
 * inconsistency: node ids end with `:line` from the AST
 * discovery, but edge `source`/`target` sometimes end with a
 * different `:line` (the line where the relation is written,
 * not the line where the target class is declared). For
 * example:
 *
 *   node Base1:3  ← declared at line 3
 *   edge target "python:…:class:Base1:0"  ← reference at line 0
 *
 * Before M17.C the loader stored `source`/`target` verbatim,
 * so the G6 renderer could not resolve the edge. The fix is to
 * rewrite each edge endpoint to an existing node id (or drop
 * the edge) so the renderer never sees an orphan reference.
 */
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { normalizeBundle } from "../loader";

const FIXTURE_PATH = resolve(
  import.meta.dirname,
  "fixtures/real-bundle-mismatch.json",
);

describe("real bundle id mismatch (M17.C / F2)", () => {
  it("loads and exposes both nodes and both edges", () => {
    const raw = JSON.parse(readFileSync(FIXTURE_PATH, "utf8")) as Record<
      string,
      unknown
    >;
    const bundle = normalizeBundle(raw, "fixture://real-bundle-mismatch");

    expect(bundle.rawKind).toBe("class-diagram");
    const names = bundle.nodes.map((n) => n.label).sort();
    expect(names).toEqual(["Base1", "Base2", "Derived"]);
  });

  it("every edge endpoint is a real node id (the renderer's only lookup key)", () => {
    const raw = JSON.parse(readFileSync(FIXTURE_PATH, "utf8")) as Record<
      string,
      unknown
    >;
    const bundle = normalizeBundle(raw, "fixture://real-bundle-mismatch");

    const ids = new Set(bundle.nodes.map((n) => n.id));
    expect(ids.size).toBe(3);
    // The three nodes from the fixture.
    expect(ids.has("python:python_sample.py:class:Base1:3")).toBe(true);
    expect(ids.has("python:python_sample.py:class:Base2:8")).toBe(true);
    expect(ids.has("python:python_sample.py:class:Derived:13")).toBe(true);

    // Edges: their source/target must be in the node-id set,
    // because the G6 renderer only does id-based lookups.
    for (const e of bundle.edges) {
      expect(
        ids.has(e.source),
        `edge ${e.id} source=${e.source} is not a known node id`,
      ).toBe(true);
      expect(
        ids.has(e.target),
        `edge ${e.id} target=${e.target} is not a known node id`,
      ).toBe(true);
    }
  });

  it("non-canonical bundles expose a wall-clock loadedAt (M17.C / F3)", () => {
    // The class-diagram shape has no `manifest` section, so the
    // sidebar used to render `loadedAt: unknown`. Falling back to
    // the wall clock is the right behaviour here: it is the moment
    // the workbench opened the bundle.
    const raw = JSON.parse(readFileSync(FIXTURE_PATH, "utf8")) as Record<
      string,
      unknown
    >;
    const before = Date.now();
    const bundle = normalizeBundle(raw, "fixture://real-bundle-mismatch");
    const after = Date.now();

    // Must be a parseable ISO timestamp between `before` and `after`.
    const ms = Date.parse(bundle.loadedAt);
    expect(Number.isFinite(ms)).toBe(true);
    expect(ms).toBeGreaterThanOrEqual(before);
    expect(ms).toBeLessThanOrEqual(after);
    expect(bundle.loadedAt).not.toBe("unknown");
  });
});

describe("loadedAt fallback (M17.C / F3, R2 preserved)", () => {
  it("canonical C4 bundle still uses manifest.generatedAt (R2)", () => {
    // R2 forbids the wall clock when a deterministic value is
    // available. The canonical viewer-bundle has `manifest.generatedAt`
    // and that value must win over `new Date()`.
    const raw = {
      manifest: {
        format: "viewer-bundle",
        schemaVersion: "1.1",
        generatedAt: "2026-01-15T10:00:00.000Z",
      },
      projection: { nodes: [], edges: [] },
      evidence: {},
      styles: {},
    } as Record<string, unknown>;

    const bundle = normalizeBundle(raw, "fixture://c4");
    expect(bundle.loadedAt).toBe("2026-01-15T10:00:00.000Z");
  });

  it("call-graph shape also falls back to wall clock (no manifest section)", () => {
    const raw = {
      schemaVersion: "1.0",
      project: "x",
      nodes: [{ id: "fn:a", name: "a", kind: "function", language: "rust" }],
      edges: [],
      errors: [],
    } as Record<string, unknown>;
    const before = Date.now();
    const bundle = normalizeBundle(raw, "fixture://callgraph");
    const after = Date.now();

    const ms = Date.parse(bundle.loadedAt);
    expect(Number.isFinite(ms)).toBe(true);
    expect(ms).toBeGreaterThanOrEqual(before);
    expect(ms).toBeLessThanOrEqual(after);
  });
});
