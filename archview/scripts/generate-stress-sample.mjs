// Generate a deterministic stress sample for perf measurement.
//
// Shape: canonical viewer-bundle (c4), so the loader produces a
// RendererBundle. Seed is fixed for reproducibility (per testing
// principles: nada de randomness sin seed fija).
//
// Topology:
//   - 1 hub system:core (context)
//   - 10 sibling systems (context)
//   - 10 containers per system (110 containers total: 1 hub + 9*10 = 91? No: 10 systems * 10 containers = 100 containers)
//   - 10 components per container (1000 components total)
//   - Total: 1 + 10 + 100 + 1000 = 1111 nodes
//
// Note: 1111 nodes stresses the canvas (G6) and Sidebar. The dev
// server in headless mode can hang on >500 nodes because layout +
// canvas paint of 1k+ elements blocks the main thread. This sample
// is intended as a manual stress marker; perf-budget automation
// uses a smaller 200-node variant generated below (`c4-stress-200`).
//
// Edges:
//   - hub system:core → every other context (10 edges)
//   - every container → its parent context (110 edges)
//   - every component → its parent container (1000 edges)
//   - extra cross-component edges: ~2 per component (2000 edges, pseudo-random with seed)
//   - the HUB NODE has ~500 incoming edges from a mix of containers and
//     components — that's the case the Sidebar relations list will need
//     to virtualize.
//
// Total: 1111 nodes, ~3120 edges. ~500 edges on the hub node = the
// Sidebar relations list stress.

import { writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const out = resolve(__dirname, "../public/samples/c4-stress-1k.json");

// Mulberry32 PRNG: 32-bit, fast, deterministic.
function mulberry32(seed) {
  let s = seed >>> 0;
  return function () {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rand = mulberry32(0xc4dec0de);

const N_CONTEXTS = 10;
const N_CONTAINERS_PER_CONTEXT = 10;
const N_COMPONENTS_PER_CONTAINER = 10;
const N_CROSS_EDGES_PER_COMPONENT = 2;
const N_HUB_INCOMING = 500;

// Smaller variant for CI / headless perf measurement.
const SMALL_N_CONTEXTS = 2;
const SMALL_N_CONTAINERS_PER_CONTEXT = 5;
const SMALL_N_COMPONENTS_PER_CONTAINER = 20;
const SMALL_N_HUB_INCOMING = 100;

const ctxHubId = "system:core";
const nodes = [];
const edges = [];

// 1) Hub context.
nodes.push({
  id: ctxHubId,
  type: "context",
  name: "core",
  description: "Hub system — stress test target (Sidebar relations list)",
  canonicalKey: "core",
});

// 2) Sibling contexts.
const siblingContextIds = [];
for (let i = 0; i < N_CONTEXTS; i++) {
  const id = `system:svc-${i.toString().padStart(2, "0")}`;
  siblingContextIds.push(id);
  nodes.push({
    id,
    type: "context",
    name: `svc-${i.toString().padStart(2, "0")}`,
    description: `Sibling system ${i}`,
    canonicalKey: `svc-${i.toString().padStart(2, "0")}`,
  });
}

// 3) Hub edges: core → every sibling.
for (const target of siblingContextIds) {
  edges.push({
    id: `rel:hub-${target}`,
    source: ctxHubId,
    target,
    predicate: "depends-on",
  });
}

// 4) Containers (10 per context, including hub).
const allContainerIds = [];
for (const ctxId of [ctxHubId, ...siblingContextIds]) {
  for (let j = 0; j < N_CONTAINERS_PER_CONTEXT; j++) {
    const cid = `container:${ctxId.split(":")[1]}-ct-${j.toString().padStart(2, "0")}`;
    allContainerIds.push(cid);
    nodes.push({
      id: cid,
      type: "container",
      name: `${ctxId.split(":")[1]}-ct-${j.toString().padStart(2, "0")}`,
      description: `Container ${j} of ${ctxId}`,
      parent: ctxId,
      canonicalKey: `${ctxId.split(":")[1]}/ct-${j.toString().padStart(2, "0")}`,
    });
    edges.push({
      id: `rel:ctx-${cid}`,
      source: ctxId,
      target: cid,
      predicate: "contains",
    });
  }
}

// 5) Components (10 per container).
const allComponentIds = [];
for (const containerId of allContainerIds) {
  for (let k = 0; k < N_COMPONENTS_PER_CONTAINER; k++) {
    const kstr = k.toString().padStart(2, "0");
    const cmid = `component:${containerId.split(":")[1]}-cp-${kstr}`;
    allComponentIds.push(cmid);
    nodes.push({
      id: cmid,
      type: "component",
      name: `${containerId.split(":")[1]}-cp-${kstr}`,
      description: `Component ${k} of ${containerId}`,
      parent: containerId,
      canonicalKey: `${containerId.split(":")[1]}/cp-${kstr}`,
    });
    edges.push({
      id: `rel:ct-${cmid}`,
      source: containerId,
      target: cmid,
      predicate: "contains",
    });
  }
}

// 6) Cross-component edges (pseudo-random, deterministic). Each component
//    gets ~2 random "calls" edges to other components in different containers.
let crossEdgeCounter = 0;
for (const cmid of allComponentIds) {
  for (let n = 0; n < N_CROSS_EDGES_PER_COMPONENT; n++) {
    const target =
      allComponentIds[Math.floor(rand() * allComponentIds.length)];
    if (target === cmid) continue;
    crossEdgeCounter++;
    edges.push({
      id: `rel:cross-${crossEdgeCounter}`,
      source: cmid,
      target,
      predicate: "calls",
    });
  }
}

// 7) Hub incoming edges: 500 components and containers point to core.
for (let h = 0; h < N_HUB_INCOMING; h++) {
  const source =
    h % 2 === 0
      ? allContainerIds[Math.floor(rand() * allContainerIds.length)]
      : allComponentIds[Math.floor(rand() * allComponentIds.length)];
  edges.push({
    id: `rel:hub-in-${h}`,
    source,
    target: ctxHubId,
    predicate: "reports-to",
  });
}

const bundle = {
  manifest: {
    schemaVersion: "1.0.0",
    format: "viewer-bundle",
    viewSelector: "context:*/container:*/component:*",
    baseRevision: "blake3:deadbeefcafef00ddeadbeefcafef00ddeadbeefcafef00ddeadbeefcafef00d",
    generatedAt: "2026-08-19T23:00:00Z",
    elementCount: nodes.length,
    edgeCount: edges.length,
    evidenceCount: 0,
    stress: {
      hub: ctxHubId,
      hubIncomingEdgeCount: N_HUB_INCOMING,
      nodeCount: nodes.length,
      edgeCount: edges.length,
    },
  },
  projection: { nodes, edges },
  evidence: { byId: {} },
  styles: { defaults: {} },
};

writeFileSync(out, JSON.stringify(bundle, null, 2));

console.log(
  `wrote ${nodes.length} nodes / ${edges.length} edges to ${out}`,
);
console.log(`hub: ${ctxHubId} with ${N_HUB_INCOMING} incoming edges`);

// ── Small variant for headless perf CI ───────────────────────────────
function generateSmall() {
  const rng = mulberry32(0x5c4a11);
  const ctxHubIdLocal = "system:core";
  const smallNodes = [];
  const smallEdges = [];
  smallNodes.push({
    id: ctxHubIdLocal,
    type: "context",
    name: "core",
    description: "Hub system — stress test target (Sidebar relations list)",
    canonicalKey: "core",
  });
  const siblingCtx = [];
  for (let i = 0; i < SMALL_N_CONTEXTS; i++) {
    const id = `system:svc-${i.toString().padStart(2, "0")}`;
    siblingCtx.push(id);
    smallNodes.push({
      id,
      type: "context",
      name: `svc-${i.toString().padStart(2, "0")}`,
      description: `Sibling system ${i}`,
      canonicalKey: `svc-${i.toString().padStart(2, "0")}`,
    });
  }
  for (const target of siblingCtx) {
    smallEdges.push({
      id: `rel:hub-${target}`,
      source: ctxHubIdLocal,
      target,
      predicate: "depends-on",
    });
  }
  const allContainers = [];
  for (const ctxId of [ctxHubIdLocal, ...siblingCtx]) {
    for (let j = 0; j < SMALL_N_CONTAINERS_PER_CONTEXT; j++) {
      const cid = `container:${ctxId.split(":")[1]}-ct-${j.toString().padStart(2, "0")}`;
      allContainers.push(cid);
      smallNodes.push({
        id: cid,
        type: "container",
        name: `${ctxId.split(":")[1]}-ct-${j.toString().padStart(2, "0")}`,
        description: `Container ${j} of ${ctxId}`,
        parent: ctxId,
        canonicalKey: `${ctxId.split(":")[1]}/ct-${j.toString().padStart(2, "0")}`,
      });
      smallEdges.push({
        id: `rel:ctx-${cid}`,
        source: ctxId,
        target: cid,
        predicate: "contains",
      });
    }
  }
  const allComponents = [];
  for (const containerId of allContainers) {
    for (let k = 0; k < SMALL_N_COMPONENTS_PER_CONTAINER; k++) {
      const kstr = k.toString().padStart(2, "0");
      const cmid = `component:${containerId.split(":")[1]}-cp-${kstr}`;
      allComponents.push(cmid);
      smallNodes.push({
        id: cmid,
        type: "component",
        name: `${containerId.split(":")[1]}-cp-${kstr}`,
        description: `Component ${k} of ${containerId}`,
        parent: containerId,
        canonicalKey: `${containerId.split(":")[1]}/cp-${kstr}`,
      });
      smallEdges.push({
        id: `rel:ct-${cmid}`,
        source: containerId,
        target: cmid,
        predicate: "contains",
      });
    }
  }
  let xc = 0;
  for (const cmid of allComponents) {
    for (let n = 0; n < N_CROSS_EDGES_PER_COMPONENT; n++) {
      const target =
        allComponents[Math.floor(rng() * allComponents.length)];
      if (target === cmid) continue;
      xc++;
      smallEdges.push({
        id: `rel:cross-${xc}`,
        source: cmid,
        target,
        predicate: "calls",
      });
    }
  }
  for (let h = 0; h < SMALL_N_HUB_INCOMING; h++) {
    const source =
      h % 2 === 0
        ? allContainers[Math.floor(rng() * allContainers.length)]
        : allComponents[Math.floor(rng() * allComponents.length)];
    smallEdges.push({
      id: `rel:hub-in-${h}`,
      source,
      target: ctxHubIdLocal,
      predicate: "reports-to",
    });
  }
  return {
    manifest: {
      schemaVersion: "1.0.0",
      format: "viewer-bundle",
      viewSelector: "context:*/container:*/component:*",
      baseRevision: "blake3:c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee",
      generatedAt: "2026-08-19T23:00:00Z",
      elementCount: smallNodes.length,
      edgeCount: smallEdges.length,
      evidenceCount: 0,
      stress: {
        hub: ctxHubIdLocal,
        hubIncomingEdgeCount: SMALL_N_HUB_INCOMING,
        nodeCount: smallNodes.length,
        edgeCount: smallEdges.length,
        purpose: "headless perf measurement (small variant)",
      },
    },
    projection: { nodes: smallNodes, edges: smallEdges },
    evidence: { byId: {} },
    styles: { defaults: {} },
  };
}

const smallOut = resolve(__dirname, "../public/samples/c4-stress-200.json");
const smallBundle = generateSmall();
writeFileSync(smallOut, JSON.stringify(smallBundle, null, 2));
console.log(
  `wrote ${smallBundle.projection.nodes.length} nodes / ${smallBundle.projection.edges.length} edges to ${smallOut}`,
);
console.log(
  `hub: ${smallBundle.manifest.stress.hub} with ${smallBundle.manifest.stress.hubIncomingEdgeCount} incoming edges`,
);
