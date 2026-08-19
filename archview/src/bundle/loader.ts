/**
 * Bundle loader — consumes JSON bundles emitted by `archctl`.
 *
 * Supported bundle shapes (per `archctl` schemas):
 * - `call-graph` → `archctl/code/call-graph-report.schema.json`
 * - `sequence` → `archctl/code/sequence-report.schema.json`
 * - `class-diagram` → `archctl/schemas/class-diagram-report.schema.json`
 * - `c4` → canonical `viewer-bundle` (`schemas/diagram-projection.schema.json`)
 *
 * The canonical C4 bundle has four sections: `manifest`, `projection`,
 * `evidence`, and `styles`. The loader rejects incomplete canonical
 * bundles, maps `projection.nodes[].type` → C4 level and
 * `projection.edges[].predicate` → edge label, preserves `evidenceRefs`
 * on node metadata, and derives `parentId` from slash-delimited
 * `canonicalKey` namespaces. Normalization is deterministic: `loadedAt`
 * comes from `manifest.generatedAt`, never from the wall clock.
 */

import type {
  RendererBundle,
  RendererEdge,
  RendererNode,
  SequenceInteraction,
} from "../types";

// Backwards-compatible aliases consumed by views. The renderer consumes
// the shared contract from `types.ts` directly (R3).
export type {
  RendererBundle as GraphBundle,
  RendererEdge as GraphEdge,
  RendererNode as GraphNode,
  SequenceInteraction,
} from "../types";

/**
 * Load a bundle from a JSON path. Works for both `file://` URLs
 * (via the open-file dialog) and `http://localhost:18080/samples/...`
 * (via the dev server).
 */
export async function loadBundle(url: string): Promise<RendererBundle> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(
      `failed to load bundle from ${url}: HTTP ${response.status}`,
    );
  }
  const raw = (await response.json()) as Record<string, unknown>;
  return normalizeBundle(raw, url);
}

/**
 * Normalize an arbitrary bundle JSON into the uniform renderer contract.
 * The detection is based on the presence of `schemaVersion`, the
 * per-domain shape discriminators (`interactions` for sequence,
 * `nodes`+`edges` for code diagrams), and `manifest.format` for the
 * canonical `viewer-bundle`.
 */
