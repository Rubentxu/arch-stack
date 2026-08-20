/**
 * Sidebar — shows evidence for the selected node + bundle metadata.
 * Evidence pointers come from the `meta` of the GraphNode (extracted
 * from archctl bundles). When a node has a `file:line` evidence, the
 * sidebar also renders a SourceDrawer (H1, ADR-041 §5–§6).
 */

import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  type Component,
} from "solid-js";
import type { GraphNode } from "../bundle/loader";
import type { RendererEdge } from "../types";
import { zoomTargetFor } from "../lib/navigation";
import type { ExplainResult } from "../lib/workspace";
import { SourceDrawer, type SourceDrawerProps } from "./SourceDrawer";
import { TabBar, TabPanel, VirtualList } from "./primitives";

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
    /** True when the bundle was exported with `--profile strict`. */
    strict?: boolean;
  } | null;
  stats?: SidebarStats;
  /** Wired by App via `useWorkspaceState()` — fetch + open-editor handlers. */
  onFetchSource?: SourceDrawerProps["fetchSource"];
  onOpenInEditor?: SourceDrawerProps["openInEditor"];
  /** ADR-062: zoom handler — App resolves the NavigationTarget. */
  onZoom?: (dir: "in" | "out") => void;
  /** ADR-062: explain action — undefined for strict bundles (no store). */
  onExplain?: (id: string) => Promise<ExplainResult>;
  /** Bundle edges for the relations section (filtered by selected node). */
  edges?: readonly RendererEdge[];
}

/** One relation row for the selected node. */
interface RelationRow {
  dir: "in" | "out";
  other: string;
  label?: string;
}

function relationsFor(
  node: GraphNode,
  edges: readonly RendererEdge[],
): RelationRow[] {
  return edges
    .filter((e) => e.source === node.id || e.target === node.id)
    .map((e) => ({
      dir: e.source === node.id ? ("out" as const) : ("in" as const),
      other: e.source === node.id ? e.target : e.source,
      label: e.label ?? e.kind,
    }));
}

/** Defensive file:line label for explain evidence entries. */
function explainEvidenceLine(ev: Record<string, unknown>): string {
  const file = ev.file ?? ev.path ?? "?";
  const line = ev.line ?? ev.start_line ?? "?";
  return `${String(file)}:${String(line)}`;
}

