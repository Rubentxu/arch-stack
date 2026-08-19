/**
 * Cross-view navigation model (ADR-062, Wave 3 items 31–33).
 *
 * Pure module — no imports from `bundle/`, `renderer/`, `components/`
 * or `lib/` (dependency rules in AGENTS.md). Operates on the canonical
 * ids and C4 level hints already present in bundle nodes, so identity
 * survives view changes without touching the graph or the schema.
 */

/** One entry in the navigation history stack. */
export interface NavEntry {
  /** Bundle URL that this entry loaded. */
  url: string;
  /** Breadcrumb label. */
  label: string;
  /** Canonical element id this entry focuses (zoom target). */
  elementId?: string;
  /** C4 level of the focused element (1=context, 2=container, 3=component). */
  level?: number;
}

/** C4 zoom level (see `RendererNode.level` mapping in types.ts). */
export type C4Level = 1 | 2 | 3;

/**
 * Build the `archctl diagram export` selector for a C4 level + element.
 * Selector grammar (`archctl/src/diagram/selector.rs`): `<c4-kind>:<scope>`.
 */
export function c4SelectorFor(level: C4Level, elementId: string): string {
  const kind =
    level === 1 ? "context" : level === 2 ? "container" : "component";
  return `c4-${kind}:${elementId}`;
}

/** The `/api/export` URL for a selector (server accepts any valid selector). */
export function exportUrlFor(selector: string): string {
  return `/api/export?selector=${encodeURIComponent(selector)}`;
}

/** Minimal node shape needed for zoom decisions. */
export interface ZoomableNode {
  id: string;
  level?: number;
  parentId?: string;
}

/** A resolved zoom navigation target. */
export interface ZoomTarget {
  url: string;
  label: string;
  elementId: string;
  level: C4Level;
}

function target(level: C4Level, elementId: string): ZoomTarget {
  const selector = c4SelectorFor(level, elementId);
  return {
    url: exportUrlFor(selector),
    label: selector,
    elementId,
    level,
  };
}

/**
 * Resolve the zoom target for `dir`.
 * - "in": level 1 → `c4-container:<id>`, level 2 → `c4-component:<id>`;
 *   null otherwise.
 * - "out": needs `parentId` + level > 1 → `<level-1>:<parentId>`;
 *   null otherwise (S2: no zoom out from the root level).
 */
export function zoomTargetFor(
  node: ZoomableNode,
  dir: "in" | "out",
): ZoomTarget | null {
  if (dir === "in") {
    if (node.level === 1) return target(2, node.id);
    if (node.level === 2) return target(3, node.id);
    return null;
  }
  if (node.parentId && typeof node.level === "number" && node.level > 1) {
    const up = (node.level - 1) as C4Level;
    return target(up, node.parentId);
  }
  return null;
}

/**
 * Immutable, bounded navigation stack with a cursor (back/forward stable
 * per ADR-056 acceptance criteria). Every operation returns a new stack,
 * which makes it a drop-in for SolidJS signals.
 */
export class NavStack {
  private readonly _entries: readonly NavEntry[];
  private readonly _cursor: number;

  constructor(entries: readonly NavEntry[] = [], cursor = -1) {
    this._entries = entries;
    this._cursor = cursor;
  }

  get length(): number {
    return this._entries.length;
  }

  get index(): number {
    return this._cursor;
  }

  all(): readonly NavEntry[] {
    return this._entries;
  }

  current(): NavEntry | null {
    return this._cursor >= 0 ? (this._entries[this._cursor] ?? null) : null;
  }

  /** Push a new entry; truncates forward history (S12). */
  push(entry: NavEntry): NavStack {
    const entries = [...this._entries.slice(0, this._cursor + 1), entry];
    return new NavStack(entries, entries.length - 1);
  }

  /** Step back; no-op at the start. */
  back(): NavStack {
    if (this._cursor <= 0) return this;
    return new NavStack(this._entries, this._cursor - 1);
  }

  /** Step forward; no-op at the end. */
  forward(): NavStack {
    if (this._cursor >= this._entries.length - 1) return this;
    return new NavStack(this._entries, this._cursor + 1);
  }

  /** Jump to an absolute index (breadcrumb click). */
  jumpTo(i: number): NavStack {
    if (i < 0 || i >= this._entries.length) return this;
    return new NavStack(this._entries, i);
  }
}
