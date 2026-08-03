import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

/**
 * R3 — Renderer boundary.
 *
 * The renderer MUST consume the shared renderer contract from `types.ts`
 * and MUST NOT import from `bundle/loader`. This is enforced statically
 * on the source so the boundary cannot regress silently.
 */

const g6Source = readFileSync(
  resolve(import.meta.dirname, "../renderer/g6.ts"),
  "utf8",
);

describe("renderer boundary (R3)", () => {
  it("resolves the bundle type from types.ts", () => {
    expect(g6Source).toContain("RendererBundle");
    expect(g6Source).toMatch(/from\s+["']\.\.\/types["']/);
  });

  it("has no import targeting bundle/loader", () => {
    expect(g6Source).not.toMatch(/from\s+["'].*bundle\/loader["']/);
    expect(g6Source).not.toContain("../bundle/loader");
  });
});