export const Sidebar: Component<SidebarProps> = (props) => {
  const [copied, setCopied] = createSignal(false);
  const [explainState, setExplainState] = createSignal<
    "idle" | "loading" | "error"
  >("idle");
  const [explainData, setExplainData] = createSignal<ExplainResult | null>(
    null,
  );
  const [explainError, setExplainError] = createSignal<string | null>(null);
  const [activeTab, setActiveTab] = createSignal<"evidence" | "relations">(
    "evidence",
  );

  // Reset per-node action state when the selection changes.
  createEffect(() => {
    const selectedId = props.node?.id;
    void selectedId; // reactive dependency
    setCopied(false);
    setExplainState("idle");
    setExplainData(null);
    setExplainError(null);
    setActiveTab("evidence");
  });

  const copyId = () => {
    const n = props.node;
    if (!n) return;
    const write = navigator.clipboard?.writeText;
    if (write) {
      void write(n.id).then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      });
    }
  };

  const runExplain = async () => {
    const n = props.node;
    if (!n || !props.onExplain) return;
    setExplainState("loading");
    setExplainError(null);
    try {
      setExplainData(await props.onExplain(n.id));
      setExplainState("idle");
    } catch (e) {
      setExplainError(e instanceof Error ? e.message : String(e));
      setExplainState("error");
    }
  };

  // Tab items reactive to node + evidence/relations counts.
  const tabItems = createMemo(() => {
    if (!props.node) {
      return [
        { id: "evidence" as const, label: "Evidence", badge: undefined },
        { id: "relations" as const, label: "Relations", badge: undefined },
      ];
    }
    const evList = extractEvidence(props.node);
    const relList = props.edges ? relationsFor(props.node, props.edges) : [];
    const evBadge =
      Array.isArray(evList) && evList.length > 0 ? evList.length : undefined;
    const relBadge =
      Array.isArray(relList) && relList.length > 0 ? relList.length : undefined;
    return [
      { id: "evidence" as const, label: "Evidence", badge: evBadge },
      { id: "relations" as const, label: "Relations", badge: relBadge },
    ];
  });

  return (
    <aside class="sidebar">
      <header class="sidebar-header">
        <h2>Bundle</h2>
        <Show when={props.bundleMeta}>
          {(meta) => (
            <dl class="bundle-meta">
              <Show when={meta().strict === true}>
                <div class="strict-badge" role="status">
                  read-only · strict bundle
                </div>
              </Show>
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

              <div class="node-actions">
                <h4>Actions</h4>
                <div class="actions-row">
                  <button type="button" onClick={copyId}>
                    {copied() ? "copied ✓" : "copy id"}
                  </button>
                  <Show when={zoomTargetFor(node(), "in") !== null}>
                    <button type="button" onClick={() => props.onZoom?.("in")}>
                      zoom in
                    </button>
                  </Show>
                  <Show when={zoomTargetFor(node(), "out") !== null}>
                    <button type="button" onClick={() => props.onZoom?.("out")}>
                      zoom out
                    </button>
                  </Show>
                  <Show when={props.onExplain}>
                    <button type="button" onClick={() => void runExplain()}>
                      {explainState() === "loading" ? "explaining…" : "explain"}
                    </button>
                  </Show>
                </div>
                <Show when={explainError()}>
                  <p class="action-error">{explainError()}</p>
                </Show>
                <Show when={explainData()}>
                  {(data) => (
                    <div class="explain-result">
                      <p class="statement">{data().subject.statement}</p>
                      <Show when={data().provenance.unsubstantiated}>
                        <p class="muted">unsubstantiated — no evidence found</p>
                      </Show>
                      <ul class="evidence-list">
                        <For each={data().provenance.evidence}>
                          {(ev) => (
                            <li>
                              <code>{explainEvidenceLine(ev)}</code>
                              <Show when={ev.claim}>
                                <span class="muted"> {String(ev.claim)}</span>
                              </Show>
                            </li>
                          )}
                        </For>
                      </ul>
                      <For each={data().warnings}>
                        {(w) => <p class="muted">⚠ {w}</p>}
                      </For>
                    </div>
                  )}
                </Show>
              </div>

              <TabBar
                items={tabItems()}
                value={activeTab()}
                onChange={setActiveTab}
                ariaLabel="Sidebar panels"
              />
              <TabPanel value="evidence" activeValue={activeTab()}>
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
                <Show
                  when={
                    props.onFetchSource &&
                    props.onOpenInEditor &&
                    typeof evLine(node(), evFile(node())) === "number"
                  }
                >
                  <SourceDrawer
                    file={evFile(node()) as string}
                    line={evLine(node(), evFile(node())) as number}
                    fetchSource={props.onFetchSource!}
                    openInEditor={props.onOpenInEditor!}
                  />
                </Show>
              </TabPanel>
              <TabPanel value="relations" activeValue={activeTab()}>
                <Show
                  when={(props.edges ?? []).length > 0}
                  fallback={<p class="muted">no relations for this node</p>}
                >
                  <VirtualList
                    items={relationsFor(node(), props.edges ?? [])}
                    itemHeight={28}
                    height={220}
                    overscan={4}
                    ariaLabel="Node relations"
                    class="relations-list"
                    itemKey={(rel, i) => `${rel.dir}-${rel.other}-${i}`}
                    renderItem={(rel) => (
                      <div class={`rel ${rel.dir}`}>
                        <span class="dir">{rel.dir === "in" ? "←" : "→"}</span>
                        <code>{rel.other}</code>
                        <Show when={rel.label}>
                          <span class="muted"> {rel.label}</span>
                        </Show>
                      </div>
                    )}
                  />
                </Show>
              </TabPanel>
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

/** Pick the first evidence file (or node.file fallback) for the drawer. */
function evFile(node: GraphNode): string | null {
  const meta = node.meta ?? {};
  if (Array.isArray(meta.evidence_refs)) {
    for (const ref of meta.evidence_refs) {
      if (typeof ref === "object" && ref !== null) {
        const r = ref as Record<string, unknown>;
        if (typeof r.file === "string" && r.file.length > 0) return r.file;
      }
    }
  }
  return node.file ?? null;
}

/** Resolve the line for `file`. Returns a number only if it's parseable. */
function evLine(node: GraphNode, file: string | null): number | null {
  if (file === null) return null;
  const meta = node.meta ?? {};
  if (Array.isArray(meta.evidence_refs)) {
    for (const ref of meta.evidence_refs) {
      if (typeof ref === "object" && ref !== null) {
        const r = ref as Record<string, unknown>;
        if (r.file === file && typeof r.line === "number") return r.line;
        if (r.file === file && typeof r.line === "string") {
          const n = Number(r.line);
          return Number.isFinite(n) ? n : null;
        }
      }
    }
  }
  if (node.file === file && typeof node.line === "number") return node.line;
  return null;
}
