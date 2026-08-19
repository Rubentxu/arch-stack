/**
 * ClassDiagramView — renders class-diagram bundles as a G6
 * dagre horizontal graph (M17.1.1).
 *
 * Layout (left-to-right):
 *   - One circle per class/interface/trait/enum.
 *   - Color encodes the stereotype (class / interface / trait / enum)
 *     via the renderer's `byKind` palette.
 *   - Edges: extends (solid), implements (dashed), composes
 *     (thick). The edge list panel below the canvas keeps the
 *     per-predicate breakdown for users that want to read it as
 *     a table.
 *
 * M17.1.1 replaced the previous card-grid render with a G6
 * graph so the user can see the inheritance / composition
 * topology at a glance. The per-class attribute / method
 * compartments live in the sidebar now (via `onSelect`).
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  onCleanup,
  onMount,
  type Component,
} from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import { GraphRenderer } from "../renderer/g6";
import { groupEdgesByPredicate } from "./ClassDiagramGraph";

export interface ClassDiagramViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  selectedId: string | null;
  onSelect: (id: string | null) => void;
}

export const ClassDiagramView: Component<ClassDiagramViewProps> = (props) => {
  let containerRef!: HTMLDivElement;
  let renderer: GraphRenderer | undefined;

  const edgeGroups = createMemo(() => groupEdgesByPredicate(props.edges));

  const nodeById = createMemo<Map<string, GraphNode>>(() => {
    const m = new Map<string, GraphNode>();
    for (const n of props.nodes) m.set(n.id, n);
    return m;
  });

  onMount(() => {
    renderer = new GraphRenderer({
      container: containerRef,
      width: containerRef.clientWidth || 800,
      height: containerRef.clientHeight || 600,
      layout: {
        type: "dagre",
        rankdir: "LR",
        align: "UL",
        nodesep: 30,
        ranksep: 80,
      },
      onNodeClick: (id) => {
        props.onSelect(id);
      },
    });
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "class-diagram-view",
      loadedAt: "0",
      rawKind: "class-diagram",
      nodes: props.nodes,
      edges: props.edges,
    });
  });

  createEffect(() => {
    if (!renderer) return;
    renderer.setData({
      schemaVersion: "0.0.0",
      source: "class-diagram-view",
      loadedAt: "0",
      rawKind: "class-diagram",
      nodes: props.nodes,
      edges: props.edges,
    });
  });

  createEffect(() => {
    const id = props.selectedId;
    if (!renderer) return;
    if (id) {
      void renderer.focusNode(id);
    } else {
      void renderer.clearFocus();
    }
  });

  onMount(() => {
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry || !renderer) return;
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) renderer.resize(width, height);
    });
    ro.observe(containerRef);
    onCleanup(() => ro.disconnect());
  });

  onCleanup(() => {
    renderer?.destroy();
    renderer = undefined;
  });

  return (
    <div class="class-diagram-view">
      <header class="cd-header">
        <h2>Class diagram</h2>
        <p class="muted">
          {props.nodes.length} classes · {props.edges.length} relations
        </p>
      </header>

      <Show
        when={props.nodes.length > 0}
        fallback={<p class="empty">No classes in this bundle.</p>}
      >
        <div ref={containerRef} class="cd-canvas" />

        <Show when={props.edges.length > 0}>
          <section class="cd-relations">
            <h3>Relations</h3>
            <For each={Object.entries(edgeGroups())}>
              {([kind, edges]) => (
                <Show when={edges.length > 0}>
                  <div class={`cd-relation-group kind-${kind}`}>
                    <h4>
                      <span class={`cd-arrow-kind kind-${kind}`}>{kind}</span>
                      <span class="muted">({edges.length})</span>
                    </h4>
                    <ul>
                      <For each={edges}>
                        {(e) => {
                          const from = nodeById().get(e.source);
                          const to = nodeById().get(e.target);
                          return (
                            <li>
                              <code>{from?.label ?? e.source}</code>
                              <span class={`cd-arrow-kind kind-${kind}`}>
                                ─▸
                              </span>
                              <code>{to?.label ?? e.target}</code>
                            </li>
                          );
                        }}
                      </For>
                    </ul>
                  </div>
                </Show>
              )}
            </For>
          </section>
        </Show>
      </Show>
    </div>
  );
};
