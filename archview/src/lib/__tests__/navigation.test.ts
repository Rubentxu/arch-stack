/**
 * Pure unit tests for the navigation model (ADR-062).
 * Covers spec scenarios S1/S2 (zoom target builders), S5 (full zoom
 * chain), S11/S12 (back/forward + truncation).
 */

import { describe, expect, it } from "vitest";
import {
  NavStack,
  c4SelectorFor,
  exportUrlFor,
  zoomTargetFor,
} from "../navigation";

describe("c4SelectorFor", () => {
  it("maps levels to C4 kinds (S5)", () => {
    expect(c4SelectorFor(1, "elm:a")).toBe("c4-context:elm:a");
    expect(c4SelectorFor(2, "elm:a")).toBe("c4-container:elm:a");
    expect(c4SelectorFor(3, "elm:a")).toBe("c4-component:elm:a");
  });

  it("builds an export URL with an encoded selector", () => {
    expect(exportUrlFor("c4-container:elm:a")).toBe(
      "/api/export?selector=c4-container%3Aelm%3Aa",
    );
  });
});

describe("zoomTargetFor", () => {
  it("zooms in from context to container (S1/S5)", () => {
    const t = zoomTargetFor({ id: "elm:a", level: 1 }, "in");
    expect(t).not.toBeNull();
    expect(t!.url).toBe("/api/export?selector=c4-container%3Aelm%3Aa");
    expect(t!.elementId).toBe("elm:a");
    expect(t!.level).toBe(2);
  });

  it("zooms in from container to component (S5)", () => {
    const t = zoomTargetFor({ id: "elm:a", level: 2 }, "in");
    expect(t!.url).toBe("/api/export?selector=c4-component%3Aelm%3Aa");
    expect(t!.level).toBe(3);
  });

  it("does not zoom in beyond component level (S5)", () => {
    expect(zoomTargetFor({ id: "elm:a", level: 3 }, "in")).toBeNull();
    expect(zoomTargetFor({ id: "elm:a" }, "in")).toBeNull();
  });

  it("zooms out to the parent element (S5)", () => {
    const t = zoomTargetFor(
      { id: "elm:c", level: 3, parentId: "elm:p" },
      "out",
    );
    expect(t).not.toBeNull();
    expect(t!.url).toBe("/api/export?selector=c4-container%3Aelm%3Ap");
    expect(t!.elementId).toBe("elm:p");
    expect(t!.level).toBe(2);
  });

  it("offers no zoom out without a parent (S2)", () => {
    expect(zoomTargetFor({ id: "elm:a", level: 1 }, "out")).toBeNull();
    expect(zoomTargetFor({ id: "elm:a" }, "out")).toBeNull();
  });
});

describe("NavStack", () => {
  const entry = (label: string) => ({ url: `test://${label}`, label });

  it("pushes entries and truncates forward history (S12)", () => {
    let s = new NavStack();
    s = s.push(entry("A")).push(entry("B")).push(entry("C"));
    expect(s.length).toBe(3);
    expect(s.index).toBe(2);

    s = s.back(); // at B
    s = s.push(entry("D")); // navigate to D from B
    expect(s.all().map((e) => e.label)).toEqual(["A", "B", "D"]);
    expect(s.index).toBe(2);
  });

  it("supports stable back/forward (S11)", () => {
    let s = new NavStack();
    s = s.push(entry("A")).push(entry("B")).push(entry("C"));
    s = s.back();
    expect(s.current()!.label).toBe("B");
    s = s.back();
    expect(s.current()!.label).toBe("A");
    // back at the start is a no-op
    expect(s.back().current()!.label).toBe("A");
    s = s.forward();
    expect(s.current()!.label).toBe("B");
    s = s.forward();
    expect(s.current()!.label).toBe("C");
    // forward at the end is a no-op
    expect(s.forward().current()!.label).toBe("C");
  });

  it("jumps to an absolute index (breadcrumbs)", () => {
    let s = new NavStack();
    s = s.push(entry("A")).push(entry("B")).push(entry("C"));
    s = s.jumpTo(0);
    expect(s.current()!.label).toBe("A");
    expect(s.forward().current()!.label).toBe("B");
    // out-of-range jumps are no-ops
    expect(s.jumpTo(9)).toBe(s);
    expect(s.jumpTo(-1)).toBe(s);
  });

  it("starts empty", () => {
    const s = new NavStack();
    expect(s.length).toBe(0);
    expect(s.current()).toBeNull();
    expect(s.back()).toBe(s);
    expect(s.forward()).toBe(s);
  });
});
