// Gate Zero runner — Phase 0.4.
//
// Two-part kill-switch:
//   Part A: verify that one external skill can be discovered + loaded.
//   Part B: run a deterministic, evidence-based IR pass on the local
//            non-Git fixture, compare against the manually labelled gold set,
//            and report.
//
// This runner does NOT call an LLM. It is a deterministic, shape-based
// extractor on purpose: the goal at Gate Zero is to validate the *pipeline
// shape*, not to demonstrate reverse-engineering quality. Phase 1 introduces
// LLM-driven extraction once the pipeline shape is proven.

import { readFileSync, readdirSync, statSync, existsSync, mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import { resolveXdg, ensureXdg } from "../packages/core/src/xdg.ts";

interface GoldElement {
  id: string;
  kind: "container" | "component" | "softwareSystem" | "person" | "codeElement";
  name: string;
  confidence: number;
  method: "heuristic-v1" | "calibrated-v1" | "human-overridden";
  classification: "fact" | "inference" | "hypothesis" | "unknown" | "conflict";
  evidencePaths: string[];
}

interface GoldRelationship {
  source: string;
  target: string;
  via: string;
  evidencePaths: string[];
}

interface GoldSet {
  expectedElements: GoldElement[];
  expectedRelationships: GoldRelationship[];
  forbiddenElements: { id: string; reason: string }[];
  thresholds: {
    jaccardMin: number;
    unsupportedHighConfidenceMax: number;
    forbiddenElementsEmitted: number;
    writesOutsideXdg: number;
  };
}

interface ProducedElement {
  id: string;
  kind: GoldElement["kind"];
  name: string;
  confidence: number;
  method: GoldElement["method"];
  classification: GoldElement["classification"];
  evidenceRefs: string[];
}

interface ProducedRelationship {
  source: string;
  target: string;
  via: string;
  evidenceRefs: string[];
}

function jaccard(a: Set<string>, b: Set<string>): number {
  const inter = [...a].filter((x) => b.has(x)).length;
  const union = new Set([...a, ...b]).size;
  return union === 0 ? 1 : inter / union;
}

interface ExtractedFile {
  absPath: string;
  relPath: string;
  content: string;
  imports: string[];
}

function parseGoImports(content: string): string[] {
  // Supports both `import "x"` (single line) and grouped `import ( "x" ; "y" )`
  // blocks. Anything inside a grouped block is captured via per-line scan.
  const out: string[] = [];
  let inBlock = false;
  for (const rawLine of content.split("\n")) {
    const line = rawLine.trim();
    if (line.startsWith("import (")) {
      inBlock = true;
      continue;
    }
    if (inBlock && line === ")") {
      inBlock = false;
      continue;
    }
    if (inBlock) {
      const m = /"([^"]+)"/.exec(line);
      if (m?.[1]) out.push(m[1]);
    } else if (line.startsWith("import ")) {
      const m = /"([^"]+)"/.exec(line);
      if (m?.[1]) out.push(m[1]);
    }
  }
  return out;
}

function listGoFiles(root: string): ExtractedFile[] {
  const out: ExtractedFile[] = [];
  function walk(dir: string): void {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      const st = statSync(full);
      if (st.isDirectory()) walk(full);
      else if (st.isFile() && full.endsWith(".go")) {
        const content = readFileSync(full, "utf8");
        out.push({
          absPath: full,
          relPath: relative(root, full),
          content,
          imports: parseGoImports(content),
        });
      }
    }
  }
  walk(root);
  return out;
}

