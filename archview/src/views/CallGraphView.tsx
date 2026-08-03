/**
 * CallGraphView — focuses on a single function and shows N levels
 * of callers/callees reachable from it.
 *
 * - Focus node: the user-selected function (or first node by default)
 * - Direction: callers (incoming), callees (outgoing), or both
 * - Depth: BFS expansion in steps (1-N). Buttons add/remove levels.
 * - Async flow: edges tagged with `message_kind` (SyncCall/AsyncCall)
 *   are rendered with distinct visual styles.
 * - Blast radius: count of unique functions reachable from the focus
 *   at the current depth (emitted via onStats for the sidebar).
 *
 * Implementation: pure data transformation (BFS) + SolidJS render.
 * No G6 canvas here — text-based list view (M17.2 MVP). M17.2.1 can
 * upgrade to G6 with hierarchical layout.
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  type Component,
} from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import type { SidebarStats } from "../components/Sidebar";
import {
  expandLevels,
  blastRadiusOf,
  MAX_DEPTH,
  type CallDirection,
  type LevelGroup,
} from "./CallGraphGraph";

export type { CallDirection };

export interface CallGraphViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  initialFocusId?: string;
  initialDepth?: number;
  onSelect: (id: string | null) => void;
  onStats?: (stats: SidebarStats) => void;
}

export const CallGraphView: Component<CallGraphViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    props.initialFocusId ?? props.nodes[0]?.id ?? null,
  );
  const [depth, setDepth] = createSignal<number>(props.initialDepth ?? 1);
  const [direction, setDirection] = createSignal<CallDirection>("callees");

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /** BFS from focus with direction + depth controls. */
  const levelGroups = createMemo<LevelGroup[]>(() =>
    expandLevels(
      props.nodes,
      props.edges,
      focusId() ?? "",
      depth(),
      direction(),
    ),
  );

  /** Blast radius: total unique functions reachable from focus. */
  const blastRadius = createMemo<number>(() => blastRadiusOf(levelGroups()));

  // Emit stats to sidebar whenever focus/depth/direction change.
  createEffect(() => {
    props.onStats?.({
      blastRadius: blastRadius(),
      depth: depth(),
      direction: direction(),
    });
  });

  return (
    <div class="callgraph-view">
      <header class="callgraph-header">
        <div class="callgraph-focus">
          <label>Focus</label>
          <select
            value={focusId() ?? ""}
            onChange={(e) => {
              setFocusId(e.currentTarget.value);
              props.onSelect(e.currentTarget.value);
            }}
          >
            <For each={props.nodes}>
              {(n) => <option value={n.id}>{n.label}</option>}
            </For>
          </select>
        </div>

        <div class="callgraph-controls">
          <div class="control-group">
            <label>Direction</label>
            <select
              value={direction()}
              onChange={(e) =>
                setDirection(e.currentTarget.value as CallDirection)
              }
            >
              <option value="callees">callees (downstream)</option>
              <option value="callers">callers (upstream)</option>
              <option value="both">both</option>
            </select>
          </div>
          <div class="control-group">
            <label>Depth</label>
            <div class="depth-buttons">
              <button
                onClick={() => setDepth(Math.max(1, depth() - 1))}
                disabled={depth() <= 1}
                aria-label="Decrease depth"
              >
                −
              </button>
              <span class="depth-value">{depth()}</span>
              <button
                onClick={() => setDepth(Math.min(MAX_DEPTH, depth() + 1))}
                disabled={depth() >= MAX_DEPTH}
                aria-label="Increase depth"
              >
                +
              </button>
            </div>
          </div>
        </div>
      </header>

      <Show
        when={focus()}
        fallback={<p class="empty">No function selected.</p>}
      >
        {(f) => (
          <div class="callgraph-body">
            <section class="callgraph-focus-detail">
              <h2>{f().label}</h2>
              <Show when={f().file}>
                <code class="callgraph-file">
                  {f().file}:{f().line ?? "?"}
                </code>
              </Show>
              <div class="callgraph-stats">
                <span>
                  <strong>{blastRadius()}</strong> reachable functions
                </span>
                <span>·</span>
                <span>
                  depth <strong>{depth()}</strong> / {MAX_DEPTH}
                </span>
              </div>
            </section>

            <section class="callgraph-levels">
              <For
                each={levelGroups()}
                fallback={
                  <p class="empty">
                    No reachable functions at depth {depth()} in this direction.
                  </p>
                }
              >
                {(group) => (
                  <div class="callgraph-level">
                    <h3>
                      L{group.depth} · {group.direction} ({group.nodes.length})
                    </h3>
                    <ul class="callgraph-list">
                      <For each={group.nodes}>
                        {(node) => (
                          <li>
                            <button
                              class="callgraph-node"
                              onClick={() => {
                                setFocusId(node.id);
                                props.onSelect(node.id);
                              }}
                            >
                              {node.label}
                              <Show when={node.file}>
                                <span class="callgraph-node-file">
                                  {node.file}:{node.line ?? "?"}
                                </span>
                              </Show>
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                    <Show when={group.edges.length > 0}>
                      <details class="callgraph-edges">
                        <summary>
                          {group.edges.length} edge
                          {group.edges.length === 1 ? "" : "s"}
                        </summary>
                        <ul>
                          <For each={group.edges}>
                            {(e) => (
                              <li>
                                <code>{e.source}</code>
                                <span
                                  class={`callgraph-edge-kind kind-${e.kind ?? "default"}`}
                                >
                                  {e.kind ?? "calls"}
                                </span>
                                <code>{e.target}</code>
                              </li>
                            )}
                          </For>
                        </ul>
                      </details>
                    </Show>
                  </div>
                )}
              </For>
            </section>
          </div>
        )}
      </Show>
    </div>
  );
};
