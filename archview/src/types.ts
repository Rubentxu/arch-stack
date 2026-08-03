/**
 * Shared renderer contract for archview bundles.
 *
 * This is the single point of contact between the bundle loader and the
 * renderer (R3): `renderer/g6.ts` resolves its bundle type from here and
 * MUST NOT import from `bundle/loader`. The loader produces values that
 * satisfy this contract; views may consume the richer loader types.
 *
 * The shape is aligned with the canonical `viewer-bundle` schema
 * (`schemas/diagram-projection.schema.json`):
 * - `RendererNode` ← projection `Node` (`id`, `type`, `name`, ...)
 * - `RendererEdge` ← projection `Edge` (`id`, `source`, `target`, `predicate`, ...)
 * - `RendererBundle` ← the four-section bundle (`manifest` + `projection`
 *   + `evidence` + `styles`)
 */

export interface RendererNode {
  id: string;
  label: string;
  kind: string;
  language?: string;
  file?: string;
  line?: number;
  /**
   * C4 hierarchy level (1-4). Derived from the canonical `type` enum:
   * `context`→1, `container`→2, `component`→3, `dynamic`/`deployment`→1.
   * Undefined for non-C4 bundles.
   */
  level?: number;
  /** Parent element id (for C4 drill-down), derived from `canonicalKey`. */
  parentId?: string;
  meta?: Record<string, unknown>;
}

export interface RendererEdge {
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

export interface RendererBundle {
  schemaVersion: string;
  source: string;
  loadedAt: string;
  nodes: RendererNode[];
  edges: RendererEdge[];
  /** Bundle shape that produced this normalized bundle. */
  rawKind: "call-graph" | "sequence" | "class-diagram" | "c4" | "unknown";
  /**
   * Original interactions, only populated for `rawKind === "sequence"`.
   * SequenceView uses this to render lifelines + arrows in time order.
   */
  interactions?: SequenceInteraction[];
  /**
   * Canonical `viewer-bundle` evidence bundle, preserved for consumers
   * (R1). Only populated for `rawKind === "c4"`.
   */
  evidence?: unknown;
  /**
   * Canonical `viewer-bundle` styles, preserved for consumers (R1).
   * Only populated for `rawKind === "c4"`.
   */
  styles?: unknown;
}