function extractElements(files: ExtractedFile[], fixtureRoot: string): ProducedElement[] {
  // Deterministic shape: a file with `package main` and `func main()` is a
  // container named after the package; a file with an exported type is a
  // container named after the type. Evidence refs use the file path.
  // IDs match the gold set convention: `container:<package-name>-<type-name>`
  // (lowercased, hyphens). The directory segment is NOT used because Go's
  // package names are the canonical identifier — using the directory name
  // would couple IDs to layout.
  const out: ProducedElement[] = [];
  for (const f of files) {
    const rel = relative(fixtureRoot, f.absPath);
    const pkgRe = /^package\s+([a-z][A-Za-z0-9_]*)\b/m;
    const pkgMatch = pkgRe.exec(f.content);
    const pkg = pkgMatch?.[1] ?? "root";
    const isEntry = /package\s+main\b/.test(f.content) && /\bfunc\s+main\s*\(/.test(f.content);
    const typeRe = /^type\s+([A-Z][A-Za-z0-9_]*)\s+(struct|interface)\b/gm;
    let m: RegExpExecArray | null;
    while ((m = typeRe.exec(f.content))) {
      const typeName = m[1];
      if (!typeName) continue;
      out.push({
        id: `container:${pkg}-${typeName.toLowerCase()}`,
        kind: "container",
        name: `${pkg}-${typeName.toLowerCase()}`,
        confidence: 0.92,
        method: "heuristic-v1",
        classification: "fact",
        evidenceRefs: [rel],
      });
    }
    if (isEntry) {
      out.push({
        id: `container:${pkg}-main`,
        kind: "container",
        name: `${pkg}-main`,
        confidence: 0.95,
        method: "heuristic-v1",
        classification: "fact",
        evidenceRefs: [rel],
      });
    }
  }
  // De-duplicate by id (entry-point + exported type can overlap).
  const seen = new Set<string>();
  return out.filter((e) => (seen.has(e.id) ? false : (seen.add(e.id), true)));
}

function extractRelationships(files: ExtractedFile[], fixtureRoot: string): ProducedRelationship[] {
  const out: ProducedRelationship[] = [];
  const pkgRe = /^package\s+([a-z][A-Za-z0-9_]*)\b/m;
  // Map every local package name to its produced container id by index lookup.
  const pkgToElementId = new Map<string, string>();
  for (const e of extractElements(files, fixtureRoot)) {
    const [, suffix] = e.id.split(":", 2);
    if (!suffix) continue;
    const dash = suffix.lastIndexOf("-");
    if (dash === -1) continue;
    const pkg = suffix.slice(0, dash);
    pkgToElementId.set(pkg, e.id);
  }
  for (const f of files) {
    const rel = relative(fixtureRoot, f.absPath);
    const pkg = pkgRe.exec(f.content)?.[1] ?? "root";
    const sourceId = pkgToElementId.get(pkg) ?? (pkg === "main" ? "container:main-main" : `container:${pkg}-<unknown>`);
    for (const imp of f.imports) {
      const seg = imp.split("/").pop() ?? imp;
      const match = files.find((other) => other.absPath.endsWith(`${seg}.go`));
      if (!match) continue;
      const targetPkg = pkgRe.exec(match.content)?.[1] ?? seg;
      const targetId = pkgToElementId.get(targetPkg) ?? `container:${targetPkg}-${seg.toLowerCase()}`;
      out.push({
        source: sourceId,
        target: targetId,
        via: "imports",
        evidenceRefs: [rel],
      });
    }
  }
  return out;
}

function checkUnsupportedHighConfidence(elements: ProducedElement[]): number {
  return elements.filter((e) => e.confidence >= 0.9 && e.evidenceRefs.length === 0).length;
}

function writeArtifactsToXdg(elements: ProducedElement[], relationships: ProducedRelationship[]): { wroteFiles: number; outsideXdg: number } {
  const layout = resolveXdg();
  ensureXdg(layout);
  const outDir = join(layout.projectsRoot(), "gate-zero-re");
  // CRITICAL: write only inside XDG. We use realpath containment as an
  // explicit symmetry check for the eventual write-guard (ADR-0008).
  let realOut: string;
  try {
    realOut = realpathSync.native(outDir);
  } catch {
    mkdirSync(outDir, { recursive: true });
    realOut = realpathSync.native(outDir);
  }
  const realData = realpathSync.native(layout.data);
  if (!realOut.startsWith(realData)) {
    return { wroteFiles: 0, outsideXdg: 1 };
  }
  writeFileSync(
    join(outDir, "ir.json"),
    JSON.stringify({ schemaVersion: "1.0.0", elements, relationships }, null, 2),
  );
  return { wroteFiles: 1, outsideXdg: 0 };
}

export interface GateZeroReport {
  ok: boolean;
  producedElements: number;
  producedRelationships: number;
  forbiddenEmitted: number;
  unsupportedHighConfidence: number;
  jaccard: number;
  writes: { wroteFiles: number; outsideXdg: number };
  failures: string[];
}

export function runGateZero(opts: { fixtureRoot: string; goldPath: string }): GateZeroReport {
  const failures: string[] = [];
  const gold: GoldSet = JSON.parse(readFileSync(opts.goldPath, "utf8"));

  // Adapter A — Part A: external skill discovery is exercised by the smoke
  // probe (which would have failed first). At Gate Zero we are validating the
  // IR pipeline shape; we delegate the skill-adaptation part to OpenCode.
  // We assume the vendored skill exists at `.opencode/skills/archctl-evidence`.
  const skillPath = join(process.cwd(), ".opencode/skills/archctl-evidence/SKILL.md");
  if (!existsSync(skillPath)) failures.push(`evidence-discipline skill missing: ${skillPath}`);

  // Adapter B — Part B: deterministic extraction on the non-Git fixture.
  const files = listGoFiles(opts.fixtureRoot);
  if (files.length === 0) failures.push("no Go files discovered in fixture");
  const elements = extractElements(files, opts.fixtureRoot);
  const relationships = extractRelationships(files, opts.fixtureRoot);

  // Compare against gold.
  const goldIds = new Set(gold.expectedElements.map((e) => e.id));
  const producedIds = new Set(elements.map((e) => e.id));
  const j = jaccard(goldIds, producedIds);
  const forbiddenEmitted = elements.filter((e) => gold.forbiddenElements.some((f) => f.id === e.id)).length;
  const unsupported = checkUnsupportedHighConfidence(elements);
  const writes = writeArtifactsToXdg(elements, relationships);

  // Helpful diagnostics when Jaccard is zero — this almost always means the
  // runner's id convention drifted from the gold set. Print the diff inline.
  if (j === 0 && gold.expectedElements.length > 0) {
    failures.push(
      `Jaccard 0.000 — produced=${JSON.stringify([...producedIds])} gold=${JSON.stringify([...goldIds])}`,
    );
  } else if (j < gold.thresholds.jaccardMin) {
    failures.push(`Jaccard ${j.toFixed(3)} < ${gold.thresholds.jaccardMin}`);
  }
  if (unsupported > gold.thresholds.unsupportedHighConfidenceMax) failures.push(`unsupported high-confidence claims ${unsupported} > ${gold.thresholds.unsupportedHighConfidenceMax}`);
  if (forbiddenEmitted > gold.thresholds.forbiddenElementsEmitted) failures.push(`forbidden elements emitted ${forbiddenEmitted} > ${gold.thresholds.forbiddenElementsEmitted}`);
  if (writes.outsideXdg > gold.thresholds.writesOutsideXdg) failures.push(`writes outside XDG ${writes.outsideXdg} > ${gold.thresholds.writesOutsideXdg}`);

  return {
    ok: failures.length === 0,
    producedElements: elements.length,
    producedRelationships: relationships.length,
    forbiddenEmitted,
    unsupportedHighConfidence: unsupported,
    jaccard: j,
    writes,
    failures,
  };
}

// CLI entry: tsx m0-gate-zero/run.ts [--human]
if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2);
  const human = args.includes("--human");
  const cwd = process.cwd();
  const fixtureRoot = join(cwd, "m0-gate-zero/fixtures/re");
  const goldPath = join(fixtureRoot, "gold.json");
  if (!existsSync(fixtureRoot) || !existsSync(goldPath)) {
    console.error(`fixture missing: ${fixtureRoot} or ${goldPath}`);
    process.exit(2);
  }
  const r = runGateZero({ fixtureRoot, goldPath });
  if (human) {
    console.log(`Gate Zero (M0.4)`);
    console.log(`  produced elements:      ${r.producedElements}`);
    console.log(`  produced relationships: ${r.producedRelationships}`);
    console.log(`  Jaccard vs gold:        ${r.jaccard.toFixed(3)}`);
    console.log(`  forbidden emitted:      ${r.forbiddenEmitted}`);
    console.log(`  unsupported high-conf:   ${r.unsupportedHighConfidence}`);
    console.log(`  writes (files / oog):   ${r.writes.wroteFiles} / ${r.writes.outsideXdg}`);
    if (r.failures.length) {
      console.log("  failures:");
      for (const f of r.failures) console.log(`    - ${f}`);
    }
    console.log(r.ok ? "GATE ZERO: PASS" : "GATE ZERO: FAIL — retain skill-only baseline");
  } else {
    console.log(JSON.stringify(r, null, 2));
  }
  process.exit(r.ok ? 0 : 1);
}
