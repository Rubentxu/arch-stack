/**
 * PackageView — renders a call-graph bundle as a package/module
 * dependency diagram.
 *
 * Derives packages from the `file` field of nodes by grouping
 * nodes whose file shares a common directory prefix. A package
 * is the first path segment (e.g. "src/auth.rs" → "src").
 *
 * M17.5 MVP: card grid + relations list + cycle detection. The view
 * derives package-level edges from the underlying call-graph by
 * counting inter-package call edges (with weight = number of calls).
 * Cycles are highlighted in the relations panel.
 *
 * Pure helpers (packageForFile / buildPackageEdges / detectCycles)
 * live in `./PackageGraph.ts` for testability without JSX imports.
 */

import { For, Show, createMemo, type Component } from "solid-js";
import type { GraphEdge, GraphNode } from "../bundle/loader";
import {
  buildPackageEdges,
  detectCycles,
  packageForFile,
} from "./PackageGraph";

export interface PackageViewProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onSelect: (id: string | null) => void;
}

interface Package {
  name: string;
  fileCount: number;
  functionCount: number;
}

export const PackageView: Component<PackageViewProps> = (props) => {
  /** Aggregate nodes by package (file directory). */
  const packages = createMemo<Package[]>(() => {
    const map = new Map<string, Package>();
    for (const node of props.nodes) {
      const pkg = packageForFile(node.file);
      const existing = map.get(pkg);
      if (existing) {
        existing.functionCount++;
      } else {
        map.set(pkg, { name: pkg, fileCount: 1, functionCount: 1 });
      }
    }
    return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
  });

  /**
   * Build package-level edges from call-graph edges. A package edge
   * (A → B) exists if any function in A calls any function in B.
   */
  const packageEdges = createMemo(() =>
    buildPackageEdges(props.nodes, props.edges),
  );

  /** Detect cycles via DFS. Returns edge keys that are part of a cycle. */
  const cycleEdges = createMemo(() => detectCycles(packageEdges()));

  return (
    <div class="package-view">
      <header class="pkg-header">
        <h2>Package diagram</h2>
        <p class="muted">
          {packages().length} packages · {packageEdges().length}{" "}
          inter-package edges
          {cycleEdges().size > 0 ? " · ⚠ cycle" : ""}
        </p>
      </header>

      <Show
        when={packages().length > 0}
        fallback={<p class="empty">No functions to derive packages from.</p>}
      >
        <div class="pkg-grid">
          <For each={packages()}>
            {(pkg) => (
              <article
                class="pkg-card"
                onClick={() => props.onSelect(pkg.name)}
                title={`${pkg.fileCount} file(s), ${pkg.functionCount} function(s)`}
              >
                <h3 class="pkg-name">{pkg.name}</h3>
                <dl class="pkg-stats">
                  <dt>files</dt>
                  <dd>{pkg.fileCount}</dd>
                  <dt>functions</dt>
                  <dd>{pkg.functionCount}</dd>
                </dl>
              </article>
            )}
          </For>
        </div>

        <Show when={packageEdges().length > 0}>
          <section class="pkg-relations">
            <h3>Inter-package dependencies</h3>
            <ul>
              <For each={packageEdges()}>
                {(e) => {
                  const isCycle = cycleEdges().has(`${e.source}\0${e.target}`);
                  return (
                    <li
                      class={`pkg-edge ${
                        isCycle ? "is-cycle" : ""
                      } weight-${Math.min(3, e.weight)}`}
                    >
                      <code>{e.source}</code>
                      <span class="pkg-arrow">
                        ─{isCycle ? "↺" : "→"}({e.weight})→
                      </span>
                      <code>{e.target}</code>
                    </li>
                  );
                }}
              </For>
            </ul>
          </section>
        </Show>
      </Show>
    </div>
  );
};
