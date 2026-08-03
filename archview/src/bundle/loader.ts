/**
 * Bundle loader — consumes JSON bundles emitted by `archctl`.
 *
 * Supported bundle shapes (per `archctl` schemas):
 * - `call-graph` → `archctl/code/call-graph-report.schema.json`
 * - `sequence` → `archctl/code/sequence-report.schema.json`
 * - `class-diagram` → `archctl/schemas/class-diagram-report.schema.json`
 * - `c4` → `archctl/diagram/projection.schema.json`
 *
 * The loader is intentionally schema-tolerant: it accepts any
 * JSON object that has either `nodes`+`edges` or `elements`+`relations`
 * shape, and normalizes to a uniform `GraphBundle` for the renderer.
 */

export interface GraphNode {
  id: string;
  label: string;
  kind: string;
  language?: string;
  file?: string;
  line?: number;
  /**
   * C4 hierarchy level (1-4). Derived from `kind` for C4 bundles:
   * 1 = Person / SoftwareSystem (Context level)
   * 2 = Container
   * 3 = Component
   * 4 = Code (defer to M17.5+)
   * Undefined for non-C4 bundles.
   */
  level?: number;
  /** Parent element id (for C4 drill-down). */
  parentId?: string;
  meta?: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  kind?: string;
  meta?: Record<string, unknown>;
}

export interface SequenceInteraction {
  order: number;
  label?: string;
  message_kind?: string;
  caller: { name?: string; file?: string; line?: number };
  callee: { name?: string; file?: string; line?: number };
}

export interface GraphBundle {
  schemaVersion: string;
  source: string;
  loadedAt: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
  /** Bundle shape that produced this normalized bundle. */
  rawKind: "call-graph" | "sequence" | "class-diagram" | "c4" | "unknown";
  /**
   * Original interactions, only populated for `rawKind === "sequence"`.
   * SequenceView uses this to render lifelines + arrows in time order.
   */
  interactions?: SequenceInteraction[];
}

/**
 * Load a bundle from a JSON path. Works for both `file://` URLs
 * (via the open-file dialog) and `http://localhost:18080/samples/...`
 * (via the dev server).
 */