export function normalizeBundle(
  raw: Record<string, unknown>,
  source: string,
): RendererBundle {
  const rawKind = detectKind(raw);

  let nodes: RendererNode[] = [];
  let edges: RendererEdge[] = [];
  let interactions: SequenceInteraction[] | undefined;
  let schemaVersion = stringOr(raw.schemaVersion, "unknown");
  let evidence: unknown;
  let styles: unknown;

  switch (rawKind) {
    case "call-graph":
      nodes = ((raw.nodes as Record<string, unknown>[] | undefined) ?? []).map(
        callGraphNodeToNode,
      );
      edges = ((raw.edges as Record<string, unknown>[] | undefined) ?? []).map(
        callGraphEdgeToEdge,
      );
      break;
    case "sequence":
      // Sequence bundles nest interactions → extract callee pairs
      // (for nodes/edges consistency) and preserve raw interactions
      // (for the SequenceView timeline render).
      nodes = extractSequenceNodes(raw);
      edges = extractSequenceEdges(raw);
      interactions = (
        (raw.interactions as Record<string, unknown>[]) ?? []
      ).map(normalizeInteraction);
      break;
    case "class-diagram": {
      // M17.C / F2: real archctl bundles can have edge endpoints
      // that do not match a node id byte-for-byte. The two common
      // shapes are (a) the trailing `:line` differs because it is
      // the line of the reference, not the line of the class
      // declaration, and (b) the endpoint is just the class name.
      // We normalise every edge endpoint to a real node id before
      // handing the bundle to the G6 renderer, which does a strict
      // id-based lookup.
      const rawNodes =
        (raw.nodes as Record<string, unknown>[] | undefined) ?? [];
      const rawEdges =
        (raw.edges as Record<string, unknown>[] | undefined) ?? [];
      nodes = rawNodes.map(classDiagramNodeToNode);
      const index = buildEndpointIndex(nodes);
      edges = rawEdges.map((e) => classDiagramEdgeToEdge(e, index));
      break;
    }
    case "c4":
      // Canonical viewer-bundle: manifest + projection + evidence + styles.
      // R1 — incomplete bundles are rejected with the missing section named.
      {
        const manifest = raw.manifest as Record<string, unknown> | undefined;
        const projection = raw.projection as
          Record<string, unknown> | undefined;
        const missing: string[] = [];
        if (manifest === undefined) missing.push("manifest");
        if (projection === undefined) missing.push("projection");
        if (raw.evidence === undefined) missing.push("evidence");
        if (raw.styles === undefined) missing.push("styles");
        if (missing.length > 0) {
          throw new Error(
            `viewer-bundle is missing required section(s): ${missing.join(", ")}`,
          );
        }

        schemaVersion = stringOr(manifest.schemaVersion, "unknown");
        evidence = raw.evidence;
        styles = raw.styles;

        const rawNodes =
          (projection.nodes as Record<string, unknown>[] | undefined) ?? [];
        const rawEdges =
          (projection.edges as Record<string, unknown>[] | undefined) ?? [];
        nodes = rawNodes.map(c4NodeToNode);
        edges = rawEdges.map(c4EdgeToEdge);
        // R7 — derive parentId from slash-delimited canonicalKey namespaces
        // only when the closest prefix names an existing node.
        nodes = deriveParentIds(nodes);
      }
      break;
    default:
      // Fallback: try `nodes`+`edges` keys with lenient typing.
      nodes = ((raw.nodes as Record<string, unknown>[] | undefined) ?? []).map(
        genericToNode,
      );
      edges = ((raw.edges as Record<string, unknown>[] | undefined) ?? []).map(
        genericToEdge,
      );
  }

  return {
    schemaVersion,
    source,
    // R2 — deterministic preferred: `manifest.generatedAt` if present.
    // M17.C / F3: bundles that are not canonical C4 (e.g. the
    // class-diagram shape produced by `archctl code class-diagram`)
    // have no `manifest` section, so they used to land in the
    // sidebar as `loadedAt: unknown`. The wall clock is fine as a
    // fallback — it is the moment the workbench opened the bundle,
    // which is what the user actually wants to see. R2 only forbids
    // using the wall clock when a deterministic value is available.
    loadedAt:
      stringOr(
        (raw.manifest as Record<string, unknown> | undefined)?.generatedAt,
        "",
      ) || new Date().toISOString(),
    nodes,
    edges,
    rawKind,
    // Item 28: strict bundles open in read-only mode (no source preview,
    // no editor handoff). Only true when manifest.strict === true;
    // absent or false → undefined (regular editable bundle).
    strict:
      rawKind === "c4" &&
      (raw.manifest as Record<string, unknown> | undefined)?.strict === true
        ? true
        : undefined,
    interactions,
    evidence,
    styles,
  };
}

function detectKind(raw: Record<string, unknown>): RendererBundle["rawKind"] {
  if (Array.isArray(raw.interactions)) return "sequence";
  const manifest = raw.manifest as Record<string, unknown> | undefined;
  if (manifest && manifest.format === "viewer-bundle") return "c4";
  if (Array.isArray(raw.nodes) && Array.isArray(raw.edges)) {
    const firstNode = raw.nodes[0] as Record<string, unknown> | undefined;
    if (firstNode && typeof firstNode.kind === "string") {
      // Call-graph nodes are `function`/`method` and ALSO carry a
      // `language` field — the function check must win (bugfix: the
      // language check previously misclassified call-graphs as
      // class-diagrams, so the G6 canvas never mounted).
      if (firstNode.kind === "function" || firstNode.kind === "method") {
        return "call-graph";
      }
      if (
        ["class", "interface", "trait", "enum"].includes(firstNode.kind) ||
        (firstNode.language && typeof firstNode.language === "string")
      ) {
        return "class-diagram";
      }
    }
    return "unknown";
  }
  return "unknown";
}

// -- per-shape normalizers -------------------------------------------------

function callGraphNodeToNode(n: Record<string, unknown>): RendererNode {
  return {
    id: stringOr(n.id, ""),
    label: stringOr(n.name, "?"),
    kind: stringOr(n.kind, "function"),
    language: stringOrUndefined(n.language),
    file: stringOrUndefined(n.file),
    line: numberOrUndefined(n.line),
    meta: { ...n },
  };
}

function callGraphEdgeToEdge(e: Record<string, unknown>): RendererEdge {
  return {
    id: stringOr(e.id, `${e.source}->${e.target}`),
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: stringOrUndefined(e.kind),
    label: stringOrUndefined(e.kind),
    meta: { ...e },
  };
}

function classDiagramNodeToNode(n: Record<string, unknown>): RendererNode {
  return {
    id: stringOr(n.canonical_key, ""),
    label: stringOr(n.name, "?"),
    kind: stringOr(n.kind, "class"),
    language: stringOrUndefined(n.language),
    file: stringOrUndefined(n.file),
    line: numberOrUndefined(n.line),
    meta: { ...n },
  };
}

