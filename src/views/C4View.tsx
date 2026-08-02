/**
 * C4 view — renders a C4 bundle with hierarchical layout + drill-down.
 *
 * Hierarchy (per C4 model):
 *   Level 1: Person, SoftwareSystem  (Context)
 *   Level 2: Container
 *   Level 3: Component
 *   Level 4: Code (defer to M17.5+)
 *
 * Drill-down: click a node at level N → it becomes the "focus" and
 * only its children (level N+1) plus its sibling systems are shown.
 * This implements the C4 "zoom in" semantic.
 */

import { For, Show, createMemo, createSignal, type Component } from "solid-js";
import type { GraphNode } from "../bundle/loader";

export interface C4ViewProps {
  nodes: GraphNode[];
  edges: { source: string; target: string }[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  /** Optional id of a system-level element to drill into on mount. */
  drillIntoId?: string;
}

export const C4View: Component<C4ViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    props.drillIntoId ?? null,
  );

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /**
   * When focused on a node, the visible set is: the focus itself +
   * its children (one level down) + the parent context (if any) for
   * orientation. Otherwise all nodes are visible.
   */
  const visibleNodes = createMemo<GraphNode[]>(() => {
    const f = focus();
    if (!f) return props.nodes;
    const children = props.nodes.filter((n) => n.parentId === f.id);
    const result: GraphNode[] = [f, ...children];
    // Include the parent context for orientation
    if (f.parentId) {
      const parent = props.nodes.find((n) => n.id === f.parentId);
      if (parent) result.push(parent);
    }
    return result;
  });

  const visibleIds = createMemo(() => new Set(visibleNodes().map((n) => n.id)));

  const visibleEdges = createMemo(() =>
    props.edges.filter(
      (e) => visibleIds().has(e.source) && visibleIds().has(e.target),
    ),
  );

  const groupedByLevel = createMemo(() => {
    const groups = new Map<number, GraphNode[]>();
    for (const n of visibleNodes()) {
      const level = n.level ?? 0;
      const arr = groups.get(level) ?? [];
      arr.push(n);
      groups.set(level, arr);
    }
    return [...groups.entries()].sort(([a], [b]) => a - b);
  });

  const handleNodeClick = (id: string) => {
    props.onSelect(id);
    setFocusId(id);
  };

  return (
    <div class="c4-view">
      <header class="c4-breadcrumb">
        <button
          class="breadcrumb-root"
          onClick={() => {
            setFocusId(null);
            props.onSelect(null);
          }}
          disabled={!focus()}
        >
          All systems
        </button>
        <Show when={focus()}>
          {(f) => (
            <>
              <span class="breadcrumb-sep">›</span>
              <span class="breadcrumb-current">{f().label}</span>
            </>
          )}
        </Show>
      </header>

      <Show
        when={visibleNodes().length > 0}
        fallback={<p class="empty">No elements to show at this level.</p>}
      >
        <div class="c4-levels">
          <For each={groupedByLevel()}>
            {([level, nodes]) => (
              <div class={`c4-level c4-level-${level}`}>
                <h3 class="c4-level-title">
                  {levelLabel(level)} ({nodes.length})
                </h3>
                <ul class="c4-element-list">
                  <For each={nodes}>
                    {(node) => (
                      <li>
                        <button
                          class={`c4-element ${
                            props.selectedId === node.id ? "selected" : ""
                          }`}
                          onClick={() => handleNodeClick(node.id)}
                        >
                          <span class="c4-element-name">{node.label}</span>
                          <Show when={getC4Meta(node, "technology")}>
                            {(tech) => (
                              <span class="c4-element-tech">[{tech()}]</span>
                            )}
                          </Show>
                          <Show when={getC4Meta(node, "description")}>
                            {(desc) => (
                              <span class="c4-element-desc">{desc()}</span>
                            )}
                          </Show>
                          <Show
                            when={
                              props.nodes.some(
                                (n) => n.parentId === node.id,
                              )
                            }
                          >
                            <span class="c4-drill-hint">▸ drill in</span>
                          </Show>
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            )}
          </For>
        </div>
      </Show>

      <Show when={visibleEdges().length > 0}>
        <footer class="c4-relations">
          <h4>Relations</h4>
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

function getC4Meta(node: GraphNode, key: string): string | undefined {
  const meta = node.meta ?? {};
  const v = meta[key];
  return typeof v === "string" ? v : undefined;
}
