/**
 * ImpactView — marks a node as "proposed for change" and shows the
 * blast radius: every node that transitively depends on it (callers)
 * or that it transitively depends on (callees).
 *
 * Why M17.7: in a refactor or feature change, you need to know
 * "if I change this function, what else is affected?" This view
 * answers that question.
 *
 * Algorithm: BFS in both directions from focus (configurable:
 * upstream / downstream / both). Each step shows the path from
 * focus → leaf so the user can see WHY a node is in the zone.
 *
 * Visual: focus highlighted orange (proposed change), impact zone
 * nodes highlighted yellow (blast radius), other nodes dimmed.
 */

import { For, Show, createMemo, createSignal, type Component } from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import type { SidebarStats } from "../components/Sidebar";

export type ImpactDirection = "upstream" | "downstream" | "both";

export interface ImpactViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  initialFocusId?: string;
  onSelect: (id: string | null) => void;
  onStats?: (stats: SidebarStats) => void;
}

interface ImpactEntry {
  nodeId: string;
  depth: number;
  direction: "upstream" | "downstream";
  path: string[];
}

export const ImpactView: Component<ImpactViewProps> = (props) => {
  const [focusId, setFocusId] = createSignal<string | null>(
    props.initialFocusId ?? props.nodes[0]?.id ?? null,
  );
  const [direction, setDirection] =
    createSignal<ImpactDirection>("both");

  const focus = createMemo(() => {
    const id = focusId();
    if (!id) return null;
    return props.nodes.find((n) => n.id === id) ?? null;
  });

  /**
   * BFS from focus in both directions. Returns entries with the
   * path from focus → leaf for context. Deduplicates nodes that
   * appear in both directions (they appear once at the shallower
   * depth).
   */
  const impactEntries = createMemo<ImpactEntry[]>(() => {
    const f = focus();
    if (!f) return [];
    const entries: ImpactEntry[] = [];
    const visited = new Map<string, ImpactEntry>();

    // Add focus itself with depth 0
    const focusEntry: ImpactEntry = {
      nodeId: f.id,
      depth: 0,
      direction: "upstream", // arbitrary, focus is the source
      path: [f.id],
    };
    entries.push(focusEntry);
    visited.set(f.id, focusEntry);

    const traverse = (
      startId: string,
      dir: "upstream" | "downstream",
    ) => {
      let frontier: Array<{ id: string; path: string[] }> = [
        { id: startId, path: [startId] },
      ];
      let depth = 0;
      const MAX_DEPTH = 5;
      while (frontier.length > 0 && depth < MAX_DEPTH) {
        depth++;
        const next: typeof frontier = [];
        for (const { id, path } of frontier) {
          const neighbors = props.edges
            .filter((e) =>
              dir === "upstream" ? e.target === id : e.source === id,
            )
            .map((e) => (dir === "upstream" ? e.source : e.target));
          for (const n of neighbors) {
            if (n === f.id) continue; // don't loop back to focus
            const newPath = [...path, n];
            const existing = visited.get(n);
            if (!existing || existing.depth > depth) {
              const entry: ImpactEntry = {
                nodeId: n,
                depth,
                direction: dir,
                path: newPath,
              };
              visited.set(n, entry);
              if (!existing) entries.push(entry);
              next.push({ id: n, path: newPath });
            }
          }
        }
        frontier = next;
      }
    };

    if (direction() === "upstream" || direction() === "both") {
      traverse(f.id, "upstream");
    }
    if (direction() === "downstream" || direction() === "both") {
      traverse(f.id, "downstream");
    }
    return entries;
  });

  /** Group entries by depth for hierarchical display. */
  const byDepth = createMemo<Record<number, ImpactEntry[]>>(() => {
    const groups: Record<number, ImpactEntry[]> = {};
    for (const e of impactEntries()) {
      if (e.depth === 0) continue; // skip focus itself
      (groups[e.depth] ??= []).push(e);
    }
    return groups;
  });

  /** Stats: count of impacted nodes + max depth. */
  const impactCount = createMemo<number>(
    () => impactEntries().filter((e) => e.depth > 0).length,
  );
  const maxDepth = createMemo<number>(() =>
    impactEntries().reduce((m, e) => Math.max(m, e.depth), 0),
  );

  const nodeById = createMemo(() => {
    const m = new Map<string, GraphNode>();
    for (const n of props.nodes) m.set(n.id, n);
    return m;
  });

  // Emit stats whenever focus/direction change.
  createMemo(() => {
    props.onStats?.({
      blastRadius: impactCount(),
      depth: maxDepth(),
      direction:
        direction() === "both"
          ? "both"
          : direction() === "upstream"
            ? "callers"
            : "callees",
    });
  });

  return (
    <div class="impact-view">
      <header class="impact-header">
        <div class="impact-focus">
          <label>Proposed change at</label>
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

        <div class="impact-controls">
          <label>Direction</label>
          <select
            value={direction()}
            onChange={(e) =>
              setDirection(e.currentTarget.value as ImpactDirection)
            }
          >
            <option value="downstream">downstream (callees)</option>
            <option value="upstream">upstream (callers)</option>
            <option value="both">both</option>
          </select>
        </div>
      </header>

      <Show
        when={focus()}
        fallback={<p class="empty">No function selected.</p>}
      >
        {(f) => (
          <div class="impact-body">
            <section class="impact-focus-detail">
              <h2>
                <span class="impact-tag focus">PROPOSED</span>
                {f().label}
              </h2>
              <Show when={f().file}>
                <code class="impact-file">
                  {f().file}:{f().line ?? "?"}
                </code>
              </Show>
              <div class="impact-stats">
                <span>
                  <strong>{impactCount()}</strong> impacted functions
                </span>
                <span>·</span>
                <span>
                  max depth <strong>{maxDepth()}</strong>
                </span>
                <span>·</span>
                <span>{direction()}</span>
              </div>
            </section>

            <Show
              when={impactCount() > 0}
              fallback={
                <p class="empty">
                  No {direction()} impact from {f().label} within 5 levels.
                </p>
              }
            >
              <section class="impact-zones">
                <h3>Blast radius by depth</h3>
                <For each={Object.entries(byDepth()).sort(([a], [b]) => Number(a) - Number(b))}>
                  {([depth, entries]) => (
                    <div class="impact-depth">
                      <h4>
                        Depth {depth}{" "}
                        <span class="muted">({entries.length})</span>
                      </h4>
                      <ul class="impact-list">
                        <For each={entries}>
                          {(entry) => {
                            const node = nodeById().get(entry.nodeId);
                            if (!node) return null;
                            return (
                              <li>
                                <button
                                  class={`impact-node ${entry.direction}`}
                                  onClick={() => props.onSelect(node.id)}
                                >
                                  <span class={`impact-arrow-${entry.direction}`}>
                                    {entry.direction === "upstream" ? "↑" : "↓"}
                                  </span>
                                  <span class="impact-node-name">{node.label}</span>
                                  <Show when={node.file}>
                                    <span class="impact-node-file">
                                      {node.file}:{node.line ?? "?"}
                                    </span>
                                  </Show>
                                </button>
                                <details class="impact-path">
                                  <summary>path from focus</summary>
                                  <ol>
                                    <For each={entry.path}>
                                      {(pid) => {
                                        const pnode = nodeById().get(pid);
                                        return (
                                          <li>
                                            <code>{pnode?.label ?? pid}</code>
                                          </li>
                                        );
                                      }}
                                    </For>
                                  </ol>
                                </details>
                              </li>
                            );
                          }}
                        </For>
                      </ul>
                    </div>
                  )}
                </For>
              </section>
            </Show>
          </div>
        )}
      </Show>
    </div>
  );
};