function classDiagramEdgeToEdge(
  e: Record<string, unknown>,
  index: EndpointIndex,
): RendererEdge {
  return {
    id: stringOr(e.canonical_key, `${e.source}->${e.target}`),
    source: resolveEndpoint(stringOr(e.source, ""), index),
    target: resolveEndpoint(stringOr(e.target, ""), index),
    kind: stringOr(e.predicate, "unknown"),
    label: stringOr(e.predicate, ""),
    meta: { ...e },
  };
}

/**
 * A small lookup index used by the class-diagram normaliser to
 * resolve edge endpoints that do not match a node id byte-for-byte.
 * Three keys are kept per node so the resolution is fast and the
 * intent is obvious at the call site.
 *
 *   byId      : the exact canonical_key
 *   byIdNoLine: the canonical_key with the trailing `:N` stripped
 *   byName    : the short name (last `:` segment)
 */
interface EndpointIndex {
  byId: Map<string, string>;
  byIdNoLine: Map<string, string>;
  byName: Map<string, string>;
}

function buildEndpointIndex(nodes: RendererNode[]): EndpointIndex {
  const byId = new Map<string, string>();
  const byIdNoLine = new Map<string, string>();
  const byName = new Map<string, string>();
  for (const n of nodes) {
    if (n.id) byId.set(n.id, n.id);
    const head = stripTrailingLine(n.id);
    if (head && head !== n.id) byIdNoLine.set(head, n.id);
    if (n.label) byName.set(n.label, n.id);
  }
  return { byId, byIdNoLine, byName };
}

/**
 * Resolve an edge endpoint to a real node id. Tries, in order:
 *   1. exact id match
 *   2. id with the trailing `:N` (line) stripped
 *   3. short name match
 * If none match, returns the original reference untouched so the
 * renderer can still warn about the orphan rather than silently
 * rewiring the edge.
 */
function resolveEndpoint(ref: string, index: EndpointIndex): string {
  if (!ref) return ref;
  if (index.byId.has(ref)) return ref;
  const head = stripTrailingLine(ref);
  if (head && index.byIdNoLine.has(head)) {
    return index.byIdNoLine.get(head)!;
  }
  if (index.byName.has(ref)) return index.byName.get(ref)!;
  return ref;
}

/** Drop the trailing `:N` if `ref` ends in one. */
function stripTrailingLine(ref: string): string {
  const idx = ref.lastIndexOf(":");
  if (idx < 0) return ref;
  const tail = ref.slice(idx + 1);
  if (/^\d+$/.test(tail)) return ref.slice(0, idx);
  return ref;
}

/**
 * Normalize a canonical projection node. `type` is the schema enum and
 * becomes both the renderer `kind` and the C4 level source; `evidenceRefs`
 * stay on `meta` for consumers (R1). `parentId` is derived later from
 * `canonicalKey` namespaces (R7).
 */
function c4NodeToNode(n: Record<string, unknown>): RendererNode {
  const type = stringOr(n.type, "unknown");
  // M81 D2: labelOverride ?? name (R2 + schema 1.1)
  const override = stringOrUndefined(n.labelOverride);
  const name = stringOr(n.name, "?");
  return {
    id: stringOr(n.id, ""),
    label: override ?? name,
    kind: type,
    level: c4LevelForType(type),
    x: numberOrUndefined(n.x),
    y: numberOrUndefined(n.y),
    collapsed: typeof n.collapsed === "boolean" ? n.collapsed : undefined,
    labelOverride: override,
    meta: { ...n },
  };
}

/**
 * Normalize a canonical projection edge. The schema `predicate` becomes
 * both the renderer `kind` and `label` (R1); the raw edge is preserved
 * on `meta`.
 */
function c4EdgeToEdge(e: Record<string, unknown>): RendererEdge {
  const predicate = stringOr(e.predicate, "unknown");
  return {
    id: stringOr(e.id, `${e.source}->${e.target}`),
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: predicate,
    label: predicate,
    meta: { ...e },
  };
}

/**
 * Map a canonical C4 type to its hierarchy level (R1):
 * `context` = 1, `container` = 2, `component` = 3,
 * `dynamic`/`deployment` = 1 (same band as context).
 * Unknown types map to 0 (out-of-band).
 */
export function c4LevelForType(type: string): number {
  switch (type) {
    case "context":
      return 1;
    case "container":
      return 2;
    case "component":
      return 3;
    case "dynamic":
    case "deployment":
      return 1;
    default:
      return 0;
  }
}

