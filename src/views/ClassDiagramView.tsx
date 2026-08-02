/**
 * ClassDiagramView — renders class-diagram bundles as UML class
 * diagrams with compartments (name / attributes / methods) and
 * kind-specific edge styles.
 *
 * Layout (left-to-right, simple flow):
 *   - One card per class with three sections:
 *     1. Header: stereotype (interface/trait/enum) + class name
 *     2. Attributes (members with member_kind="field")
 *     3. Methods (members with member_kind="fn" / "method")
 *   - Edges: extends (solid with triangle), implements (dashed),
 *     composes (solid with diamond).
 *
 * M17.4 MVP is a card grid + edge list. M17.4.1 can upgrade to a
 * force-directed graph layout via G6 with compartment nodes.
 */

import { For, Show, createMemo, type Component } from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";

export interface ClassDiagramViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onSelect: (id: string | null) => void;
}

interface ClassMember {
  name: string;
  member_kind: string;
  signature?: string;
  line?: number;
}

export const ClassDiagramView: Component<ClassDiagramViewProps> = (props) => {
  /** Partition members by kind (field vs fn). */
  const partition = (node: GraphNode): { fields: ClassMember[]; methods: ClassMember[] } => {
    const raw = (node.meta?.members as ClassMember[] | undefined) ?? [];
    const fields: ClassMember[] = [];
    const methods: ClassMember[] = [];
    for (const m of raw) {
      if (m.member_kind === "field") fields.push(m);
      else if (m.member_kind === "fn" || m.member_kind === "method") methods.push(m);
    }
    return { fields, methods };
  };

  /** Stereotype (interface / trait / enum) for the header. */
  const stereotype = (kind: string): string | undefined => {
    if (kind === "interface") return "<<interface>>";
    if (kind === "trait") return "<<trait>>";
    if (kind === "enum") return "<<enum>>";
    return undefined;
  };

  /** Edges grouped by predicate kind for the bottom panel. */
  const edgeGroups = createMemo(() => {
    const groups: Record<string, GraphEdge[]> = {
      extends: [],
      implements: [],
      composes: [],
      other: [],
    };
    for (const e of props.edges) {
      const k = e.kind ?? "other";
      if (k in groups) groups[k].push(e);
      else groups.other.push(e);
    }
    return groups;
  });

  const nodeById = createMemo<Map<string, GraphNode>>(() => {
    const m = new Map<string, GraphNode>();
    for (const n of props.nodes) m.set(n.id, n);
    return m;
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
        fallback={
          <p class="empty">No classes in this bundle.</p>
        }
      >
        <div class="cd-grid">
          <For each={props.nodes}>
            {(node) => {
              const { fields, methods } = partition(node);
              return (
                <article
                  class={`cd-card cd-kind-${node.kind}`}
                  onClick={() => props.onSelect(node.id)}
                >
                  <header class="cd-card-header">
                    <Show when={stereotype(node.kind)}>
                      {(s) => <span class="cd-stereotype">{s()}</span>}
                    </Show>
                    <h3 class="cd-name">{node.label}</h3>
                    <Show when={node.language}>
                      <span class="cd-language">{node.language}</span>
                    </Show>
                    <Show when={node.file}>
                      <span class="cd-file">
                        {node.file}:{node.line ?? "?"}
                      </span>
                    </Show>
                  </header>

                  <Show when={fields.length > 0}>
                    <section class="cd-compartment cd-attributes">
                      <h4>attributes</h4>
                      <ul>
                        <For each={fields}>
                          {(f) => (
                            <li>
                              <span class="cd-member-name">
                                {f.name || "(anon)"}
                              </span>
                              <Show when={f.signature}>
                                <span class="cd-member-sig">
                                  : {f.signature}
                                </span>
                              </Show>
                            </li>
                          )}
                        </For>
                      </ul>
                    </section>
                  </Show>

                  <Show when={methods.length > 0}>
                    <section class="cd-compartment cd-methods">
                      <h4>methods</h4>
                      <ul>
                        <For each={methods}>
                          {(m) => (
                            <li>
                              <span class="cd-member-name">
                                {m.name || "(anon)"}()
                              </span>
                              <Show when={m.signature}>
                                <span class="cd-member-sig">
                                  {m.signature}
                                </span>
                              </Show>
                            </li>
                          )}
                        </For>
                      </ul>
                    </section>
                  </Show>
                </article>
              );
            }}
          </For>
        </div>

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
                              <span class={`cd-arrow-kind kind-${kind}`}>─▸</span>
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
