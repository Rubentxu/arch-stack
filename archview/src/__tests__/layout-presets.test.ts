/**
 * Tests for layout presets — the ELK layered configs that the
 * views used to pass as dagre options.
 *
 * We assert the direction + spacing values that map directly to
 * the previous per-view dagre configs. If a preset's direction
 * changes, the corresponding view's visual flips — we want a
 * test to catch that.
 */

import { describe, it, expect } from "vitest";
import {
  TB_LAYERED,
  LR_LAYERED,
  DEFAULT_LAYOUT,
} from "../renderer/layout-presets";

describe("layout presets", () => {
  it("TB_LAYERED lays out top-to-bottom", () => {
    expect(TB_LAYERED["elk.algorithm"]).toBe("layered");
    expect(TB_LAYERED["elk.direction"]).toBe("DOWN");
  });

  it("LR_LAYERED lays out left-to-right", () => {
    expect(LR_LAYERED["elk.algorithm"]).toBe("layered");
    expect(LR_LAYERED["elk.direction"]).toBe("RIGHT");
  });

  it("TB and LR share the same spacing values", () => {
    // The visual density of the workbench should not change just
    // because the user flipped direction. We pin the spacing here
    // so a refactor that breaks one but not the other fails the
    // test loudly.
    expect(TB_LAYERED["elk.layered.spacing.nodeNodeBetweenLayers"]).toBe(
      LR_LAYERED["elk.layered.spacing.nodeNodeBetweenLayers"],
    );
    expect(TB_LAYERED["elk.layered.spacing.nodeNode"]).toBe(
      LR_LAYERED["elk.layered.spacing.nodeNode"],
    );
  });

  it("DEFAULT_LAYOUT is the top-to-bottom preset", () => {
    // The renderer falls back to DEFAULT_LAYOUT when a view does
    // not pass options. Pin the default to TB so the default view
    // is C4-shaped (top-down tree).
    expect(DEFAULT_LAYOUT).toBe(TB_LAYERED);
  });

  it("every preset declares a non-empty padding", () => {
    // ELK requires `[T,R,B,L]` syntax. Catch typos like
    // `"[top=24]"` (missing right/bottom/left).
    for (const preset of [TB_LAYERED, LR_LAYERED]) {
      const padding = preset["elk.padding"];
      expect(padding).toBeDefined();
      expect(padding).toMatch(/^\[.*,.*,.*,.*\]$/);
    }
  });
});
