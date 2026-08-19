/**
 * C4 view — renders a C4 bundle as a hierarchical G6 graph with
 * drill-down (M17.1).
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
  visibleEdgesFor,
  visibleNodesForFocus,
} from "./C4Graph";

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
  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /** Visible set: focus + descendants + ancestors (or all when
   *  unfocused). */
  const visibleNodes = createMemo<GraphNode[]>(() =>
    visibleNodesForFocus(props.nodes, focusId()),
  );

  const visibleEdges = createMemo(() =>
    visibleEdgesFor(props.edges, visibleNodes()),
  );

  const groupedByLevel = createMemo(() => groupNodesByLevel(visibleNodes()));

  const trail = createMemo(() => breadcrumbTrail(props.nodes, focusId()));

  const handleNodeClick = (id: string) => {
    props.onSelect(id);
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

  // Re-push data when the visible set changes (drill-in/out).
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
