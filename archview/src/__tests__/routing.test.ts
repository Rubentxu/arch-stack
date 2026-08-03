import { describe, it, expect } from "vitest";
import { resolveView } from "../routing";

/**
 * R3 — Collision-free seven-view routing matrix.
 *
 * The manual router must expose exactly these specialized outcomes:
 * C4, Sequence, Class diagram, Call graph, Package, Impact, and
 * (via drift mode) Drift. `resolveView` is the pure discriminant that
 * App uses to pick the rendered view, so the matrix is testable
 * without a DOM.
 */

describe("resolveView routing matrix (R3)", () => {
  it("maps each bundle kind to exactly one specialized view", () => {
    expect(resolveView("c4", "impact")).toBe("c4");
    expect(resolveView("sequence", "impact")).toBe("sequence");
    expect(resolveView("class-diagram", "impact")).toBe("class-diagram");
  });

  it("defaults call-graph bundles to Impact", () => {
    expect(resolveView("call-graph", "impact")).toBe("impact");
  });

  it("routes the call-graph selector to Call graph and Package", () => {
    expect(resolveView("call-graph", "call-graph")).toBe("call-graph");
    expect(resolveView("call-graph", "package")).toBe("package");
  });

  it("falls back to generic for unknown kinds without collision", () => {
    expect(resolveView("unknown", "impact")).toBe("generic");
    expect(resolveView("bogus", "package")).toBe("generic");
  });
});
