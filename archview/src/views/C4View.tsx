/**
 * C4 view — renders a C4 bundle as a hierarchical G6 graph with
 * drill-down (M17.1) and semantic zoom (M18).
 *
 * Hierarchy (per C4 model):
 *   Level 1: Person, SoftwareSystem  (Context)
 *   Level 2: Container
 *   Level 3: Component
 *   Level 4: Code (defer to M17.1.1)
 *
 * Drill-down: click a node at level N → it becomes the "focus" and
 * only its descendants (level N+1) plus its ancestors are shown.
 * The graph re-lays-out and fits the view to the focus node.
 *
 * Semantic zoom: a pill bar above the canvas selects a global
 * level filter (Context / Container / Component / Code). When a
 * pill is active, drill-down is suppressed — the user sees every
 * node at that level, regardless of focus. Focus is still tracked
 * for sidebar selection. The last picked level persists across
 * bundle loads via localStorage.
 *
 * M17.1 changed the rendering substrate from a column-of-`<ul>`
 * text view to a G6 canvas with dagre (top→bottom) layout. The
 * pure helpers in `./C4Graph` are unchanged.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  untrack,
  type Component,
} from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import { GraphRenderer } from "../renderer/g6";
import {
  breadcrumbTrail,
  groupNodesByLevel,
  levelCounts,
  visibleEdgesFor,
  visibleNodesWithLevel,
} from "./C4Graph";

const LEVEL_STORAGE_KEY = "archview.c4.lastLevel";

function readStoredLevel(): number | null {
  // jsdom (vitest) exposes `localStorage` as a plain object, not
  // the Storage instance browsers provide. Guard against both
  // missing entirely and the typeof-not-callable case.
  if (typeof localStorage === "undefined") return null;
  if (typeof localStorage.getItem !== "function") return null;
  try {
    const raw = localStorage.getItem(LEVEL_STORAGE_KEY);
    if (!raw) return null;
    const n = Number.parseInt(raw, 10);
    if (Number.isFinite(n) && n >= 1 && n <= 4) return n;
  } catch {
    return null;
  }
  return null;
}

function writeStoredLevel(level: number | null): void {
  if (typeof localStorage === "undefined") return;
  if (typeof localStorage.setItem !== "function") return;
  try {
    if (level === null) {
      localStorage.removeItem(LEVEL_STORAGE_KEY);
    } else {
      localStorage.setItem(LEVEL_STORAGE_KEY, String(level));
    }
  } catch {
    // Storage may be disabled (Safari private mode, sandboxed
    // iframe). The filter still works in-memory; the persistence
    // is a nicety, not a contract.
  }
}

export interface C4ViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  /** Optional id of a system-level element to drill into on mount. */
  drillIntoId?: string;
}

