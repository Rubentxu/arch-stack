/**
 * Sidebar — shows evidence for the selected node + bundle metadata.
 * Evidence pointers come from the `meta` of the GraphNode (extracted
 * from archctl bundles).
 */

import { For, Show, type Component } from "solid-js";
import type { GraphNode } from "../bundle/loader";

export interface SidebarStats {
  /** Computed by the call-graph view: unique reachable functions. */
  blastRadius?: number;
  /** Max depth explored. */
  depth?: number;
  /** Current focus direction. */
  direction?: "callees" | "callers" | "both";
}

export interface SidebarProps {
  node: GraphNode | null;
  bundleMeta: {
    source: string;
    schemaVersion: string;
    loadedAt: string;
    rawKind: string;
  } | null;
  stats?: SidebarStats;
}

export const Sidebar: Component<SidebarProps> = (props) => {
  return (
    <aside class="sidebar">
      <header class="sidebar-header">
        <h2>Bundle</h2>
        <Show when={props.bundleMeta}>
          {(meta) => (
            <dl class="bundle-meta">
              <dt>source</dt>
              <dd>{meta().source}</dd>
              <dt>schemaVersion</dt>
              <dd>{meta().schemaVersion}</dd>
              <dt>rawKind</dt>
              <dd>{meta().rawKind}</dd>
              <dt>loadedAt</dt>
              <dd>{meta().loadedAt}</dd>
            </dl>
          )}
        </Show>
      </header>

      <section class="sidebar-selection">
        <Show when={props.stats?.blastRadius !== undefined}>
          <div class="sidebar-stats">
            <h3>Call graph stats</h3>
            <dl>
              <dt>blast radius</dt>
              <dd>
                <strong>{props.stats!.blastRadius}</strong> reachable functions
              </dd>
              <Show when={props.stats?.depth !== undefined}>
                <dt>depth</dt>
                <dd>
                  {props.stats!.depth}{" "}
                  <span class="muted">({props.stats?.direction})</span>
                </dd>
              </Show>
            </dl>
          </div>
        </Show>

        <Show
          when={props.node}
          fallback={<p class="empty">Select a node to inspect its evidence.</p>}
        >
          {(node) => (
            <div class="node-detail">
              <h3>{node().label}</h3>
              <dl class="node-meta">
                <dt>kind</dt>
                <dd>
                  {node().kind}
                  <Show when={node().level !== undefined && node().level! > 0}>
                    <span class="level-tag">L{node().level}</span>
                  </Show>
                </dd>
                <Show when={getMetaString(node(), "technology")}>
                  {(tech) => (
                    <>
                      <dt>technology</dt>
                      <dd>{tech()}</dd>
                    </>
                  )}
                </Show>
                <Show when={getMetaString(node(), "description")}>
                  {(desc) => (
                    <>
                      <dt>description</dt>
                      <dd class="multiline">{desc()}</dd>
                    </>
                  )}
                </Show>
                <Show when={node().language}>
                  <dt>language</dt>
                  <dd>{node().language}</dd>
                </Show>
                <Show when={node().file}>
                  <dt>file</dt>
                  <dd>
                    <code>
                      {node().file}:{node().line ?? "?"}
                    </code>
                  </dd>
                </Show>
                <Show when={node().parentId}>
                  <dt>parent</dt>
                  <dd>
                    <code>{node().parentId}</code>
                  </dd>
                </Show>
              </dl>
              <h4>Evidence</h4>
              <ul class="evidence-list">
                <For each={extractEvidence(node())}>
                  {(ev) => (
                    <li>
                      <code>
                        {ev.file}:{ev.line}
                      </code>
                      <span class="confidence">
                        confidence: {ev.confidence}
                      </span>
                    </li>
                  )}
                </For>
              </ul>
            </div>
          )}
        </Show>
      </section>
    </aside>
  );
};

interface EvidenceRef {
  file: string;
  line: number | string;
  confidence: number | string;
}

function getMetaString(node: GraphNode, key: string): string | undefined {
  const meta = node.meta ?? {};
  const v = meta[key];
  return typeof v === "string" && v.length > 0 ? v : undefined;
}

function extractEvidence(node: GraphNode): EvidenceRef[] {
  const meta = node.meta ?? {};
  const refs: EvidenceRef[] = [];
  if (Array.isArray(meta.evidence_refs)) {
    for (const ref of meta.evidence_refs) {
      if (typeof ref === "object" && ref !== null) {
        const r = ref as Record<string, unknown>;
        refs.push({
          file: String(r.file ?? node.file ?? "?"),
          line: String(r.line ?? "?"),
          confidence: String(r.confidence ?? "?"),
        });
      }
    }
  }
  if (refs.length === 0 && node.file) {
    refs.push({
      file: node.file,
      line: node.line ?? "?",
      confidence: "?",
    });
  }
  return refs;
}
