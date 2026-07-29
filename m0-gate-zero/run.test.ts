import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runGateZero } from "./run.ts";

function setupFixture(): { fixtureRoot: string; goldPath: string } {
  const root = mkdtempSync(join(tmpdir(), "archctl-gz-"));
  const fixtureRoot = join(root, "re");
  mkdirSync(join(fixtureRoot, "internal/orders"), { recursive: true });
  mkdirSync(join(fixtureRoot, "internal/store"), { recursive: true });
  writeFileSync(join(fixtureRoot, "main.go"), `package main\n\nfunc main() {}\n`);
  writeFileSync(join(fixtureRoot, "internal/orders/service.go"), `package orders\n\ntype Service struct{}\n`);
  writeFileSync(join(fixtureRoot, "internal/orders/repo.go"), `package orders\n\ntype Repo struct{}\n`);
  writeFileSync(join(fixtureRoot, "internal/store/sqlite.go"), `package store\n\ntype SQLite struct{}\n`);
  writeFileSync(join(fixtureRoot, "README.md"), "# Tiny fixture\n");
  const goldPath = join(fixtureRoot, "gold.json");
  writeFileSync(
    goldPath,
    JSON.stringify({
      expectedElements: [
        { id: "container:main-main", kind: "container", name: "main-main", confidence: 0.95, method: "heuristic-v1", classification: "fact", evidencePaths: ["main.go"] },
        { id: "container:orders-service", kind: "container", name: "orders-service", confidence: 0.92, method: "heuristic-v1", classification: "fact", evidencePaths: ["internal/orders/service.go"] },
        { id: "container:orders-repo", kind: "container", name: "orders-repo", confidence: 0.92, method: "heuristic-v1", classification: "fact", evidencePaths: ["internal/orders/repo.go"] },
        { id: "container:store-sqlite", kind: "container", name: "store-sqlite", confidence: 0.92, method: "heuristic-v1", classification: "fact", evidencePaths: ["internal/store/sqlite.go"] },
      ],
      expectedRelationships: [],
      forbiddenElements: [
        { id: "container:metrics-exporter", reason: "README speculates but no source references it." },
      ],
      thresholds: { jaccardMin: 0.95, unsupportedHighConfidenceMax: 0, forbiddenElementsEmitted: 0, writesOutsideXdg: 0 },
    }),
  );
  return { fixtureRoot, goldPath };
}

test("Gate Zero passes on the canonical fixture", () => {
  const { fixtureRoot, goldPath } = setupFixture();
  const r = runGateZero({ fixtureRoot, goldPath });
  assert.equal(r.ok, true, `unexpected failures: ${JSON.stringify(r.failures)}`);
  assert.equal(r.unsupportedHighConfidence, 0);
  assert.equal(r.forbiddenEmitted, 0);
  assert.equal(r.writes.outsideXdg, 0);
  assert.ok(r.jaccard >= 0.95);
});

test("Gate Zero fails loudly when evidence is missing (high confidence unsupported)", () => {
  const { fixtureRoot, goldPath } = setupFixture();
  // Delete the sqlite file → sqlite element is no longer produced.
  rmSync(join(fixtureRoot, "internal/store/sqlite.go"));
  const r = runGateZero({ fixtureRoot, goldPath });
  assert.equal(r.ok, false);
  assert.ok(r.jaccard < 0.95, "Jaccard must drop below threshold when an element disappears");
});

test("Gate Zero enforces data-not-instructions: README-only claims do not produce elements", () => {
  const { fixtureRoot, goldPath } = setupFixture();
  writeFileSync(
    join(fixtureRoot, "README.md"),
    "# Tiny fixture\n\nA future metrics-exporter sidecar will be wired up.\n",
  );
  const r = runGateZero({ fixtureRoot, goldPath });
  assert.equal(r.forbiddenEmitted, 0, "forbidden element must not be produced");
  assert.equal(r.ok, true);
});

test("Gate Zero writes only inside XDG (realpath containment)", () => {
  const { fixtureRoot, goldPath } = setupFixture();
  const r = runGateZero({ fixtureRoot, goldPath });
  assert.equal(r.writes.outsideXdg, 0, "writes must be confined to XDG");
});