export const C4View: Component<C4ViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    // drillIntoId is read once at mount and is not expected to
    // change — `untrack` makes that intent explicit and silences
    // the solid/reactivity linter.
    untrack(() => props.drillIntoId ?? null),
  );
  // M18: global level filter. null = no filter (drill-down mode).
  const [levelFilter, setLevelFilter] = createSignal<number | null>(
    readStoredLevel(),
  );
  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /** Visible set: level filter wins when set, otherwise drill-down
   *  (focus + descendants + ancestors, or all when unfocused). */
  const visibleNodes = createMemo<GraphNode[]>(() =>
    visibleNodesWithLevel(props.nodes, levelFilter(), focusId()),
  );

  const visibleEdges = createMemo(() =>
    visibleEdgesFor(props.edges, visibleNodes()),
  );

  const groupedByLevel = createMemo(() => groupNodesByLevel(visibleNodes()));

  const trail = createMemo(() => breadcrumbTrail(props.nodes, focusId()));

  /** Pill bar data: each present C4 level → count of nodes there.
   *  Pills for levels that do not exist in the bundle are not
   *  rendered (no empty pills). */
  const levelPills = createMemo(() => levelCounts(props.nodes));

  const handleNodeClick = (id: string) => {
    props.onSelect(id);
    // Clicking a node while a level filter is active only selects
    // it (for the sidebar) — the visible set is already the whole
    // level, so re-pushing it would be a no-op. Drilling back into
    // a subtree requires clearing the pill first.
    setFocusId(id);
  };

  const handleReset = () => {
    setFocusId(null);
    props.onSelect(null);
  };

  const handleBreadcrumbJump = (id: string) => {
    setFocusId(id);
    props.onSelect(id);
  };

  const handleLevelPillClick = (level: number) => {
    // Toggle: clicking the active pill clears the filter; clicking
    // a new pill sets it. This keeps the bar one-click from the
    // "drill-down" mode and avoids the need for a separate
    // "All levels" button.
    if (levelFilter() === level) {
      setLevelFilter(null);
      writeStoredLevel(null);
    } else {
      setLevelFilter(level);
      writeStoredLevel(level);
    }
  };

  // Mount the renderer once the container is in the DOM.
  onMount(() => {
    const width = containerRef.clientWidth || 800;
    const height = containerRef.clientHeight || 600;
    renderer = new GraphRenderer({
      container: containerRef,
      width,
      height,
      layout: {
        type: "dagre",
        rankdir: "TB",
        align: "UL",
        nodesep: 40,
        ranksep: 60,
      },
      onNodeClick: (id) => {
        handleNodeClick(id);
      },
    });

    // Initial data push.
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "c4-view",
      loadedAt: "0",
      rawKind: "c4",
      nodes: visibleNodes(),
      edges: visibleEdges(),
    });
  });

  // Re-push data when the visible set changes (drill-in/out OR
  // level pill toggle). M18: both effects share the same
  // `visibleNodes` memo, so a single createEffect covers both
  // interactions.
  createEffect(() => {
    const nodes = visibleNodes();
    const edges = visibleEdges();
    if (!renderer) return;
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "c4-view",
      loadedAt: "0",
      rawKind: "c4",
      nodes,
      edges,
    });
  });

  // Update focus ring + fit-view when focusId changes.
  createEffect(() => {
    const id = focusId();
    if (!renderer) return;
    if (id) {
      void renderer.focusNode(id);
    } else {
      void renderer.clearFocus();
    }
  });

  // Resize observer for the canvas container.
  onMount(() => {
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry || !renderer) return;
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) {
        renderer.resize(width, height);
      }
    });
    ro.observe(containerRef);
    onCleanup(() => ro.disconnect());
  });

  // Cleanup on unmount.
  onCleanup(() => {
    renderer?.destroy();
    renderer = undefined;
  });

  return (
    <div class="c4-view">
      <header class="c4-breadcrumb">
        <button
          class="breadcrumb-root"
          onClick={handleReset}
          disabled={!focus()}
        >
          All systems
        </button>
        <Show when={trail().length > 0}>
          <For each={trail()}>
            {(id, idx) => {
              const node = props.nodes.find((n) => n.id === id);
              if (!node) return null;
              const isLast = () => idx() === trail().length - 1;
              return (
                <>
                  <Show when={idx() > 0}>
                    <span class="breadcrumb-sep">›</span>
                  </Show>
                  <Show
                    when={!isLast()}
                    fallback={
                      <span class="breadcrumb-current">{node.label}</span>
                    }
                  >
                    <button
                      class="breadcrumb-link"
                      onClick={() => handleBreadcrumbJump(id)}
                    >
                      {node.label}
                    </button>
                  </Show>
                </>
              );
            }}
          </For>
        </Show>
      </header>

      <nav class="c4-level-pills" aria-label="Semantic zoom level">
        <button
          type="button"
          class="c4-level-pill"
          classList={{ "is-active": levelFilter() === null }}
          aria-pressed={levelFilter() === null}
          onClick={() => {
            setLevelFilter(null);
            writeStoredLevel(null);
          }}
        >
          All levels
        </button>
        <For each={levelPills()}>
          {([level, count]) => (
            <button
              type="button"
              class="c4-level-pill"
              classList={{ "is-active": levelFilter() === level }}
              aria-pressed={levelFilter() === level}
              onClick={() => handleLevelPillClick(level)}
            >
              <span class="c4-level-pill-label">{levelLabel(level)}</span>
              <span class="c4-level-pill-count" aria-label={`${count} nodes`}>
                {count}
              </span>
            </button>
          )}
        </For>
      </nav>

      <div ref={containerRef} class="c4-canvas" />

      <Show when={visibleEdges().length > 0}>
        <footer class="c4-relations">
          <h4>Relations ({visibleEdges().length})</h4>
          <ul>
            <For each={visibleEdges()}>
              {(e) => (
                <li>
                  <code>{e.source}</code> → <code>{e.target}</code>
                </li>
              )}
            </For>
          </ul>
        </footer>
      </Show>

      <Show
        when={visibleNodes().length === 0}
        fallback={
          <aside class="c4-levels" aria-label="C4 levels">
            <For each={groupedByLevel()}>
              {([level, nodes]) => (
                <div class={`c4-level c4-level-${level}`}>
                  <h3 class="c4-level-title">
                    {levelLabel(level)} ({nodes.length})
                  </h3>
                </div>
              )}
            </For>
          </aside>
        }
      >
        <p class="empty">No elements to show at this level.</p>
      </Show>
    </div>
  );
};

function levelLabel(level: number): string {
  switch (level) {
    case 1:
      return "Context";
    case 2:
      return "Container";
    case 3:
      return "Component";
    case 4:
      return "Code";
    default:
      return "Other";
  }
}
