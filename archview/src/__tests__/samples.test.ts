import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { normalizeBundle } from "../bundle/loader";

/**
 * R4 — Canonical samples.
 *
 * The four C4 samples MUST use viewer-bundle fields
 * (manifest/projection/evidence/styles) and MUST NOT retain legacy
 * element/relation fields. Each sample is loaded
 * through the real normalizer to prove the consuming path works.
 */

const SAMPLES = [
  "c4-context.json",
  "c4-container.json",
  "c4-declared.json",
  "c4-actual.json",
  "c4-semantic-zoom.json",
  "c4-stress-200.json",
  "c4-stress-1k.json",
] as const;

function loadSample(name: string): Record<string, unknown> {
  return JSON.parse(
    readFileSync(
      resolve(import.meta.dirname, "../../public/samples", name),
      "utf8",
    ),
  ) as Record<string, unknown>;
}

describe("canonical C4 samples (R4)", () => {
  for (const name of SAMPLES) {
    it(`loads ${name} as c4 without legacy fields`, () => {
      const raw = loadSample(name);

      // No legacy C4 fields anywhere at the top level.
      expect(raw).not.toHaveProperty("elements");
      expect(raw).not.toHaveProperty("relations");
      expect(raw).not.toHaveProperty("predicate_id");

      // Canonical sections present.
      expect(raw).toHaveProperty("manifest");
      expect(raw).toHaveProperty("projection");
      expect(raw).toHaveProperty("evidence");
      expect(raw).toHaveProperty("styles");
      expect((raw.manifest as Record<string, unknown>).format).toBe(
        "viewer-bundle",
      );

      // Loading through the real normalizer yields c4.
      const bundle = normalizeBundle(raw, `sample://${name}`);
      expect(bundle.rawKind).toBe("c4");

      // Every projection node maps to a level via its type.
      const projection = raw.projection as {
        nodes: { id: string; type: string }[];
      };
      expect(projection.nodes.length).toBeGreaterThan(0);
      for (const node of projection.nodes) {
        const normalized = bundle.nodes.find((n) => n.id === node.id);
        expect(normalized).toBeDefined();
        expect(normalized!.level).toBeGreaterThan(0);
      }
    });
  }
});