export async function loadBundle(url: string): Promise<GraphBundle> {
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
 * Normalize an arbitrary bundle JSON into the uniform `GraphBundle`
 * shape. The detection is based on the presence of `schemaVersion`
 * and the per-domain shape discriminators (`interactions` for
 * sequence, `nodes`+`edges` for the rest).
 */
export function normalizeBundle(
  raw: Record<string, unknown>,
  source: string,
): GraphBundle {
  const schemaVersion = stringOr(raw.schemaVersion, "unknown");
  const rawKind = detectKind(raw);

  let nodes: GraphNode[] = [];
  let edges: GraphEdge[] = [];
  let interactions: SequenceInteraction[] | undefined;

  switch (rawKind) {
    case "call-graph":
      nodes = (raw.nodes as Record<string, unknown>[] | undefined ?? []).map(
        callGraphNodeToNode,
      );
      edges = (raw.edges as Record<string, unknown>[] | undefined ?? []).map(
        callGraphEdgeToEdge,
      );
      break;
    case "sequence":
      // Sequence bundles nest interactions → extract callee pairs
      // (for nodes/edges consistency) and preserve raw interactions
      // (for the SequenceView timeline render).
      nodes = extractSequenceNodes(raw);
      edges = extractSequenceEdges(raw);
      interactions = (raw.interactions as Record<string, unknown>[] ?? []).map(
        normalizeInteraction,
      );
      break;
    case "class-diagram":
      nodes = (raw.nodes as Record<string, unknown>[] | undefined ?? []).map(
        classDiagramNodeToNode,
      );
      edges = (raw.edges as Record<string, unknown>[] | undefined ?? []).map(
        classDiagramEdgeToEdge,
      );
      break;
    case "c4":
      // C4 bundle uses `elements` + `relations` per the projection schema.
      nodes = (raw.elements as Record<string, unknown>[] | undefined ?? []).map(
        c4ElementToNode,
      );
      edges = (raw.relations as Record<string, unknown>[] | undefined ?? []).map(
        c4RelationToEdge,
      );
      break;
    default:
      // Fallback: try `nodes`+`edges` keys with lenient typing.
      nodes = (raw.nodes as Record<string, unknown>[] | undefined ?? []).map(
        genericToNode,
      );
      edges = (raw.edges as Record<string, unknown>[] | undefined ?? []).map(
        genericToEdge,
      );
  }

  return {
    schemaVersion,
    source,
    loadedAt: new Date().toISOString(),
    nodes,
    edges,
    rawKind,
    interactions,
  };
}

function detectKind(raw: Record<string, unknown>): GraphBundle["rawKind"] {
  if (Array.isArray(raw.interactions)) return "sequence";
  if (Array.isArray(raw.nodes) && Array.isArray(raw.edges)) {
    const firstNode = raw.nodes[0] as Record<string, unknown> | undefined;
    if (firstNode && typeof firstNode.kind === "string") {
      if (
        ["class", "interface", "trait", "enum"].includes(firstNode.kind) ||
        (firstNode.language && typeof firstNode.language === "string")
      ) {
        return "class-diagram";
      }
      if (firstNode.kind === "function" || firstNode.kind === "method") {
        return "call-graph";
      }
    }
    return "unknown";
  }
  if (Array.isArray(raw.elements) && Array.isArray(raw.relations)) {
    return "c4";
  }
  return "unknown";
}

// -- per-shape normalizers -------------------------------------------------

function callGraphNodeToNode(n: Record<string, unknown>): GraphNode {
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

function callGraphEdgeToEdge(e: Record<string, unknown>): GraphEdge {
  return {
    id: stringOr(e.id, `${e.source}->${e.target}`),
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: stringOrUndefined(e.kind),
    label: stringOrUndefined(e.kind),
    meta: { ...e },
  };
}

function classDiagramNodeToNode(n: Record<string, unknown>): GraphNode {
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

function classDiagramEdgeToEdge(e: Record<string, unknown>): GraphEdge {
  return {
    id: stringOr(e.canonical_key, `${e.source}->${e.target}`),
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: stringOr(e.predicate, "unknown"),
    label: stringOr(e.predicate, ""),
    meta: { ...e },
  };
}

function c4ElementToNode(e: Record<string, unknown>): GraphNode {
  const kind = stringOr(e.kind, "Element");
  return {
    id: stringOr(e.id, ""),
    label: stringOr(e.name, "?"),
    kind,
    language: undefined,
    file: undefined,
    line: undefined,
    level: c4LevelForKind(kind),
    parentId: stringOrUndefined(e.parent),
    meta: { ...e },
  };
}

function c4RelationToEdge(e: Record<string, unknown>): GraphEdge {
  return {
    id: `${e.source}->${e.target}`,
    source: stringOr(e.source, ""),
    target: stringOr(e.target, ""),
    kind: stringOr(e.predicate_id, "unknown"),
    label: stringOr(e.predicate_id, ""),
    meta: { ...e },
  };
}

/**
 * Map a C4 element kind to its hierarchy level (1-4).
 * Context = 1 (Person, SoftwareSystem)
 * Container = 2
 * Component = 3
 * Code = 4 (defer to M17.5+ for code-level rendering)
 * Unknown / Workspace = 0 (treat as out-of-band)
 */
export function c4LevelForKind(kind: string): number {
  const k = kind.toLowerCase();
  if (k === "person" || k === "softwaresystem" || k === "system") return 1;
  if (k === "container" || k === "containerinstance" || k.endsWith(":container")) return 2;
  if (k === "component" || k === "componentinstance" || k.endsWith(":component")) return 3;
  if (k === "code" || k === "codeinstance" || k.endsWith(":code")) return 4;
  return 0;
}

function extractSequenceNodes(raw: Record<string, unknown>): GraphNode[] {
  const map = new Map<string, GraphNode>();
  for (const i of raw.interactions as Record<string, unknown>[] ?? []) {
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

function extractSequenceEdges(raw: Record<string, unknown>): GraphEdge[] {
  const edges: GraphEdge[] = [];
  let order = 0;
  for (const i of raw.interactions as Record<string, unknown>[] ?? []) {
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

function genericToNode(n: Record<string, unknown>): GraphNode {
  return {
    id: stringOr(n.id, ""),
    label: stringOr(n.name, stringOr(n.label, "?")),
    kind: stringOr(n.kind, "unknown"),
    meta: { ...n },
  };
}

function genericToEdge(e: Record<string, unknown>): GraphEdge {
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