/**
 * Derive `parentId` for nodes from slash-delimited `canonicalKey`
 * namespaces (R7). A node's parent is the node whose `canonicalKey`
 * is the closest exact prefix of its own key. Flat keys, keys with no
 * matching prefix, and keys with no ancestor all stay root (no
 * `parentId`). No hierarchy is invented beyond the existing keys.
 */
function deriveParentIds(nodes: RendererNode[]): RendererNode[] {
  const byKey = new Map<string, string>();
  for (const n of nodes) {
    const key = n.meta?.canonicalKey;
    if (typeof key === "string" && key.length > 0) byKey.set(key, n.id);
  }
  return nodes.map((n) => {
    const key = n.meta?.canonicalKey;
    if (typeof key !== "string" || key.length === 0) return n;
    const parts = key.split("/");
    // Walk from the longest prefix down to the shortest exact prefix.
    for (let i = parts.length - 1; i >= 1; i--) {
      const prefix = parts.slice(0, i).join("/");
      const parentId = byKey.get(prefix);
      if (parentId && parentId !== n.id) {
        return { ...n, parentId };
      }
    }
    return n;
  });
}

function extractSequenceNodes(raw: Record<string, unknown>): RendererNode[] {
  const map = new Map<string, RendererNode>();
  for (const i of (raw.interactions as Record<string, unknown>[]) ?? []) {
    for (const side of ["caller", "callee"] as const) {
      const ref = i[side] as Record<string, unknown> | undefined;
      if (!ref) continue;
      const fn = ref.name as string | undefined;
      if (!fn) continue;
      const file = ref.file as string | undefined;
      const key = `${file ?? ""}:${fn}`;
      if (!map.has(key)) {
        map.set(key, {
          id: key,
          label: `${fn}`,
          kind: "function",
          file,
          line: ref.line as number | undefined,
        });
      }
    }
  }
  return [...map.values()];
}

function extractSequenceEdges(raw: Record<string, unknown>): RendererEdge[] {
  const edges: RendererEdge[] = [];
  let order = 0;
  for (const i of (raw.interactions as Record<string, unknown>[]) ?? []) {
    const caller = i.caller as Record<string, unknown> | undefined;
    const callee = i.callee as Record<string, unknown> | undefined;
    if (!caller || !callee) continue;
    const cn = caller.name as string | undefined;
    const dn = callee.name as string | undefined;
    if (!cn || !dn) continue;
    const cf = caller.file as string | undefined;
    const df = callee.file as string | undefined;
    const source = `${cf ?? ""}:${cn}`;
    const target = `${df ?? ""}:${dn}`;
    edges.push({
      id: `${order++}-${source}->${target}`,
      source,
      target,
      kind: stringOrUndefined(i.message_kind),
      label: stringOr(i.label, `${cn}→${dn}`),
      meta: { order: i.order, line: i.line },
    });
  }
  return edges;
}

/** Normalize a raw sequence interaction into the typed shape. */
function normalizeInteraction(i: Record<string, unknown>): SequenceInteraction {
  const caller = (i.caller ?? {}) as Record<string, unknown>;
  const callee = (i.callee ?? {}) as Record<string, unknown>;
  return {
    order: typeof i.order === "number" ? i.order : 0,
    label: stringOrUndefined(i.label),
    message_kind: stringOrUndefined(i.message_kind),
    caller: {
      name: stringOrUndefined(caller.name),
      file: stringOrUndefined(caller.file),
      line: numberOrUndefined(caller.line),
    },
    callee: {
      name: stringOrUndefined(callee.name),
      file: stringOrUndefined(callee.file),
      line: numberOrUndefined(callee.line),
    },
  };
}

function genericToNode(n: Record<string, unknown>): RendererNode {
  return {
    id: stringOr(n.id, ""),
    label: stringOr(n.name, stringOr(n.label, "?")),
    kind: stringOr(n.kind, "unknown"),
    meta: { ...n },
  };
}

function genericToEdge(e: Record<string, unknown>): RendererEdge {
  return {
    id: stringOr(e.id, `${e.source}->${e.target}`),
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: stringOrUndefined(e.kind),
    label: stringOrUndefined(e.kind),
    meta: { ...e },
  };
}

// -- helpers --------------------------------------------------------------

function stringOr(v: unknown, fallback: string): string {
  return typeof v === "string" ? v : fallback;
}

function stringOrUndefined(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

function numberOrUndefined(v: unknown): number | undefined {
  return typeof v === "number" ? v : undefined;
}
